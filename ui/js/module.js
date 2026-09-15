// Shared boot for the settings windows. They are separate top-level windows
// rather than dialogs, so they can stay open beside the game while it runs.

import { setLanguage, t, translateDocument } from "./i18n.js";

export const invoke = window.__TAURI__.core.invoke;

let statusbar = null;

export function say(key, vars, tone = "info") {
  statusbar ??= document.querySelector(".statusbar");
  statusbar.textContent = t(key, vars);
  statusbar.dataset.tone = tone;
}

export function report(err) {
  const message = typeof err === "string" ? err : (err?.message ?? String(err));
  say("status.error", { message }, "warn");
}

/// Picks up the same language the manager is in, so the two windows never
/// disagree about which language the app is speaking.
export async function boot(onReady) {
  setLanguage(await invoke("system_language"));
  translateDocument();
  try {
    await onReady();
  } catch (err) {
    report(err);
  }
}

export { t, translateDocument };
