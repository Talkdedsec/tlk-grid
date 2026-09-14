# tlk-grid

Oyun penceresini, oyunun kendi render çözünürlüğüne dokunmadan boyutlandır,
konumlandır ve büyüt. Görüntü büyür, FPS yerinde kalır — çünkü hiçbir şey
yeniden render edilmez; sadece pencere ve pencerenin canlı DWM kopyası boyut
değiştirir.

Açık kaynak, yalnız Windows. [English](README.md)

> **Durum: erken.** Pencere keşfi, monitör tespiti, yerleşim, konumlandırma,
> kısayollar ve büyütme çalışıyor. Katmanlar, CurveFX ve SQUASH aşağıdaki yol
> haritasında; henüz derlemede yok.

## Ne yapar

- **Pencereyi tam istediğin yere koyar** — hızlı yarımlar, ×4/×6/×8/×10 bölme
  hücreleri, en-boy ön ayarları (21:9, 32:9, 16:9, 4:3, 16:10, 1:1) ya da elle
  X/Y/W/H. Hepsi monitörünün ölçekli önizlemesi üzerinde çizilir.
- **İstendiğinde kenarlıksız** — başlık çubuğunu ve çerçeveyi soyar, render
  çözünürlüğünü korur.
- **Çoklu instance** — aynı oyunun iki kopyası pencere tanıtıcısı ve süreç
  kimliğiyle ayrılır, `#1 #2 #3` diye numaralanır.
- **Düzgün davranan kısayollar** — bir tuşa ya da farenin yan düğmesine bağla;
  bağlama yalnız hedef pencere öndeyken tetiklenir ve tuş oyuna geçmez. Ana
  denetim, uygulamayı açmadan bunu askıya alır; ne ana denetim ne de F8 tuşu yutar.
- **Oyuna dokunmadan büyütür** — bağlamayı basılı tut, pencerenin canlı DWM
  kopyası bir overlay üzerinde büyütülür. Oyun ne oynatılır ne de haberi olur;
  kendi çözünürlüğünde render etmeye devam eder, FPS yerinde kalır. Basılıyken
  tekerlek katsayıyı değiştirir. `WINDOW` ve `STRETCH` bunun yerine gerçek
  pencereyi oynatır ve bırakınca geri koyar.
- **Yerleşimi tutar** — pencereyi sabitlersen, oyun Alt-Tab sonrası geri
  oynattığında yerleşim yeniden dayatılır.
- **Geri alır** — F8, tlk-grid'in dokunduğu her pencereyi eski stiline ve
  konumuna döndürür; her yerden çalışır.

## Nasıl çalışır

Hepsi dört belgelenmiş Windows API'siyle:

| Parça | API |
| --- | --- |
| Pencere keşfi, konumlandırma, kenarlıksız | Win32 `EnumWindows`, `SetWindowPos`, pencere stilleri |
| Piksel doğruluğunda çerçeveleme | DWM `DWMWA_EXTENDED_FRAME_BOUNDS` |
| Büyütme | DWM thumbnail (`DwmRegisterThumbnail` + `rcDestination`) |
| Dürbün lensi, HUD katmanları *(planlı)* | Aynısı, `rcSource` bir bölgeye ayarlanmış hâli |
| Kavisli ekran, CRT, motion blur *(planlı)* | Windows.Graphics.Capture → D3D11 → HLSL |
| Özel ekran modları *(planlı)* | NVIDIA NvAPI |

Kod enjeksiyonu yok, kernel sürücüsü yok, başka bir sürecin belleğini okuma ya
da yazma yok. tlk-grid bir pencere yöneticisidir; trainer ya da hile değildir.

**Önemli uyarı:** bazı kernel seviyesi anti-cheat'ler, nasıl yapıldığından
bağımsız olarak, en üstte duran overlay pencerelerine ve oyun penceresini dışarıdan
oynatan programlara itiraz eder. Çevrimiçi kullanmadan önce oyunun kurallarına bak.

## Gereksinimler

- Windows 10 sürüm 1903 (build 18362) ya da üstü, 64-bit
- Desktop Window Manager açık bir GPU
- WebView2 çalışma zamanı — Windows 10/11'de hazır gelir
- Oyunun pencereli ya da kenarlıksız pencereli modda çalışıyor olması

## Derleme

```sh
cargo test --workspace          # yerleşim matematiği + canlı enumerasyon testi
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release           # target/release/tlk-grid.exe, tek dosya
```

## Yol haritası

| Faz | Kapsam | Durum |
| --- | --- | --- |
| 0 | Kabuk, tepsi, tek dosya derleme | bitti |
| 1 | Hedef keşfi, monitörler, yerleşim, konumlandırma, F8 | bitti |
| 2 | Kısayollar: düşük seviye hook, hold/toggle, master bypass, Alt-Tab nöbeti | bitti |
| 3 | Thumbnail zoom, window/stretch yöntemleri, tekerlekle faktör | bitti |
| 4 | Siyah bantlar, özel overlay'ler, nişangâh, dürbün lensi, HUD katmanları | sırada |
| 5 | CurveFX: kavisli ekran, CRT, dört motion blur modu | |
| 6 | SQUASH: NvAPI özel ekran modları | |
| 7 | Profiller, hazır ayar kitaplığı, dil anahtarı | kısmen (dil bitti) |
| 8 | Kılavuz, tanılama, yayın paketi | |

## Arayüz dili

Varsayılan İngilizce; Türkçe bir Windows'ta Türkçe açılır. Araç çubuğundaki
`EN`/`TR` düğmesi anında değiştirir ve seçim kalıcıdır.

## Teşekkür

Özellik kümesi ve etkileşim düzeni, bu fikri ilk kuran
[WinGrid](https://store.steampowered.com/app/4847000/WinGrid/) (Zhazira)
örnek alınarak tasarlandı. Oradan hiçbir kod, varlık ya da metin kullanılmadı;
bu depodaki her şey sıfırdan yazıldı.

MIT lisansı. Bkz. [LICENSE](LICENSE).
