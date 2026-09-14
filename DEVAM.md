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
| Lisans | MIT | Yayılım kapatılıp satılma riskinden kıymetli. |
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

Kısayolların oyun içinde çalışması ve tuşun oyuna sızmaması `WH_KEYBOARD_LL` /
`WH_MOUSE_LL` + `GetForegroundWindow` kapısı demek. `RegisterHotKey` fare yan
tuşlarını (M4/M5) alamadığı için yetmiyor — Faz 2'nin çekirdeği bu.

## Durum

**Faz 0 — bitti.** Tauri 2 kabuğu, koyu başlık çubuğu, tepsi ikonu (Aç / Hepsini
geri al / Çıkış), kapatınca tepsiye inme, tek dosya 8.3 MB exe.

**Faz 1 — bitti.** `core/` içinde:
- `target.rs` — `EnumWindows` keşfi, HWND/PID, tekrar eden başlıkların `#1 #2 #3`
  numaralanması, cloaked/tool/owned pencere elemesi.
- `display.rs` — monitör listesi, EDID adı, native mod, Hz, DPI ölçeği.
- `layout.rs` — saf matematik, 14 test: bölme ızgarası (×4=2×2 … ×10=5×2, iki
  satır sabit), en-boy "contain" yerleşimi, letterbox bantları, zoom kelepçesi ve
  ondalık virgül ayrıştırma.
- `frame.rs` — gölge payı düzeltmeli `place`, kenarlıksız stil soyma, `capture` /
  `restore` çifti.

Arayüz tarafı: üst bar + ikon şeridi, sütun kart tuvali, `+ PENCERE EKLE` kartı,
ölçekli önizleme (hücre tıklama + kutu çizme), X/Y/W/H, alt durum çubuğu.
Doğrulandı: gerçek makinede TR otomatik seçildi, monitör `1920×1080 @ 144 Hz`
okundu, 14 test ve `clippy -D warnings` temiz.

## Sırada — Faz 2 (bind altyapısı)

1. Ayrı iplikte `WH_KEYBOARD_LL` + `WH_MOUSE_LL` hook'u, kendi mesaj döngüsüyle.
   Ana iplik bloklanırsa Windows hook'u sessizce düşürür.
2. Odak kapısı: bind yalnız hedef pencere öndeyken tetiklenir.
3. Hold / Toggle modları, tuşun oyuna sızmaması, master bind'in **hiçbir zaman**
   tuşu yutmaması.
4. F8'in global hale gelmesi (şu an sadece uygulama odaktayken).
5. `SetWinEventHook` nöbetçisi: Alt-Tab sonrası rect'i yeniden dayatma.

**Kabul:** RMB'ye bağlı bind oyun içinde tetikleniyor ve tuş oyuna sızmıyor;
alt-tab sonrası pencere konumunu koruyor; F8 her şeyi orijinaline döndürüyor.

## Doğrulanmamış, Faz 1'de denenecek

- **DPI modu.** Başka bir sürecin DPI farkındalığı çalışma anında
  değiştirilemiyor. En olası yol `HKCU\...\AppCompatFlags\Layers` per-app bayrağı
  — ama bu hedefin yeniden başlatılmasını gerektirir. Gerçek bir oyunla denenip
  doğrulanmazsa modül "yeniden başlatma gerekir" etiketiyle çıkacak.
- **WGC sarı çerçeve.** Windows 10 1903-1909'da pencere yakalamada zorunluydu;
  `IsBorderRequired` kontrolü ve gerekirse thumbnail moduna düşme Faz 5'te.

## Tuzaklar (yaşandı)

- `windows` 0.62'de `MONITORINFOF_PRIMARY` `Win32::Graphics::Gdi` altında yok;
  sabit elle yazıldı. `HWND` `Win32::Foundation`'da, `WindowsAndMessaging`'de değil.
- Kart şablonunda (`<template>`) `id` kullanılamaz — her klon aynı id'yi
  çoğaltır. Etiketler `<label class="field"><span>` sarmalıyla kuruldu.
- `PrintWindow` ile pencere yakalarken `GetClientRect` değil `GetWindowRect`
  ölçüsü lazım; yoksa alt kısım kırpılır (durum çubuğu görünmez sanılır).
