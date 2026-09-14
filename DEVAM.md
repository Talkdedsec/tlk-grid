# tlk-grid — devam

WinGrid'in (Steam app/4847000, Zhazira, 3 Tem 2026) açık kaynak, sıfırdan
yazılmış karşılığı. Kapsam kesilmiyor: 30 özelliğin 27'si birebir, 2'si
uyarlanmış, 1'i (Steam Workshop/Cloud) App ID gerektirdiği için yerine geçen bir
preset deposu alıyor.

Tam envanter ve API haritası:
https://claude.ai/code/artifact/1a4d2e32-785c-4d8c-a57b-b0c30ddbf11a

## Kararlar

| Konu | Karar | Neden |
| --- | --- | --- |
| Stack | Rust + Tauri 2 + WebView2 | Kaynak uygulamanın arayüzü de WebView2/HTML; aynı motorla UI/UX birebir tutturuluyor. Tek 8 MB exe, .NET bağımlılığı yok. |
| Lisans | MIT | Yayılım, kapatılıp satılma riskinden kıymetli. |
| Dağıtım | GitHub Releases (+ winget) | Steam tarafı kapsam dışı bırakıldı. |
| Dil | Ana dil İngilizce, TR tam çeviri | Windows yereli Türkçeyse TR açılır; araç çubuğundaki EN/TR anında değiştirir. |
| Kod dili | İngilizce | Depo public; katkı önünü açmak için. Türkçe yalnız i18n sözlüğünde. |
| Workshop yerine | `tlk-grid-presets` public deposu + `index.json` | App ID olmadan Workshop mümkün değil. |

## Asıl mekanizma

Ürünün tamamı dört API kararının üstünde duruyor:

1. `DwmRegisterThumbnail` + `rcSource` — hedef pencerenin canlı GPU kopyasının
   bir parçasını istediğin yere büyütür. **Thumbnail Zoom, Scope lensi ve Layers
   bu tek çağrının üç kullanımı.** Faz 3-4'ün tamamı buraya bağlı.
2. `SetWindowPos` + stil soyma, `DWMWA_EXTENDED_FRAME_BOUNDS` ile görünmez gölge
   payının düşülmesi. Bu düzeltme olmadan Win11'de her pencere ~7 px kayar.
3. `Windows.Graphics.Capture` → D3D11 → HLSL — CurveFX/CRT/motion blur. Thumbnail
   piksel erişimi vermediği için efektler ayrı yakalama hattına düşüyor.
4. NvAPI custom display — SQUASH. NVIDIA dışında karşılığı yok.

## Durum

**Faz 0 — bitti.** Tauri 2 kabuğu, koyu başlık çubuğu, tepsi ikonu (Aç / Hepsini
geri al / Çıkış), kapatınca tepsiye inme, tek dosya 8.4 MB exe.

**Faz 1 — bitti.** `core/` içinde:
- `target.rs` — `EnumWindows` keşfi, HWND/PID, tekrar eden başlıkların `#1 #2 #3`
  numaralanması, cloaked/tool/owned pencere elemesi.
- `display.rs` — monitör listesi, EDID adı, native mod, Hz, DPI ölçeği.
- `layout.rs` — saf matematik: bölme ızgarası (×4=2×2 … ×10=5×2, iki satır
  sabit), en-boy "contain" yerleşimi, letterbox bantları, zoom kelepçesi ve
  ondalık virgül ayrıştırma.
- `frame.rs` — gölge payı düzeltmeli `place`, kenarlıksız stil soyma, `capture` /
  `restore` çifti.

**Faz 2 — bitti.**
- `bind.rs` — tetikleyici (tuş / fare, M4-M5 dahil), hold/toggle, eylem kümesi,
  VK adlandırma. Saf veri, testli.
- `input.rs` — `WH_KEYBOARD_LL` + `WH_MOUSE_LL` + `SetWinEventHook`, hepsi tek
  ipliğin kendi mesaj döngüsünde. Karar mantığı (`decide`, `decide_wheel`)
  Win32'den ayrı tutuldu; 11 test sistem girdisi üretmeden kuralları doğruluyor.
- `app/hotkeys.rs` — hook ipliğinden gelen olayları alıp pencereye dokunan taraf.
- Arayüz: kart içinde KISAYOLLAR bölümü, tıkla-bas bağlama alanı, temizleme,
  HOLD/TOGGLE anahtarı; araç çubuğunda tekerlek ve ana denetim düğmeleri gerçek
  anahtar oldu.

Kurallar: bağlanan tuş yalnız hedef pencere öndeyken yutulur; ana denetim ve F8
her yerde çalışır ve tuşu **asla** yutmaz; tekerlek en son basılan bağlamayı
ayarlar; tuş tekrarı eylemi bir kez tetikler.

## Neyin kanıtı var

| İddia | Kanıt |
| --- | --- |
| Hook kuruluyor | Uygulama açılıyor; `SetWindowsHookExW` başarısız olsa `setup` hata döner ve açılmazdı |
| F8 arka planda çalışıyor | `SendInput` ile uygulama odakta değilken F8 gönderildi; durum çubuğu gerçek sayıyla yanıtladı |
| Pencere keşfi | Canlı makinede "7 pencere bulundu", ayrıca `target.rs`'te canlı smoke test |
| Monitör okuma | `Generic PnP Monitor 1920×1080 @ 144 Hz` |
| TR otomatik | Uygulama Türkçe açtı, EN/TR düğmesi yerinde |
| Bağlama kuralları | 31 test, `clippy -D warnings` temiz |

Henüz **gerçek bir oyunla** doğrulanmadı: tuşun oyuna sızmaması (yutma) ve
Alt-Tab nöbetçisinin sahada davranışı. İkisinin de birim testi var, saha testi yok.

## Sırada — Faz 3 (thumbnail zoom)

Katmanlı overlay penceresi, `DwmRegisterThumbnail`, `rcSource` merkez hesabı,
0.01×–64× faktör, tekerlekle canlı ayar, ×1.25/×1.5/×2, WINDOW ve STRETCH
yöntemleri. Faz 2'nin `Engaged`/`Released`/`Wheel` olayları zaten bu modülü
bekliyor.

**Kabul:** bind basılıyken görüntü FPS düşüşü olmadan büyüyor, bırakınca anında
dönüyor, titreme yok.

## Kaynak uygulamanın yama notlarından çıkanlar

Mağaza açıklaması 14 Eyl itibarıyla neredeyse aynı (sadece bir destek satırı
eklenmiş), ama duyurularda plana giren ayrıntılar var:

- **SQUASH artık kendi araç çubuğu düğmesinde** (11 Eyl). Kart içinde mod rozeti
  olarak değil, ayrı pencere olarak kurulacak.
- **Özel çözünürlük oluşturmada bozuk modlar baştan reddediliyor** — EDID limit
  kontrolü uygulamadan önce.
- **CurveFX:** eğrilik aralığı −200…+200; en-boy oranını koruyan "Window Size"
  kaydırıcısı; pikselleştirme ve CRT birlikte çalışır (kaynakta CRT açıkken
  pikselleştirme kapanıyordu, düzeltilmiş).
- **Motion Blur:** ayarları sıfırlamayan ON/OFF anahtarı; blur kaydırıcısı
  Shutter modunda da etkili; arayüz yalnız seçili modda anlamlı ayarları gösterir;
  Center koruması iki modda da var.
- **Scope bind'i:** kısa basış lensi aç/kapa, basılı tutmak **her zaman** açar;
  basılıyken tekerlek yalnız lensi boyutlandırır (tam ekran zoom'a kazara geçmez);
  boyutlandırırken yarı saydam kare + parlak kenarlık gösterilir.
- **Tekerlek adımları:** nişangâh ±%10, dürbün ±25 px / tık.
- **Otomatik temizlik:** oyun penceresi kapanınca thumbnail, layer, nişangâh ve
  dürbün kaldırılır.
- **Alt-Tab:** oyundan çıkınca overlay kendini gizler, dönünce anında geri gelir;
  hold modundaki bağlamalar kendiliğinden yeniden tetiklenmez.
- **LoL/Dota notu:** imleç koordinatı yeniden eşleme anti-cheat yüzünden
  reddediliyor. Bizde de kırmızı çizgi kalacak.

## Tuzaklar (yaşandı)

- `windows` 0.62'de `MONITORINFOF_PRIMARY` `Win32::Graphics::Gdi` altında yok;
  sabit elle yazıldı. `HWND` `Win32::Foundation`'da, `WindowsAndMessaging`'de değil.
- Kart şablonunda (`<template>`) `id` kullanılamaz — her klon aynı id'yi
  çoğaltır. Etiketler `<label class="field"><span>` sarmalıyla kuruldu.
- `PrintWindow` ile pencere yakalarken `GetClientRect` değil `GetWindowRect`
  ölçüsü lazım; yoksa alt kısım kırpılır (durum çubuğu yokmuş gibi görünür).
- CSS'te `.result { display: inline-block }` tarayıcının `[hidden]` varsayılanını
  eziyor; `display` veren her kurala `[hidden] { display: none }` eşlik etmeli.
- Hook geri çağrısında bloklayan `lock()` tüm makinenin klavyesini dondurur;
  `try_lock` + çekişmede geçir şart.
