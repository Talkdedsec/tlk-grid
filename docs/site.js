(() => {
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const root = document.documentElement;
  root.classList.add('js');

  /* ---------- language ---------- */
  const TR = {
    'skip': 'İçeriğe geç',
    'nav.zoom': 'Büyütme', 'nav.how': 'Nasıl çalışır', 'nav.road': 'Yol haritası', 'nav.get': 'İndir',
    'hero.over': 'Windows 10 1903+ · Win32 + DWM · MIT · açık kaynak demo',
    'hero.a': 'Görüntü büyür.', 'hero.b': 'FPS yerinde kalır.',
    'hero.lede': 'Oyun penceresini, oyunun render çözünürlüğüne dokunmadan boyutlandır, yerleştir ve büyüt. Hiçbir şey yeniden render edilmez; sadece pencere ve onun canlı DWM kopyası boyut değiştirir.',
    'rig.aria': 'Pencere yerleşimi demosu', 'rig.layout': 'Yerleşim', 'rig.window': 'Pencere', 'game.aria': 'Sabit çözünürlükte render eden bir oyun',
    'l.full': 'Tam ekran', 'l.left': 'Sol yarı', 'l.right': 'Sağ yarı', 'l.borderless': 'Kenarlıksız', 'l.restore': 'Geri al',
    'r.render': 'render',
    'tape.aria': 'Durum',
    'tape.run': 'DEMO, SÜRÜM DEĞİL · DEMO, SÜRÜM DEĞİL · DEMO, SÜRÜM DEĞİL · DEMO, SÜRÜM DEĞİL · DEMO, SÜRÜM DEĞİL · DEMO, SÜRÜM DEĞİL ·',
    'tape.p': 'Aşağıdaki her şey çalışıyor ve gerçek pencerelere, gerçek bir klavyeye karşı test edildi, ama <b>hiçbiri bir maç boyunca kullanılmadı</b>. Yaklaşımın ne yaptığını görmek için kullan, önemli bir şeyin ortasında değil; çevrimiçi kullanmadan önce anti-cheat notunu oku.',
    'z.over': '01 — Büyütme',
    'z.h': 'Pencereyi büyüt. <span class="acc">Oyunun haberi olmaz.</span>',
    'z.p': 'Bağlamayı basılı tut, pencerenin canlı DWM kopyası bir overlay üzerinde büyütülür. Oyun ne oynatılır ne de haberi olur; kendi çözünürlüğünde render etmeye devam eder. Basılıyken tekerlekle katsayıyı değiştir.',
    'z.try': 'Yukarıdaki pencerede dene: <b>basılı tut</b> ya da <kbd>Z</kbd> tuşunu basılı tut. Tekerlek katsayıyı değiştirir.',
    'z.factor': 'katsayı', 'z.method': 'yöntem',
    'z.thumb': 'Kaynak dikdörtgeni verilmiş DwmRegisterThumbnail. Overlay büyür; oyun penceresi olduğu yerde kalır.',
    'z.win': 'Bunun yerine gerçek pencereyi oynatır, bıraktığın anda geri koyar.',
    'f.over': '02 — Ne yapar',
    'f1.h': 'Pencereyi tam istediğin yere koyar', 'f1.p': 'Hızlı yarımlar, ×4 ×6 ×8 ×10 bölme hücreleri, en-boy ön ayarları ya da elle X Y W H; hepsi monitörünün ölçekli önizlemesi üzerinde.',
    'f2.h': 'İstendiğinde kenarlıksız', 'f2.p': 'Başlık çubuğunu ve çerçeveyi soyar, render çözünürlüğünü korur.',
    'f3.h': 'Çoklu instance', 'f3.p': 'Aynı oyunun iki kopyası pencere tanıtıcısı ve süreç kimliğiyle ayrılır, #1 #2 #3 diye numaralanır.',
    'f4.h': 'Düzgün davranan kısayollar', 'f4.p': 'Bir tuş ya da farenin yan düğmesi. Bağlama yalnız hedef öndeyken tetiklenir, tuş oyuna geçmez. Ana denetim ve F8 tuşunu asla yutmaz.',
    'f5.h': 'Yerleşimi tutar', 'f5.p': 'Pencereyi sabitle; oyun Alt-Tab sonrası geri oynattığında yerleşim yeniden dayatılır.',
    'f6.h': 'Kaldığın yerden devam eder', 'f6.p': 'Düzen bir dahaki açılışta geri gelir. Adlı profiller pencerelerini çalıştırılabilir dosyadan bulur; yeniden başlatmayı ve değişen başlığı atlatır.',
    'h.over': '03 — Nasıl çalışır', 'h.h': 'Dört belgelenmiş API. <span class="acc">Başka hiçbir şey.</span>',
    'a1': 'Keşif, konumlandırma, kenarlıksız', 'a2': 'Piksel doğruluğunda çerçeveleme', 'a3': 'Büyütme', 'a4': 'Dürbün lensi, HUD katmanları', 'a5': 'Kavisli ekran, CRT, motion blur', 'a6': 'Özel ekran modları',
    'planned': 'planlı',
    'no.1': 'Kod enjeksiyonu', 'no.2': 'Kernel sürücüsü', 'no.3': 'Başka sürecin belleğini okumak',
    'h.caution': '<b>Önemli uyarı.</b> Bazı kernel seviyesi anti-cheat’ler, nasıl yapıldığından bağımsız olarak, en üstte duran overlay pencerelerine ve oyun penceresini dışarıdan oynatan programlara itiraz eder. Çevrimiçi kullanmadan önce oyunun kurallarına bak.',
    'r.over': '04 — Yol haritası',
    'p0': 'Kabuk, tray, tek dosya derleme', 'p1': 'Keşif, monitörler, yerleşim, konumlandırma, F8', 'p2': 'Kısayollar: low-level hook, basılı/aç-kapa, ana denetim, Alt-Tab koruması',
    'p3': 'Thumbnail büyütme, window ve stretch yöntemleri, tekerlek katsayısı', 'p4': 'Siyah bantlar, özel overlay’ler, nişangah, dürbün lensi, HUD katmanları',
    'p5': 'CurveFX: kavisli ekran, CRT, dört motion-blur modu', 'p6': 'SQUASH: NvAPI özel ekran modları', 'p7': 'Profiller, ön ayar kütüphanesi, dil değişimi', 'p8': 'Rehber, tanılama, sürüm paketleme',
    'next': 'sıradaki', 'partial': 'kısmen',
    'g.over': '05 — İndir', 'g.h': 'Tek exe. <span class="acc">F8 her şeyi geri alır.</span>',
    'req.1': 'Windows 10 1903 (build 18362) ya da üstü, 64-bit', 'req.2': 'Desktop Window Manager açık bir GPU', 'req.3': 'WebView2 çalışma zamanı, Windows 10 ve 11’de hazır gelir', 'req.4': 'Oyun pencereli ya da kenarlıksız pencereli modda',
    'g.build': 'demo derleme · x64', 'g.attached': 'ekli', 'g.src': 'ya da derle',
    'g.credit': 'Özellik seti ve etkileşim modeli, fikrin ilk ortaya çıktığı yer olan Zhazira’nın <a href="https://store.steampowered.com/app/4847000/WinGrid/">WinGrid</a>’inden örnek alındı. Ondan hiçbir kod, görsel ya da metin kullanılmadı.',
    'copy': 'Kopyala', 'copied': 'Kopyalandı',
    'foot.arch': 'Mimari', 'foot.log': 'Değişiklikler', 'foot.sec': 'Güvenlik'
  };
  let lang = 'en';
  const orig = new Map();
  const keep = (el, a, get) => { if (!orig.has(el)) orig.set(el, {}); const o = orig.get(el); if (!(a in o)) o[a] = get(); return o[a]; };
  function applyLang(l) {
    lang = l; root.lang = l;
    const tr = k => l === 'tr' ? TR[k] : undefined;
    $$('[data-i18n]').forEach(el => { const en = keep(el, 't', () => el.textContent); el.textContent = tr(el.dataset.i18n) ?? en; });
    $$('[data-i18n-html]').forEach(el => { const en = keep(el, 'h', () => el.innerHTML); el.innerHTML = tr(el.dataset.i18nHtml) ?? en; });
    $$('[data-i18n-aria]').forEach(el => { const en = keep(el, 'a', () => el.getAttribute('aria-label')); el.setAttribute('aria-label', tr(el.dataset.i18nAria) ?? en); });
    try { localStorage.setItem('tlk-grid-lang', l); } catch {}
  }
  $('.lang').addEventListener('click', () => applyLang(lang === 'en' ? 'tr' : 'en'));
  addEventListener('scroll', () => document.body.classList.toggle('scrolled', scrollY > 8), { passive: true });

  /* ---------- the "game": rendered at one fixed size no matter how big the window is ---------- */
  const game = $('.game'), g = game.getContext('2d'), lens = $('.lens'), lz = lens.getContext('2d');
  const W = game.width, H = game.height, HZ = H * .56;
  const stars = Array.from({ length: 70 }, (_, i) => [(i * 137.5) % W, ((i * 71.3) % (HZ - 40)), (i % 3) + 1]);
  let t0 = performance.now(), visible = true, fps = 144;
  function draw(now) {
    const t = reduce ? 0 : (now - t0) / 1000;
    const sky = g.createLinearGradient(0, 0, 0, HZ);
    sky.addColorStop(0, '#03070c'); sky.addColorStop(.7, '#062222'); sky.addColorStop(1, '#0d4a3b');
    g.fillStyle = sky; g.fillRect(0, 0, W, HZ);
    g.fillStyle = 'rgba(200,255,235,.55)';
    stars.forEach(([x, y, s]) => g.fillRect(x, y, s * .8, s * .8));
    // sun with cut bands
    const sx = W * .5, sy = HZ - 10, R = 120;
    const sun = g.createLinearGradient(0, sy - R, 0, sy);
    sun.addColorStop(0, '#ffe36b'); sun.addColorStop(1, '#00cb91');
    g.save(); g.beginPath(); g.arc(sx, sy, R, Math.PI, 0); g.clip();
    g.fillStyle = sun; g.fillRect(sx - R, sy - R, R * 2, R);
    g.fillStyle = '#062222';
    for (let i = 0; i < 6; i++) { const y = sy - 12 - i * 16; g.fillRect(sx - R, y, R * 2, 2 + (5 - i) * 1.4); }
    g.restore();
    // mountains
    g.fillStyle = '#04100f'; g.beginPath(); g.moveTo(0, HZ);
    for (let x = 0; x <= W; x += 40) g.lineTo(x, HZ - 30 - Math.abs(Math.sin(x * .011) * 70) - Math.abs(Math.sin(x * .031)) * 26);
    g.lineTo(W, HZ); g.fill();
    g.strokeStyle = 'rgba(0,203,145,.55)'; g.lineWidth = 1.2; g.stroke();
    // floor grid, scrolling towards you
    g.fillStyle = '#020607'; g.fillRect(0, HZ, W, H - HZ);
    g.strokeStyle = 'rgba(0,203,145,.6)'; g.lineWidth = 1;
    for (let i = -14; i <= 14; i++) { g.beginPath(); g.moveTo(W / 2 + i * 14, HZ); g.lineTo(W / 2 + i * 130, H); g.stroke(); }
    const off = (t * .9) % 1;
    for (let k = 0; k < 14; k++) { const z = (k + off) / 14, y = HZ + (H - HZ) * z * z; g.globalAlpha = .25 + z * .75; g.beginPath(); g.moveTo(0, y); g.lineTo(W, y); g.stroke(); }
    g.globalAlpha = 1;
    // HUD
    g.font = '500 18px "Geist Mono", monospace'; g.fillStyle = '#7fe6c4';
    if (!reduce && Math.random() < .03) fps = 143 + Math.round(Math.random());
    g.fillText('FPS ' + fps, 22, 34);
    g.fillStyle = 'rgba(228,233,238,.6)'; g.textAlign = 'right'; g.fillText('1920×1080', W - 22, 34); g.textAlign = 'left';
    g.strokeStyle = '#e4e9ee'; g.lineWidth = 2;
    const cx = W / 2, cy = H / 2 + 20;
    [[-18, 0, -7, 0], [7, 0, 18, 0], [0, -18, 0, -7], [0, 7, 0, 18]].forEach(([a, b, c, d]) => { g.beginPath(); g.moveTo(cx + a, cy + b); g.lineTo(cx + c, cy + d); g.stroke(); });
    g.fillStyle = '#00cb91'; g.fillRect(22, H - 40, 180, 8); g.fillStyle = 'rgba(228,233,238,.6)'; g.fillText('HP 100', 22, H - 52);
    g.beginPath(); g.arc(W - 80, H - 80, 52, 0, Math.PI * 2); g.fillStyle = 'rgba(0,0,0,.5)'; g.fill(); g.strokeStyle = 'rgba(0,203,145,.5)'; g.lineWidth = 1.5; g.stroke();
    g.fillStyle = '#ffcf3a'; g.fillRect(W - 82, H - 82, 5, 5);
    g.fillStyle = '#ff5d73'; g.fillRect(W - 80 + Math.cos(t) * 30, H - 80 + Math.sin(t * 1.3) * 22, 4, 4);
    if (zoom.on) drawLens();
  }
  function frame(now) { if (visible) draw(now); requestAnimationFrame(frame); }
  new IntersectionObserver(([e]) => { visible = e.isIntersecting; }).observe(game);

  /* ---------- zoom: a magnified copy of a source rectangle, like rcSource on a DWM thumbnail ---------- */
  const gw = $('#gw'), zf = $('#z-f');
  const zoom = { on: false, f: 2.5, x: .5, y: .55 };
  function drawLens() {
    const sw = W / zoom.f, sh = H / zoom.f;
    const sx = Math.max(0, Math.min(W - sw, zoom.x * W - sw / 2)), sy = Math.max(0, Math.min(H - sh, zoom.y * H - sh / 2));
    lz.imageSmoothingEnabled = true;
    lz.drawImage(game, sx, sy, sw, sh, 0, 0, W, H);
    lz.strokeStyle = '#00cb91'; lz.lineWidth = 4; lz.strokeRect(2, 2, W - 4, H - 4);
    lz.font = '500 16px "Geist Mono", monospace'; lz.fillStyle = '#00cb91'; lz.fillText('DWM THUMBNAIL · ' + zoom.f.toFixed(1) + '×', 18, H - 18);
  }
  const setF = f => { zoom.f = Math.max(1.5, Math.min(6, Math.round(f * 2) / 2)); zf.textContent = zoom.f.toFixed(1) + '×'; };
  const zon = () => { zoom.on = true; gw.classList.add('zooming'); drawLens(); };
  const zoff = () => { zoom.on = false; gw.classList.remove('zooming'); };
  const aim = e => { const r = game.getBoundingClientRect(); zoom.x = (e.clientX - r.left) / r.width; zoom.y = (e.clientY - r.top) / r.height; };
  gw.addEventListener('pointerdown', e => { gw.setPointerCapture(e.pointerId); aim(e); zon(); });
  gw.addEventListener('pointermove', e => { if (zoom.on) { aim(e); drawLens(); } });
  ['pointerup', 'pointercancel', 'lostpointercapture'].forEach(ev => gw.addEventListener(ev, zoff));
  gw.addEventListener('wheel', e => { if (!zoom.on) return; e.preventDefault(); setF(zoom.f + (e.deltaY < 0 ? .5 : -.5)); drawLens(); }, { passive: false });
  addEventListener('keydown', e => { if ((e.key === 'z' || e.key === 'Z') && !e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey) { zoom.x = .5; zoom.y = .55; zon(); } });
  addEventListener('keyup', e => { if (e.key === 'z' || e.key === 'Z') zoff(); });

  /* ---------- layout on a 2560×1440 monitor ---------- */
  const MW = 2560, MH = 1440, START = { x: 640, y: 360, w: 1280, h: 720 };
  const screen = $('.screen'), cells = $('.cells');
  cells.innerHTML = '<i></i>'.repeat(4);
  cells.style.gridTemplateColumns = 'repeat(2, 1fr)';
  let cur = { ...START }, cell = -1;
  const shown = { x: START.x, y: START.y, w: START.w, h: START.h };
  function place(r) {
    cur = r;
    Object.assign(gw.style, { left: r.x / MW * 100 + '%', top: r.y / MH * 100 + '%', width: r.w / MW * 100 + '%', height: r.h / MH * 100 + '%' });
    countTo(r);
  }
  function countTo(r) {
    const from = { ...shown }, t1 = performance.now(), D = reduce ? 0 : 700;
    const step = now => {
      const k = D ? Math.min(1, (now - t1) / D) : 1, e = k < .5 ? 8 * k ** 4 : 1 - (-2 * k + 2) ** 4 / 2;
      for (const key of ['x', 'y', 'w', 'h']) { shown[key] = Math.round(from[key] + (r[key] - from[key]) * e); $('#r-' + key).textContent = shown[key]; }
      if (k < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }
  function aspect(a) {
    const m = MW / MH;
    if (a >= m) { const h = Math.round(MW / a); return { x: 0, y: Math.round((MH - h) / 2), w: MW, h }; }
    const w = Math.round(MH * a); return { x: Math.round((MW - w) / 2), y: 0, w, h: MH };
  }
  const LAYOUT = {
    full: () => ({ x: 0, y: 0, w: MW, h: MH }),
    left: () => ({ x: 0, y: 0, w: MW / 2, h: MH }),
    right: () => ({ x: MW / 2, y: 0, w: MW / 2, h: MH }),
    x4: () => { cell = (cell + 1) % 4; return { x: (cell % 2) * MW / 2, y: Math.floor(cell / 2) * MH / 2, w: MW / 2, h: MH / 2 }; },
    '21:9': () => aspect(21 / 9), '32:9': () => aspect(32 / 9), '4:3': () => aspect(4 / 3), '1:1': () => aspect(1)
  };
  $$('[data-l]', $('.ctrl')).forEach(b => b.addEventListener('click', () => {
    if (b.dataset.l !== 'x4') cell = -1;
    $$('[data-l]', $('.ctrl')).forEach(x => x.classList.toggle('on', x === b));
    screen.classList.toggle('show-cells', b.dataset.l === 'x4');
    place(LAYOUT[b.dataset.l]());
  }));
  const bl = $('#borderless');
  bl.addEventListener('click', () => { const on = bl.getAttribute('aria-pressed') !== 'true'; bl.setAttribute('aria-pressed', on); gw.classList.toggle('borderless', on); });
  function restore() {
    cell = -1; screen.classList.remove('show-cells');
    $$('[data-l]', $('.ctrl')).forEach(x => x.classList.remove('on'));
    bl.setAttribute('aria-pressed', 'false'); gw.classList.remove('borderless');
    place({ ...START });
  }
  $('#f8').addEventListener('click', restore);
  addEventListener('keydown', e => { if (e.key === 'F8') { e.preventDefault(); restore(); } });

  /* ---------- copy + reveals ---------- */
  $$('.copy').forEach(b => b.addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(b.previousElementSibling.textContent); } catch { return; }
    b.textContent = lang === 'tr' ? TR.copied : 'Copied'; b.classList.add('ok');
    setTimeout(() => { b.textContent = lang === 'tr' ? TR.copy : 'Copy'; b.classList.remove('ok'); }, 1400);
  }));
  $$('.phases li').forEach((li, i) => li.style.setProperty('--i', i));
  const io = new IntersectionObserver(es => es.forEach(e => { if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); } }), { rootMargin: '0px 0px -10% 0px' });
  $$('.reveal').forEach(el => io.observe(el));

  /* ---------- start ---------- */
  let saved = null;
  try { saved = localStorage.getItem('tlk-grid-lang'); } catch {}
  const q = new URLSearchParams(location.search).get('lang');
  applyLang(q === 'tr' || q === 'en' ? q : saved || (/^tr\b/i.test(navigator.language) ? 'tr' : 'en'));
  place({ ...START });
  document.fonts.ready.then(() => draw(performance.now()));
  requestAnimationFrame(frame);
})();
