import { language, missingKeys, other, setLanguage, t, translateDocument } from "./i18n.js";

const invoke = window.__TAURI__.core.invoke;

const canvas = document.querySelector(".canvas");
const addCard = document.querySelector(".add-card");
const statusbar = document.querySelector(".statusbar");
const cardTemplate = document.querySelector("#card-template");

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

/* ------------------------------------------------------------------ boot */

async function boot() {
  const gaps = missingKeys();
  if (Object.keys(gaps).length) console.warn("untranslated keys", gaps);

  setLanguage(await invoke("system_language"));
  translateDocument();
  paintLanguageButton();

  monitors = await invoke("list_monitors");
  wireToolbar();
  addWindowCard();
  say("status.ready");
}

/* --------------------------------------------------------------- toolbar */

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

  for (const tool of document.querySelectorAll(".tool[data-stage]")) {
    tool.addEventListener("click", () =>
      say("status.notBuilt", { module: tool.title }, "warn")
    );
  }

  addCard.addEventListener("click", addWindowCard);

  // F8 is the emergency reset in the guide; the app honours it while focused,
  // and a global hook takes over once the bind layer lands.
  window.addEventListener("keydown", (event) => {
    if (event.key === "F8") {
      event.preventDefault();
      restoreEverything();
    }
  });
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

/* ------------------------------------------------------------------ card */

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
  paintResult(card);
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

/* --------------------------------------------------------------- sources */

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

  if (card.handle && windows.some((w) => w.handle === card.handle)) {
    select.value = String(card.handle);
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

/* ---------------------------------------------------------------- layout */

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
      request: { handle: card.handle, rect: card.rect, borderless: card.borderless },
    });
    card.rect = placed;
    syncCard(card);
    say("status.applied", placed);
  } catch (err) {
    reportFailure(err);
  }
}

/* --------------------------------------------------------------- preview */

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

window.addEventListener("resize", () => {
  for (const card of cards) drawPreview(card);
});

boot().catch(reportFailure);
