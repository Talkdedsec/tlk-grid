import { boot, invoke, report, say, t } from "./module.js";

const enabled = document.querySelector(".enabled");
const size = document.querySelector(".size");
const sizeValue = document.querySelector(".size-value");
const note = document.querySelector(".backdrop-note");

let seeThrough = true;

boot(async () => {
  paint(await invoke("scope_state"));
  wire();
  say("scope.ready");
});

function wire() {
  enabled.addEventListener("click", () => {
    enabled.setAttribute(
      "aria-checked",
      String(enabled.getAttribute("aria-checked") !== "true")
    );
    apply();
  });

  size.addEventListener("input", () => {
    sizeValue.textContent = `${size.value} px`;
  });
  size.addEventListener("change", apply);

  for (const button of document.querySelectorAll(".backdrop")) {
    button.addEventListener("click", () => {
      seeThrough = button.dataset.seeThrough === "true";
      paintBackdrop();
      apply();
    });
  }

  document.querySelector(".save").addEventListener("click", apply);
}

async function apply() {
  try {
    paint(
      await invoke("set_scope", {
        enabled: enabled.getAttribute("aria-checked") === "true",
        size: Number(size.value),
        seeThrough,
      })
    );
    say(
      enabled.getAttribute("aria-checked") === "true" ? "scope.on" : "scope.off",
      { size: size.value }
    );
  } catch (err) {
    report(err);
  }
}

function paint(state) {
  enabled.setAttribute("aria-checked", String(state.enabled));
  size.value = String(state.size);
  sizeValue.textContent = `${state.size} px`;
  seeThrough = state.see_through;
  paintBackdrop();
}

function paintBackdrop() {
  for (const button of document.querySelectorAll(".backdrop")) {
    button.setAttribute(
      "aria-pressed",
      String((button.dataset.seeThrough === "true") === seeThrough)
    );
  }
  note.textContent = t(seeThrough ? "scope.noteSeeThrough" : "scope.noteBlack");
}
