// English is the source language. Turkish is a full translation, not a subset:
// a missing key would silently fall back and leave a mixed-language screen,
// so `checkParity` fails loudly in development instead.

const en = {
  "app.subtitle": "manager",
  "tool.lang": "Interface language",
  "tool.zoomScroll": "Wheel adjusts the zoom factor",
  "tool.presets": "Preset library",
  "tool.crosshair": "Crosshair",
  "tool.scope": "Scope lens",
  "tool.curvefx": "CurveFX",
  "tool.layers": "Layers",
  "tool.master": "Master control",
  "tool.help": "Guide",
  "tool.panic": "Restore every window (F8)",
  "tool.load": "Load profile",
  "tool.save": "Save profile",

  "card.title": "Window setup",
  "card.index": "WINDOW #{n}",
  "card.close": "Release this window",
  "card.mode.window": "WINDOW",
  "card.mode.squash": "SQUASH",

  "target.title": "Target window",
  "target.hint":
    "The window tlk-grid will control. Run the game in windowed or borderless mode, then pick it here — refresh if it is missing from the list.",
  "target.placeholder": "— select window —",
  "target.refresh": "Refresh the list",

  "monitor.title": "Monitor native resolution",
  "monitor.hint":
    "Your monitor's real resolution. The layout preview below is drawn at this size.",

  "layout.title": "Window position & size",
  "layout.hint":
    "Where the window sits on screen, in real pixels. Drag a box on the preview, use the split buttons, or type numbers: X/Y is the top-left corner, W/H is the size.",
  "layout.center": "Center",
  "layout.left": "Left",
  "layout.right": "Right",
  "layout.top": "Top",
  "layout.bottom": "Bottom",
  "layout.borderless": "Borderless (keeps the rendered resolution)",
  "layout.keepOnAltTab": "Hold the layout through Alt-Tab",
  "layout.saveSize": "Save size & pos",

  "result.label": "Result",
  "result.base": "base",
  "apply": "APPLY",

  "add.window": "ADD WINDOW",

  "status.ready": "Ready — pick a window to begin.",
  "status.listed": "{n} window(s) found.",
  "status.noWindows": "No eligible window found. Run the game in windowed or borderless mode.",
  "status.selected": "{name} selected — {w}×{h} at {x},{y}.",
  "status.applied": "Applied — {w}×{h} at {x},{y}.",
  "status.released": "Window restored.",
  "status.restoredAll": "{n} window(s) restored.",
  "status.nothingToRestore": "Nothing to restore.",
  "status.gone": "That window has closed. Refresh the list.",
  "status.error": "Failed: {message}",
  "status.notBuilt": "{module} arrives in a later release.",

  "binds.title": "Hotkeys",
  "binds.hint":
    "Click a key field, then press the key or mouse button you want. Side buttons work. The bound key is held back from the game while the target window is in front — except the master and F8 binds, which always pass through.",
  "binds.resize": "RESIZE",
  "binds.blackBars": "BLACK BARS",
  "binds.scope": "SCOPE",
  "binds.crosshair": "CROSSHAIR",
  "binds.setKey": "click, then press a key",
  "binds.capturing": "press a key or mouse button…",
  "binds.clear": "Clear this bind",
  "binds.hold": "HOLD",
  "binds.toggle": "TOGGLE",
  "binds.factor": "Factor",

  "layout.pin": "Hold the layout through Alt-Tab",

  "status.bindSet": "{action} bound to {key}.",
  "status.bindCleared": "{action} bind cleared.",
  "status.capturing": "Press the key or mouse button to bind. Esc cancels.",
  "status.engaged": "{action} on.",
  "status.released": "{action} off.",
  "status.wheelNotch": "{action} — wheel {notches}.",
  "status.masterOn": "RESIZE live.",
  "status.masterOff": "RESIZE bypassed — the key goes to the game.",
  "status.wheelOn": "Wheel adjusts the held bind.",
  "status.wheelOff": "Wheel left to the game.",
  "status.pinned": "Layout pinned — it will be restored after Alt-Tab.",
  "status.unpinned": "Layout no longer pinned.",
};

const tr = {
  "app.subtitle": "yönetici",
  "tool.lang": "Arayüz dili",
  "tool.zoomScroll": "Tekerlek zoom katsayısını değiştirir",
  "tool.presets": "Hazır ayar kitaplığı",
  "tool.crosshair": "Nişangâh",
  "tool.scope": "Dürbün lensi",
  "tool.curvefx": "CurveFX",
  "tool.layers": "Katmanlar",
  "tool.master": "Ana denetim",
  "tool.help": "Kılavuz",
  "tool.panic": "Bütün pencereleri geri al (F8)",
  "tool.load": "Profil yükle",
  "tool.save": "Profil kaydet",

  "card.title": "Pencere ayarı",
  "card.index": "PENCERE #{n}",
  "card.close": "Bu pencereyi bırak",
  "card.mode.window": "PENCERE",
  "card.mode.squash": "SQUASH",

  "target.title": "Hedef pencere",
  "target.hint":
    "tlk-grid'in yöneteceği pencere. Oyunu pencereli ya da kenarlıksız modda çalıştır, sonra buradan seç — listede yoksa yenile.",
  "target.placeholder": "— pencere seç —",
  "target.refresh": "Listeyi yenile",

  "monitor.title": "Monitörün gerçek çözünürlüğü",
  "monitor.hint":
    "Monitörünün gerçek çözünürlüğü. Aşağıdaki yerleşim önizlemesi bu ölçüde çizilir.",

  "layout.title": "Pencere konumu ve boyutu",
  "layout.hint":
    "Pencerenin ekranda duracağı yer, gerçek piksel olarak. Önizlemede kutu çiz, bölme düğmelerini kullan ya da sayıyla yaz: X/Y sol üst köşe, W/H boyut.",
  "layout.center": "Ortala",
  "layout.left": "Sol",
  "layout.right": "Sağ",
  "layout.top": "Üst",
  "layout.bottom": "Alt",
  "layout.borderless": "Kenarlıksız (render çözünürlüğünü korur)",
  "layout.keepOnAltTab": "Alt-Tab sonrası yerleşimi koru",
  "layout.saveSize": "Boyutu ve konumu kaydet",

  "result.label": "Sonuç",
  "result.base": "temel",
  "apply": "UYGULA",

  "add.window": "PENCERE EKLE",

  "status.ready": "Hazır — başlamak için bir pencere seç.",
  "status.listed": "{n} pencere bulundu.",
  "status.noWindows": "Uygun pencere yok. Oyunu pencereli ya da kenarlıksız modda çalıştır.",
  "status.selected": "{name} seçildi — {x},{y} konumunda {w}×{h}.",
  "status.applied": "Uygulandı — {x},{y} konumunda {w}×{h}.",
  "status.released": "Pencere eski hâline döndü.",
  "status.restoredAll": "{n} pencere geri alındı.",
  "status.nothingToRestore": "Geri alınacak bir şey yok.",
  "status.gone": "O pencere kapanmış. Listeyi yenile.",
  "status.error": "Başarısız: {message}",
  "status.notBuilt": "{module} sonraki sürümde geliyor.",

  "binds.title": "Kısayollar",
  "binds.hint":
    "Tuş alanına tıkla, sonra istediğin tuşa ya da fare düğmesine bas. Yan tuşlar da çalışır. Bağlanan tuş, hedef pencere öndeyken oyuna geçmez — ana denetim ve F8 hariç; onlar her zaman geçer.",
  "binds.resize": "BÜYÜT",
  "binds.blackBars": "SİYAH BANT",
  "binds.scope": "DÜRBÜN",
  "binds.crosshair": "NİŞANGÂH",
  "binds.setKey": "tıkla, sonra bir tuşa bas",
  "binds.capturing": "bir tuşa ya da fare düğmesine bas…",
  "binds.clear": "Bu bağlamayı temizle",
  "binds.hold": "BASILI",
  "binds.toggle": "AÇ/KAPA",
  "binds.factor": "Katsayı",

  "layout.pin": "Alt-Tab sonrası yerleşimi koru",

  "status.bindSet": "{action} → {key}.",
  "status.bindCleared": "{action} bağlaması temizlendi.",
  "status.capturing": "Bağlanacak tuşa ya da fare düğmesine bas. Esc iptal eder.",
  "status.engaged": "{action} açık.",
  "status.released": "{action} kapalı.",
  "status.wheelNotch": "{action} — tekerlek {notches}.",
  "status.masterOn": "BÜYÜT etkin.",
  "status.masterOff": "BÜYÜT devre dışı — tuş oyuna gidiyor.",
  "status.wheelOn": "Tekerlek basılı bağlamayı ayarlıyor.",
  "status.wheelOff": "Tekerlek oyuna bırakıldı.",
  "status.pinned": "Yerleşim sabitlendi — Alt-Tab sonrası geri gelecek.",
  "status.unpinned": "Yerleşim sabitlemesi kaldırıldı.",
};

const dictionaries = { en, tr };

let active = "en";

export function setLanguage(tag) {
  active = dictionaries[tag] ? tag : "en";
  document.documentElement.lang = active;
  return active;
}

export function language() {
  return active;
}

export function other() {
  return active === "en" ? "tr" : "en";
}

export function t(key, vars) {
  const text = dictionaries[active][key] ?? dictionaries.en[key] ?? key;
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name) =>
    Object.hasOwn(vars, name) ? String(vars[name]) : whole
  );
}

/// Rewrites every element carrying a data-i18n attribute. Called on boot and
/// again on every language switch, so the toolbar toggle needs no reload.
export function translateDocument(root = document) {
  for (const node of root.querySelectorAll("[data-i18n]")) {
    node.textContent = t(node.dataset.i18n);
  }
  for (const node of root.querySelectorAll("[data-i18n-title]")) {
    node.title = t(node.dataset.i18nTitle);
  }
  for (const node of root.querySelectorAll("[data-i18n-aria]")) {
    node.setAttribute("aria-label", t(node.dataset.i18nAria));
  }
}

export function missingKeys() {
  const source = Object.keys(en);
  const gaps = {};
  for (const [tag, dict] of Object.entries(dictionaries)) {
    const absent = source.filter((key) => !(key in dict));
    if (absent.length) gaps[tag] = absent;
  }
  return gaps;
}
