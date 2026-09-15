import { boot, invoke, report, say, t } from "./module.js";

const preview = document.querySelector(".crosshair-preview");
const file = document.querySelector(".file");
const scale = document.querySelector(".scale");
const scaleValue = document.querySelector(".scale-value");
const visible = document.querySelector(".visible");

let state = { loaded: false, visible: false, scale_percent: 100, width: 0, height: 0 };

boot(async () => {
  paint(await invoke("crosshair_state"));
  wire();
  say(state.loaded ? "crosshair.ready" : "crosshair.none");
});

function wire() {
  document.querySelector(".pick").addEventListener("click", () => file.click());
  file.addEventListener("change", load);

  scale.addEventListener("input", () => {
    scaleValue.textContent = `${scale.value}%`;
    drawPreview();
  });
  // The slider fires on every pixel of drag; only the settled value is worth a
  // round trip and a repaint of the real overlay.
  scale.addEventListener("change", async () => {
    try {
      paint(await invoke("set_crosshair_scale", { percent: Number(scale.value) }));
      say("crosshair.scaled", { percent: state.scale_percent });
    } catch (err) {
      report(err);
    }
  });

  visible.addEventListener("click", async () => {
    if (!state.loaded) return say("crosshair.none", null, "warn");
    try {
      paint(await invoke("toggle_crosshair", { visible: !state.visible }));
      say(state.visible ? "crosshair.shown" : "crosshair.hidden");
    } catch (err) {
      report(err);
    }
  });

  document.querySelector(".remove").addEventListener("click", async () => {
    paint(await invoke("clear_crosshair"));
    preview.replaceChildren();
    say("crosshair.removed");
  });
}

async function load() {
  const chosen = file.files?.[0];
  if (!chosen) return;
  try {
    const bytes = Array.from(new Uint8Array(await chosen.arrayBuffer()));
    paint(await invoke("set_crosshair_image", { bytes }));
    say("crosshair.loaded", { name: chosen.name });
  } catch (err) {
    report(err);
  } finally {
    // Clearing the input lets the same file be picked again after a Remove.
    file.value = "";
  }
}

function paint(next) {
  state = next;
  scale.value = String(state.scale_percent);
  scale.disabled = !state.loaded;
  scaleValue.textContent = `${state.scale_percent}%`;
  visible.setAttribute("aria-checked", String(state.visible));
  drawPreview();
}

/// A to-scale mock of the middle of a 1920×1080 screen, so the size slider can
/// be judged without alt-tabbing into a game.
function drawPreview() {
  if (!state.loaded || !state.width) {
    preview.replaceChildren();
    return;
  }
  const percent = Number(scale.value) || state.scale_percent;
  const shown = (state.width * percent) / 100 / 1920;

  const mark = document.createElement("div");
  mark.className = "box";
  mark.style.left = `${(0.5 - shown / 2) * 100}%`;
  mark.style.width = `${shown * 100}%`;
  mark.style.aspectRatio = `${state.width} / ${state.height}`;
  mark.style.top = "50%";
  mark.style.transform = "translateY(-50%)";

  const caption = document.createElement("div");
  caption.className = "center-dot";
  caption.style.left = "50%";
  caption.style.top = "50%";

  preview.replaceChildren(mark, caption);
  preview.title = t("crosshair.previewTitle", {
    w: Math.round((state.width * percent) / 100),
    h: Math.round((state.height * percent) / 100),
  });
}
