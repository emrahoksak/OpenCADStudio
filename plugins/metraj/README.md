# OpenCADStudio Metraj

OpenCADStudio'nun yerel eklentisi, Metraj şeridindeki **Metraj Paneli** veya `METRAJ` komutuyla açılır. Panelden proje, grup ve metraj kalemi oluşturun. **Aktif grup** listesinden çalışacağınız projeyi ve grubu seçin. Her grubun metraj ve demir kayıtları ile icmali ayrı tutulur.

Çizimde önce nesneleri seçin, ardından panelde **Seçimden Metraj Al** veya **Seçimden Demir Al** düğmesine basın. Metraj için LINE/LWPOLYLINE uzunluğu veya kapalı LWPOLYLINE/HATCH alanı seçilir; çizim birimi ölçüm sırasında belirtilir. m² kaleminde uzunluk × yükseklik, m³ kaleminde alan × yükseklik kullanılabilir. Manuel kayıt ve minha seçenekleri de vardır.

Demir ayarlarında Mod1, Mod2, Mod2++, yatay, düşey ve adetsiz eşleştirme bulunur. Çap sembollerini, toleransı, tek veya karma rengi ve çerçeveyi ayarlayabilirsiniz. Grup icmali ve detaylar `.xlsx` olarak aktarılabilir. Eşleşen metinler ve ölçülen çizgiler `HESAPLANDI` katmanına taşınır. Çizimdeki işaretleri CAD geri alma komutuyla, metraj satırlarını ise panelden yönetebilirsiniz.

Windows paketindeki `OpenCADStudio.exe`, aynı klasördeki `plugins/metraj.opencad` eklentisini açılışta bulur. Eklentiyi ayrı kurarken `plugin.toml` ve DLL dosyalarını `%APPDATA%\OpenCADStudio\plugins\metraj.opencad` dizinine koyun. Uygulamanın API, Rust derleyicisi ve çizim kitaplığı sürümleri `plugin.toml` ile eşleşmelidir.

**SQLite Projelerini Aktar** düğmesinden eski Metraj veritabanını seçebilirsiniz. Eklenti dosyayı salt okunur açar ve projeleri bir kez aktarır. Grup bilgisi bulunmayan eski kayıtlar **Grupsuz eski kayıtlar** grubunda yer alır. Veriler `%APPDATA%\OpenCADStudio\metraj\projects.json` içinde tutulur.

Çerçeve, OpenCADStudio'nun sunduğu metin sınırlarını kapsayan eksenlere paralel dikdörtgendir. Bu sınırlar yazı tipine ve dönüşe göre yaklaşık olabilir. Çizim güncellemesi başarısız olursa kaydedilen metraj korunur ve komut satırında hata gösterilir.

Kaynakları derlemek için depo kökünden `cargo +1.100.0 build --release --locked --bin OpenCADStudio` ve `cargo +1.100.0 build --release --locked --manifest-path plugins/metraj/Cargo.toml` çalıştırın.
