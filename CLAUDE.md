# tlk-grid

Proje durumu, kararlar ve kalan işler `DEVAM.md` içinde. Burada sadece komutlar
ve kırmızı çizgiler var.

## Komutlar

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release          # target/release/tlk-grid.exe
```

## Kırmızı çizgiler

- **Enjeksiyon yok.** Kod enjeksiyonu, kernel sürücüsü, başka sürecin belleğini
  okuma/yazma — hiçbiri girmez. Depo bunu iddia ediyor, kod da tutmak zorunda.
- **Kod ve arayüz metni İngilizce.** Depo public ve ana dil İngilizce. Türkçe
  yalnız `ui/js/i18n.js` sözlüğünde ve `README.tr.md`'de.
- **`ui/js/i18n.js` iki sözlük tek parça.** `en` sözlüğüne anahtar eklenince `tr`
  de eklenir; eksik anahtar sessizce İngilizceye düşer ve ekran iki dilli kalır.
- **Yeni Win32 çağrısı test edilir.** `core/src/layout.rs` saf matematik ve test
  zorunlu; platform sarmalayıcıları en az bir canlı smoke test ister.
- **Kaynak uygulamadan kopya yok.** WinGrid'in kodu, görseli, metni alınmaz;
  yalnız özellik kümesi ve etkileşim düzeni örnek alınır.
