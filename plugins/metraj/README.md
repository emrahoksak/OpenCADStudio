# OpenCADStudio Metraj

Bir çizim açıp şeritteki **Metraj** sekmesine geçin. Eklentinin üç paneli etkin proje ve grubu paylaşır. Her grubun kalemleri, ölçümleri, demir kayıtları ve icmali ayrı tutulur.

## Metraj Ayarları

**Metraj Ayarları** (`METRAJ_AYARLAR`) panelinden proje ve grup oluşturun. Yeni proje seçilir ve eklenen grup etkinleştirilir. Mevcut bir projeyle çalışmak için listeden projeyi seçip **Projeyi Seç** düğmesine basın. Ardından çalışma grubunu seçip **Grubu Etkinleştir** düğmesine basın.

Demir ayarları seçili projeye kaydedilir. Mod1, Mod2, Mod2++, yatay, düşey veya adetsiz eşleştirme modunu, çap sembollerini ve toleransı belirleyin. Tek renk için bir hazır renk veya `#RRGGBB` değerini seçin. Karma renk kullanıldığında her eşleşme ayrı renkle işaretlenir. Çerçeve seçeneği, her eşleşmenin metin sınırlarını kapsayan eksenlere paralel dikdörtgeni çizer.

**SQLite'tan Aktar** düğmesi eski Metraj veritabanındaki projeleri salt okunur dosyadan aktarır. Grup bilgisi bulunmayan eski kayıtlar **Grupsuz eski kayıtlar** grubuna alınır.

## Metraj

**Metraj** (`METRAJ`) panelinde **Yeni Metraj Kalemi** düğmesiyle ad, poz no ve birim girin. Liste her kalemin net miktarını gösterir. Kaleme çift tıklayın veya yanındaki **Aç** düğmesine basın.

Kalem ekranında çizim birimini ve ölçüm yöntemini seçin. Çizimdeki nesneleri seçip **Seçimi Kaleme Ekle** düğmesine basın. Uzunluk için LINE veya LWPOLYLINE, alan için kapalı LWPOLYLINE veya HATCH kullanılabilir. m² kaleminde uzunluk × yükseklik, m³ kaleminde alan × yükseklik kullanılabilir. **Minha** düşüm kaydeder. Manuel düşüm için eksi miktar girin. Ölçüm kayıtları ve net miktar kalem ekranında görünür.

Kalem listesindeki **Excel'e Aktar** düğmesi etkin grubun icmalini ve detaylarını `.xlsx` olarak kaydeder.

## Demir Metrajı

**Demir Metrajı** (`METRAJ_DEMIR_PANEL`) panelinde çizimdeki TEXT ve MTEXT yazılarını seçip **Seçili Yazılardan Demir Al** düğmesine basın. Adet çarpanını ve metraj veya minha işlemini panelden belirleyin. Manuel giriş için adet, çap ve birim boy girin. Panel, grubun kayıtlarını, çap bazında net boy ve ağırlığını, toplam kg ve ton değerlerini gösterir.

Eşleşen metinler ve ölçülen nesneler `HESAPLANDI` katmanına taşınır. Çizim işaretleri CAD geri alma komutuyla, ölçüm satırları paneldeki **Kaydı Sil** işlemiyle yönetilir. Çerçeve ölçüleri yazı tipi ve dönüşe bağlı olarak yaklaşık olabilir.

## Kurulum ve derleme

Windows sürümündeki `OpenCADStudio.exe`, aynı klasördeki `plugins/metraj.opencad` eklentisini açılışta yükler. Eklentiyi ayrı kurarken paket içindeki `metraj.opencad` klasörünü `%APPDATA%\OpenCADStudio\plugins` dizinine yerleştirin. Uygulamanın API, Rust derleyicisi ve çizim kitaplığı sürümleri `plugin.toml` ile eşleşmelidir. Metraj verileri `%APPDATA%\OpenCADStudio\metraj\projects.json` dosyasında tutulur.

Depo kökünden `cargo +1.100.0 build --release --locked --bin OpenCADStudio` ve `cargo +1.100.0 build --release --locked --manifest-path plugins/metraj/Cargo.toml` komutlarıyla derleyin.
