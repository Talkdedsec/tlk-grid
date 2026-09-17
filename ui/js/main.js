import { language, missingKeys, other, setLanguage, t, translateDocument } from "./i18n.js";

const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

/** Actions a window card can bind. Scope and crosshair get their own windows. */
const CARD_ACTIONS = ["resize", "black-bars"];
const ACTION_LABEL = { resize: "binds.resize", "black-bars": "binds.blackBars" };

/** Toolbar buttons that reveal a settings window of the same label. */
const MODULE_WINDOWS = ["crosshair", "scope"];
/** Only Resize carries a zoom factor; Black Bars is a plain on/off overlay. */
const FACTOR_ACTIONS = new Set(["resize"]);

const canvas = document.querySelector(".canvas");
const addCard = document.querySelector(".add-card");
const statusbar = document.querySelector(".statusbar");
const cardTemplate = document.querySelector("#card-template");
const bindTemplate = document.querySelector("#bind-template");
const panel = document.querySelector(".profiles");
const profileName = document.querySelector(".profile-name");
const profileList = document.querySelector(".profile-list");

/** Which bind field is armed, if any: { card, action }. */
let capturing = null;
let masterOn = true;
let wheelAdjusts = true;

/** Monitors are shared by every card; the window list is re-read on demand. */
let monitors = [];
const cards = [];
let nextCardIndex = 1;

function say(key, vars, tone = "info") {
  statusbar.textContent = t(key, vars);
  statusbar.dataset.tone = tone;
}

function reportFailure(err) {
  const message = typeof err === "string" ? err : (err?.message ?? String(err));
  if (message.includes("is gone")) {
    say("status.gone", null, "warn");
  } else {
    say("status.error", { message }, "warn");
  }
}

/* boot */

async function boot() {
  const gaps = missingKeys();
  if (Object.keys(gaps).length) console.warn("untranslated keys", gaps);

  setLanguage(await invoke("system_language"));
  translateDocument();
  paintLanguageButton();

  monitors = await invoke("list_monitors");
  await listen("input", (message) => onInput(message.payload));
  await listen("factor", (message) => {
    const card = cards.find((c) => c.handle === bindTargetHandle) ?? cards[0];
    if (!card) return;
    card.factor = String(message.payload).replace(".", ",");
    const field = card.el.querySelector(".bind-factor");
    if (field) field.value = card.factor;
    paintZoomResult(card);
  });
  await listen("restored", async (message) => {
    for (const card of cards) await readBackRect(card);
    say(message.payload > 0 ? "status.restoredAll" : "status.nothingToRestore", {
      n: message.payload,
    });
  });
  wireToolbar();

  const previous = await invoke("restore_session").catch(() => null);
  if (previous && previous.profile.windows.length) {
    await applyProfile(previous);
    say("profiles.restored");
  } else {
    addWindowCard();
    say("status.ready");
  }
  refreshProfileList();
}

/* toolbar */

function paintLanguageButton() {
  document.querySelector('[data-tool="lang"]').textContent = language().toUpperCase();
}

function wireToolbar() {
  document.querySelector('[data-tool="lang"]').addEventListener("click", () => {
    setLanguage(other());
    translateDocument();
    paintLanguageButton();
    for (const card of cards) refreshCardLabels(card);
  });

  document.querySelector('[data-tool="panic"]').addEventListener("click", restoreEverything);

  const wheelButton = document.querySelector('[data-tool="zoom-scroll"]');
  wheelButton.removeAttribute("data-stage");
  wheelButton.setAttribute("aria-pressed", String(wheelAdjusts));
  wheelButton.addEventListener("click", async () => {
    wheelAdjusts = !wheelAdjusts;
    await invoke("set_wheel_adjusts", { enabled: wheelAdjusts });
    wheelButton.setAttribute("aria-pressed", String(wheelAdjusts));
    say(wheelAdjusts ? "status.wheelOn" : "status.wheelOff");
  });

  const masterButton = document.querySelector('[data-tool="master"]');
  masterButton.removeAttribute("data-stage");
  masterButton.setAttribute("aria-pressed", String(masterOn));
  masterButton.addEventListener("click", () => setMaster(!masterOn));

  for (const tool of ["load", "save"]) {
    const button = document.querySelector(`[data-tool="${tool}"]`);
    button.addEventListener("click", () => {
      panel.hidden = !panel.hidden;
      if (!panel.hidden) {
        refreshProfileList();
        if (tool === "save") profileName.focus();
      }
    });
  }
  document.querySelector(".profile-save").addEventListener("click", saveProfile);
  profileName.addEventListener("keydown", (event) => {
    if (event.key === "Enter") saveProfile();
  });

  for (const tool of MODULE_WINDOWS) {
    const button = document.querySelector(`[data-tool="${tool}"]`);
    button.removeAttribute("data-stage");
    button.addEventListener("click", () => openModule(tool));
  }

  for (const tool of document.querySelectorAll(".tool[data-stage]")) {
    tool.addEventListener("click", () =>
      say("status.notBuilt", { module: tool.title }, "warn")
    );
  }

  // Esc backs out of an armed bind field without assigning anything.
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && capturing) {
      event.preventDefault();
      invoke("cancel_bind_capture");
      endCapture();
    }
  });

  addCard.addEventListener("click", addWindowCard);
  // F8 is handled by the input hook, so it works with the app in the
  // background too. Nothing to bind here.
}

/** The settings windows are declared in the config and start hidden, so the
 * toolbar only has to reveal one. Creating them on demand would put window
 * creation on the click path for no gain. */
async function openModule(label) {
  try {
    const { WebviewWindow } = window.__TAURI__.webviewWindow;
    const win = await WebviewWindow.getByLabel(label);
    if (!win) return say("status.notBuilt", { module: t(`tool.${label}`) }, "warn");
    await win.show();
    await win.unminimize();
    await win.setFocus();
  } catch (err) {
    reportFailure(err);
  }
}

async function setMaster(on) {
  masterOn = on;
  await invoke("set_master", { enabled: on });
  document.querySelector('[data-tool="master"]').setAttribute("aria-pressed", String(on));
  say(on ? "status.masterOn" : "status.masterOff", null, on ? "info" : "warn");
}

async function restoreEverything() {
  try {
    const report = await invoke("restore_everything");
    if (report.restored === 0 && report.failed === 0) {
      say("status.nothingToRestore");
    } else {
      say("status.restoredAll", { n: report.restored });
    }
    for (const card of cards) await readBackRect(card);
  } catch (err) {
    reportFailure(err);
  }
}

/* card */

function addWindowCard() {
  const el = cardTemplate.content.firstElementChild.cloneNode(true);
  const card = {
    index: nextCardIndex++,
    el,
    handle: null,
    monitor: monitors[0] ?? null,
    rect: null,
    divisions: 0,
    borderless: false,
    pin: false,
    target: null,
    method: "thumbnail",
    factor: "1,5",
    binds: Object.fromEntries(CARD_ACTIONS.map((action) => [action, null])),
  };
  cards.push(card);

  translateDocument(el);
  refreshCardLabels(card);
  wireCard(card);
  canvas.insertBefore(el, addCard);
  fillMonitors(card);
  refreshTargets(card);
  return card;
}

function refreshCardLabels(card) {
  card.el.querySelector(".card-index").textContent = t("card.index", { n: card.index });
  card.el.querySelector(".card-mode").textContent = t("card.mode.window");
  for (const action of CARD_ACTIONS) paintBind(card, action);
  if (card.el.querySelector("[data-method]")) paintMethod(card);
  paintResult(card);
  paintZoomResult(card);
}

function q(card, selector) {
  return card.el.querySelector(selector);
}

function wireCard(card) {
  q(card, ".card-close").addEventListener("click", () => closeCard(card));
  q(card, ".target-refresh").addEventListener("click", () => refreshTargets(card));

  q(card, ".target-select").addEventListener("change", (event) => {
    const handle = Number(event.target.value);
    card.handle = Number.isFinite(handle) && handle !== 0 ? handle : null;
    card.target = (card.available ?? []).find((w) => w.handle === card.handle) ?? null;
    onTargetPicked(card);
  });

  q(card, ".monitor-select").addEventListener("change", (event) => {
    card.monitor = monitors.find((m) => m.device === event.target.value) ?? card.monitor;
    paintMonitorSize(card);
    drawPreview(card);
  });

  for (const button of card.el.querySelectorAll("[data-anchor]")) {
    button.addEventListener("click", async () => {
      if (!card.monitor) return;
      const size = card.rect ? [card.rect.w, card.rect.h] : null;
      card.rect = await invoke("anchor_rect", {
        work: card.monitor.work,
        anchor: button.dataset.anchor,
        size: button.dataset.anchor === "center" ? size : null,
      });
      card.divisions = 0;
      syncCard(card);
    });
  }

  for (const chip of card.el.querySelectorAll("[data-divisions]")) {
    chip.addEventListener("click", () => selectDivisions(card, Number(chip.dataset.divisions)));
  }

  for (const input of card.el.querySelectorAll(".geom input")) {
    input.addEventListener("change", () => readGeomInputs(card));
  }

  q(card, ".borderless").addEventListener("change", (event) => {
    card.borderless = event.target.checked;
  });

  q(card, ".pin").addEventListener("change", (event) => {
    card.pin = event.target.checked;
    say(card.pin ? "status.pinned" : "status.unpinned");
    if (card.handle && card.rect) applyCard(card);
  });

  buildBinds(card);

  q(card, ".apply").addEventListener("click", () => applyCard(card));

  wirePreview(card);
}

async function closeCard(card) {
  if (card.handle) {
    try {
      await invoke("release_window", { handle: card.handle });
      say("status.released");
    } catch (err) {
      reportFailure(err);
    }
  }
  card.el.remove();
  cards.splice(cards.indexOf(card), 1);
}

/* sources */

function fillMonitors(card) {
  const select = q(card, ".monitor-select");
  select.replaceChildren(
    ...monitors.map((monitor) => {
      const option = document.createElement("option");
      option.value = monitor.device;
      option.textContent = `${monitor.name} ${monitor.bounds.w}×${monitor.bounds.h}${
        monitor.refresh_hz ? ` @ ${monitor.refresh_hz} Hz` : ""
      }`;
      return option;
    })
  );
  if (card.monitor) select.value = card.monitor.device;
  paintMonitorSize(card);
  drawPreview(card);
}

function paintMonitorSize(card) {
  q(card, ".monitor-w").value = card.monitor?.bounds.w ?? "";
  q(card, ".monitor-h").value = card.monitor?.bounds.h ?? "";
}

async function refreshTargets(card) {
  let windows;
  try {
    windows = await invoke("list_windows");
  } catch (err) {
    reportFailure(err);
    return;
  }

  const select = q(card, ".target-select");
  const placeholder = document.createElement("option");
  placeholder.value = "0";
  placeholder.textContent = t("target.placeholder");
  select.replaceChildren(
    placeholder,
    ...windows.map((w) => {
      const option = document.createElement("option");
      option.value = String(w.handle);
      option.textContent = label(w);
      return option;
    })
  );

  card.available = windows;
  if (card.handle && windows.some((w) => w.handle === card.handle)) {
    select.value = String(card.handle);
    card.target = windows.find((w) => w.handle === card.handle) ?? card.target;
  } else if (card.handle) {
    card.handle = null;
    say("status.gone", null, "warn");
    return;
  }

  say(windows.length ? "status.listed" : "status.noWindows", { n: windows.length },
    windows.length ? "info" : "warn");
}

function label(w) {
  const base = w.process ? `${w.title} (${w.process})` : w.title;
  return w.occurrence ? `${base} #${w.occurrence}` : base;
}

async function onTargetPicked(card) {
  if (!card.handle) return;
  try {
    card.monitor = await invoke("monitor_for_window", { handle: card.handle });
    fillMonitors(card);
    await readBackRect(card);
    await pushBinds(card);
    await pushZoom(card);
    if (card.rect) {
      say("status.selected", {
        name: q(card, ".target-select").selectedOptions[0].textContent,
        ...card.rect,
      });
    }
  } catch (err) {
    reportFailure(err);
  }
}

async function readBackRect(card) {
  if (!card.handle) return;
  try {
    card.rect = await invoke("window_rect", { handle: card.handle });
    syncCard(card);
  } catch (err) {
    reportFailure(err);
  }
}

/* layout */

async function selectDivisions(card, divisions) {
  card.divisions = card.divisions === divisions ? 0 : divisions;
  for (const chip of card.el.querySelectorAll("[data-divisions]")) {
    chip.setAttribute("aria-pressed", String(Number(chip.dataset.divisions) === card.divisions));
  }
  drawPreview(card);
}

function readGeomInputs(card) {
  const value = (selector) => Number(q(card, selector).value) || 0;
  card.rect = { x: value(".geom-x"), y: value(".geom-y"), w: value(".geom-w"), h: value(".geom-h") };
  syncCard(card);
}

function syncCard(card) {
  paintGeomInputs(card);
  drawPreview(card);
  paintResult(card);
  q(card, ".apply").disabled = !(card.handle && card.rect);
}

function paintGeomInputs(card) {
  q(card, ".geom-x").value = card.rect?.x ?? "";
  q(card, ".geom-y").value = card.rect?.y ?? "";
  q(card, ".geom-w").value = card.rect?.w ?? "";
  q(card, ".geom-h").value = card.rect?.h ?? "";
}

function paintResult(card) {
  const node = q(card, ".result");
  if (!card.rect) {
    node.hidden = true;
    return;
  }
  node.hidden = false;
  node.textContent = `${t("result.label")}: ${card.rect.w} × ${card.rect.h}`;
}

async function applyCard(card) {
  if (!card.handle || !card.rect) return;
  try {
    const placed = await invoke("apply_placement", {
      request: {
        handle: card.handle,
        rect: card.rect,
        borderless: card.borderless,
        pin: card.pin,
      },
    });
    card.rect = placed;
    syncCard(card);
    await pushZoom(card);
    rememberSession();
    say("status.applied", placed);
  } catch (err) {
    reportFailure(err);
  }
}

/* preview */

/** Monitor pixels per preview pixel; the preview is always drawn to scale. */
function previewScale(card) {
  const preview = q(card, ".preview");
  const bounds = card.monitor?.bounds;
  if (!bounds || !bounds.w) return null;
  const width = preview.clientWidth;
  return { preview, bounds, k: width / bounds.w };
}

async function drawPreview(card) {
  const view = previewScale(card);
  if (!view) return;
  const { preview, bounds, k } = view;
  preview.style.aspectRatio = `${bounds.w} / ${bounds.h}`;

  const parts = [];

  if (card.divisions >= 4) {
    const cells = await invoke("split_cells", { work: bounds, divisions: card.divisions });
    for (const cell of cells) {
      const node = document.createElement("div");
      node.className = "cell";
      node.dataset.index = String(cell.index);
      Object.assign(node.style, boxStyle(cell.rect, bounds, k));
      node.innerHTML = `<b>${cell.index}</b><span>${cell.rect.x},${cell.rect.y} · ${cell.rect.w}×${cell.rect.h}</span>`;
      node.style.pointerEvents = "auto";
      node.style.cursor = "pointer";
      node.addEventListener("click", () => {
        card.rect = cell.rect;
        syncCard(card);
      });
      parts.push(node);
    }
  }

  if (card.rect) {
    const box = document.createElement("div");
    box.className = "box";
    Object.assign(box.style, boxStyle(card.rect, bounds, k));
    parts.push(box);
  }

  const dot = document.createElement("div");
  dot.className = "center-dot";
  dot.style.left = "50%";
  dot.style.top = "50%";
  parts.push(dot);

  preview.replaceChildren(...parts);
  markHotCell(card);
}

function boxStyle(rect, bounds, k) {
  return {
    left: `${(rect.x - bounds.x) * k}px`,
    top: `${(rect.y - bounds.y) * k}px`,
    width: `${rect.w * k}px`,
    height: `${rect.h * k}px`,
  };
}

function markHotCell(card) {
  if (!card.rect) return;
  for (const node of card.el.querySelectorAll(".cell")) {
    const box = q(card, ".box");
    if (!box) return;
    const overlaps =
      Math.abs(node.offsetLeft - box.offsetLeft) < 2 &&
      Math.abs(node.offsetTop - box.offsetTop) < 2 &&
      Math.abs(node.offsetWidth - box.offsetWidth) < 2;
    node.classList.toggle("hot", overlaps);
  }
}

/** Drag anywhere on the preview to draw the window rect in monitor pixels. */
function wirePreview(card) {
  const preview = q(card, ".preview");
  let origin = null;

  preview.addEventListener("pointerdown", (event) => {
    const view = previewScale(card);
    if (!view) return;
    preview.setPointerCapture(event.pointerId);
    const local = localPoint(preview, event);
    origin = { x: local.x / view.k + view.bounds.x, y: local.y / view.k + view.bounds.y };
  });

  preview.addEventListener("pointermove", (event) => {
    if (!origin) return;
    const view = previewScale(card);
    const local = localPoint(preview, event);
    const now = { x: local.x / view.k + view.bounds.x, y: local.y / view.k + view.bounds.y };
    card.rect = {
      x: Math.round(Math.min(origin.x, now.x)),
      y: Math.round(Math.min(origin.y, now.y)),
      w: Math.round(Math.abs(now.x - origin.x)),
      h: Math.round(Math.abs(now.y - origin.y)),
    };
    paintGeomInputs(card);
    drawPreview(card);
  });

  const finish = () => {
    if (!origin) return;
    origin = null;
    // A click with no drag is a cell pick, not a 0×0 window.
    if (card.rect && (card.rect.w < 8 || card.rect.h < 8)) return readBackRect(card);
    syncCard(card);
  };

  preview.addEventListener("pointerup", finish);
  preview.addEventListener("pointercancel", finish);
}

function localPoint(element, event) {
  const box = element.getBoundingClientRect();
  return { x: event.clientX - box.left, y: event.clientY - box.top };
}

/* binds */

function buildBinds(card) {
  const host = q(card, ".bind-rows");
  for (const action of CARD_ACTIONS) {
    const row = bindTemplate.content.firstElementChild.cloneNode(true);
    row.dataset.action = action;
    translateDocument(row);
    row.querySelector(".bind-key").addEventListener("click", () => beginCapture(card, action));
    row.querySelector(".bind-clear").addEventListener("click", () => clearBind(card, action));
    row.querySelector(".bind-mode").addEventListener("click", () => cycleMode(card, action));

    const factor = row.querySelector(".bind-factor");
    if (FACTOR_ACTIONS.has(action)) {
      factor.value = card.factor;
      factor.addEventListener("change", () => {
        card.factor = factor.value;
        pushZoom(card);
      });
    } else {
      factor.hidden = true;
    }

    host.append(row);
    paintBind(card, action);
  }

  for (const chip of card.el.querySelectorAll("[data-method]")) {
    chip.addEventListener("click", () => selectMethod(card, chip.dataset.method));
  }
  for (const chip of card.el.querySelectorAll(".mult")) {
    chip.addEventListener("click", () => {
      card.factor = String(Number(chip.dataset.mult));
      q(card, ".bind-factor").value = card.factor;
      pushZoom(card);
    });
  }
  paintMethod(card);
}

function selectMethod(card, method) {
  card.method = method;
  paintMethod(card);
  pushZoom(card);
}

function paintMethod(card) {
  for (const chip of card.el.querySelectorAll("[data-method]")) {
    chip.setAttribute("aria-pressed", String(chip.dataset.method === card.method));
  }
  q(card, ".method-note").textContent = t(`zoom.note.${card.method}`);
}

/** Hands the backend what the Resize bind should do for this window. */
async function pushZoom(card) {
  if (!card.handle) return;
  try {
    const factor = await invoke("set_zoom", {
      handle: card.handle,
      factor: card.factor,
      method: card.method,
      borderless: card.borderless,
    });
    card.factor = String(factor).replace(".", ",");
    q(card, ".bind-factor").value = card.factor;
    paintZoomResult(card);
    say("status.factorSet", { factor: card.factor });
  } catch (err) {
    reportFailure(err);
  }
}

/** `Result: 5760 × 1620 (base 1920×540)` under the multiplier buttons. */
async function paintZoomResult(card) {
  const node = q(card, ".zoom-result");
  const factor = Number(String(card.factor).replace(",", "."));
  if (!card.rect || !Number.isFinite(factor)) {
    node.hidden = true;
    return;
  }
  const dest = await invoke("zoom_destination", { base: card.rect, factor });
  node.hidden = false;
  node.textContent =
    `${t("result.label")}: ${dest.w} × ${dest.h} ` +
    `(${t("result.base")} ${card.rect.w}×${card.rect.h})`;
}

function bindRow(card, action) {
  return card.el.querySelector('.bind[data-action="' + action + '"]');
}

function paintBind(card, action) {
  const row = bindRow(card, action);
  if (!row) return;
  const bind = card.binds[action];
  row.querySelector(".bind-action").textContent = t(ACTION_LABEL[action]);

  const key = row.querySelector(".bind-key");
  const armed = Boolean(capturing && capturing.card === card && capturing.action === action);
  key.dataset.capturing = String(armed);
  key.dataset.empty = String(!bind && !armed);
  key.textContent = armed ? t("binds.capturing") : (bind ? bind.label : t("binds.setKey"));

  row.querySelector(".bind-mode").textContent = t(
    bind && bind.mode === "toggle" ? "binds.toggle" : "binds.hold"
  );
}

async function beginCapture(card, action) {
  const previous = capturing;
  if (previous) await invoke("cancel_bind_capture");
  capturing = { card, action };
  if (previous) paintBind(previous.card, previous.action);
  paintBind(card, action);
  say("status.capturing");
  await invoke("begin_bind_capture");
}

function endCapture() {
  if (!capturing) return;
  const { card, action } = capturing;
  capturing = null;
  paintBind(card, action);
}

async function clearBind(card, action) {
  card.binds[action] = null;
  paintBind(card, action);
  await pushBinds(card);
  say("status.bindCleared", { action: t(ACTION_LABEL[action]) });
}

async function cycleMode(card, action) {
  const bind = card.binds[action];
  if (!bind) return;
  bind.mode = bind.mode === "toggle" ? "hold" : "toggle";
  paintBind(card, action);
  await pushBinds(card);
}

/** The hook holds one bind set at a time, so the card whose window is in front
 * owns it. Switching targets swaps the set rather than merging them. */
let bindTargetHandle = null;

async function pushBinds(card) {
  bindTargetHandle = card.handle;
  const binds = CARD_ACTIONS.filter((action) => card.binds[action]).map((action) => ({
    action,
    trigger: card.binds[action].trigger,
    mode: card.binds[action].mode,
    swallow: true,
  }));
  await invoke("set_bind_target", { handle: card.handle });
  await invoke("set_binds", { binds });
  rememberSession();
}

function cardFor(handle) {
  return cards.find((card) => card.handle === handle) ?? null;
}

function actionName(action) {
  return t(ACTION_LABEL[action] ?? action);
}

async function onInput(event) {
  switch (event.event) {
    case "captured":
      return onCaptured(event);
    case "engaged":
    case "released":
      return onBindState(event);
    case "wheel":
      say("status.wheelNotch", {
        action: actionName(event.action),
        notches: event.notches > 0 ? "+" + event.notches : String(event.notches),
      });
      return;
    case "foreground": {
      const card = cardFor(event.handle);
      if (card) await pushBinds(card);
      return;
    }
    default:
      return;
  }
}

async function onCaptured(event) {
  if (!capturing) return;
  const { card, action } = capturing;
  const label = await invoke("trigger_label", { trigger: event.trigger });
  card.binds[action] = { trigger: event.trigger, mode: "hold", label };
  endCapture();
  await pushBinds(card);
  say("status.bindSet", { action: actionName(action), key: label });
}

async function onBindState(event) {
  const engaged = event.event === "engaged";

  // F8 reports through the `restored` event, which carries the real count.
  if (event.action === "panic") return;

  if (event.action === "master") {
    // The hook already flipped the switch; only the toolbar needs telling.
    masterOn = !engaged;
    document.querySelector('[data-tool="master"]').setAttribute("aria-pressed", String(masterOn));
    say(masterOn ? "status.masterOn" : "status.masterOff", null, masterOn ? "info" : "warn");
    return;
  }

  markEngaged(event.action, engaged);
  say(engaged ? "status.engaged" : "status.released", { action: actionName(event.action) });
}

function markEngaged(action, on) {
  for (const card of cards) {
    const row = bindRow(card, action);
    if (row) row.dataset.engaged = String(on);
  }
}

/* profiles */

let sessionTimer = null;

/** Writing on every keystroke would hammer the disk; a short pause is enough. */
function rememberSession() {
  clearTimeout(sessionTimer);
  sessionTimer = setTimeout(() => {
    invoke("remember_session", { profile: collectProfile("session") }).catch(() => {});
  }, 400);
}

function collectProfile(name) {
  return {
    version: 1,
    name,
    language: language(),
    wheel_adjusts: wheelAdjusts,
    scope: { enabled: false, size: 650, see_through: true },
    crosshair_scale: null,
    windows: cards.filter((card) => card.rect).map(cardToSetup),
  };
}

function cardToSetup(card) {
  const factor = Number(String(card.factor).replace(",", ".")) || 1;
  return {
    process: card.target?.process ?? "",
    title: card.target?.title ?? "",
    occurrence: card.target?.occurrence ?? null,
    monitor: card.monitor?.device ?? "",
    rect: card.rect,
    borderless: card.borderless,
    pin: card.pin,
    method: card.method,
    factor,
    binds: CARD_ACTIONS.filter((action) => card.binds[action]).map((action) => ({
      action,
      trigger: card.binds[action].trigger,
      mode: card.binds[action].mode,
      swallow: true,
    })),
  };
}

/** Rebuilds the canvas from a profile whose cards have already been matched
 * against the windows that are open now. */
async function applyProfile(resolved) {
  for (const card of [...cards]) {
    card.el.remove();
    cards.splice(cards.indexOf(card), 1);
  }
  nextCardIndex = 1;

  const setups = resolved.profile.windows;
  for (let index = 0; index < setups.length; index += 1) {
    const setup = setups[index];
    const found = resolved.windows[index] ?? {};
    const card = addWindowCard();

    card.rect = setup.rect;
    card.borderless = setup.borderless;
    card.pin = setup.pin;
    card.method = setup.method;
    card.factor = String(setup.factor).replace(".", ",");
    card.monitor = monitors.find((m) => m.device === setup.monitor) ?? card.monitor;
    for (const bind of setup.binds ?? []) {
      card.binds[bind.action] = {
        trigger: bind.trigger,
        mode: bind.mode,
        label: await invoke("trigger_label", { trigger: bind.trigger }),
      };
    }

    q(card, ".borderless").checked = card.borderless;
    q(card, ".pin").checked = card.pin;
    q(card, ".bind-factor").value = card.factor;
    fillMonitors(card);
    refreshCardLabels(card);

    if (found.handle) {
      card.handle = found.handle;
      await refreshTargets(card);
      q(card, ".target-select").value = String(found.handle);
      await pushBinds(card);
      await pushZoom(card);
      if (found.quality === "same-process") {
        say("profiles.weakMatch", { name: found.label ?? setup.process }, "warn");
      }
    }
    syncCard(card);
  }

  if (!cards.length) addWindowCard();
}

async function refreshProfileList() {
  let names = [];
  try {
    names = await invoke("list_profiles");
  } catch (err) {
    return reportFailure(err);
  }

  profileList.dataset.empty = t("profiles.none");
  profileList.replaceChildren(
    ...names.map((name) => {
      const row = document.createElement("div");
      row.className = "saved";

      const open = document.createElement("button");
      open.type = "button";
      open.className = "open";
      open.textContent = name;
      open.addEventListener("click", () => loadProfile(name));

      const drop = document.createElement("button");
      drop.type = "button";
      drop.className = "drop";
      drop.textContent = "×";
      drop.title = t("profiles.delete");
      drop.addEventListener("click", () => deleteProfile(name));

      row.append(open, drop);
      return row;
    })
  );
}

async function loadProfile(name) {
  try {
    const resolved = await invoke("load_profile", { name });
    await applyProfile(resolved);
    const matched = resolved.windows.filter((w) => w.handle).length;
    say("profiles.loaded", { name, matched, total: resolved.windows.length });
    rememberSession();
  } catch (err) {
    reportFailure(err);
  }
}

async function saveProfile() {
  const name = profileName.value.trim();
  if (!name) return say("profiles.needsName", null, "warn");
  try {
    await invoke("save_profile", { profile: collectProfile(name) });
    profileName.value = "";
    await refreshProfileList();
    say("profiles.saved", { name });
  } catch (err) {
    reportFailure(err);
  }
}

async function deleteProfile(name) {
  try {
    await invoke("delete_profile", { name });
    await refreshProfileList();
    say("profiles.deleted", { name });
  } catch (err) {
    reportFailure(err);
  }
}

window.addEventListener("resize", () => {
  for (const card of cards) drawPreview(card);
});

boot().catch(reportFailure);
