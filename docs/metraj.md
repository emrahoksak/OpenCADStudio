# Metraj modülü

Metraj, masaüstü OpenCADStudio içinde çalışır. `METRAJ` komutu soldaki mahal ağacını ve sağdaki **Metraj kalemleri** ile **Metraj atamaları** panellerini açar. Projeler aynı SQLite veritabanında proje kimliğiyle ayrılır; çizim geometrisi CAD dosyasında kalır.

Veritabanı, hem Debug hem Release sürümünde kullanıcı ayar dizinindeki **OpenCADStudio/metraj.sqlite** dosyasıdır. Windows'ta bu konum `%APPDATA%/OpenCADStudio/metraj.sqlite` olur. Son oluşturulan veya seçilen proje aynı veritabanında hatırlanır; uygulama açılışında proje, mahal/poz kayıtları ve demir verileri yeniden yüklenir. Eski veritabanında kayıtlı seçim yoksa en son oluşturulan proje açılır. Önceden yerleştirilmiş metraj panelleri de açılır. Erişim hataları boş bir veritabanına geçilerek gizlenmez. Eski `metraj-data/metraj.sqlite` dosyası korunur; içindeki projeler gerekirse SQLite içe aktarma ile alınabilir.

## Mahaller

Proje seçimi ve mahal filtresi ağacın üstündedir. **Proje ve mahal düzeni** başlığını açarak proje, iş grubu, kat ve mahal oluşturabilirsiniz. Ağaç sırası iş grubu, kat, mahal, bölüm ve atanmış kalemdir. Her seviye bağımsız açılıp kapanır. Mahal filtresi ad, kat ve iş grubunda arama yapar.

`MM`, PLINE araçlarıyla mahal sınırı çizer ve kapanan sınırdan otomatik mahal oluşturur. `NM`, seçilmiş tek bir kapalı 2B polyline veya rectangle sınırını mahale bağlar. Çizimin `INSUNITS` değeri tanımlı olmalıdır. Alanlar m², uzunluklar metreye çevrilir.

Ağaçtan seçilen mahal çizimde, çizimden seçilen mahal ağaçta seçilir. Atama panelindeki sabit **Hedef mahal** alanı proje, iş grubu, kat, mahal adı ve ölçülerini gösterir. Mahal adı geometrisine göre ölçeklenir; altındaki poz yazısının CAD metin yüksekliği **3 çizim birimidir**.

## Kalemler ve grup pozları

İki katalog aynı panelde **Kalemler** ve **Grup pozları** sekmelerindedir. Kalemler sekmesinin üstündeki filtre poz numarasına göre çalışır. Grup sekmesi grup adına veya gruptaki pozlara göre filtrelenir. Oluşturma formları daraltılabilir. Her grup bağımsız açılıp kapanır; grup üyeleri görülebilir, çıkarılabilir ve filtrelenmiş katalogdan yeni üyeler eklenebilir.

Kalem satırları ve grup tutamaçları mahal ağacındaki mahal veya bölüme ya da çizimdeki mahalin üzerine sürüklenebilir. Sürükleme, atama panelinde seçili bölüm, hesap ve **Tip** ayarını kullanır.

## Atamalar

1. Hedef mahali seçin.
2. **Bölüm**, **Hesap** ve **Tip** alanlarını ayarlayın. Tip **+ Ekle** veya **− Minha** olabilir; ayrı bir minha oluşturma formu yoktur.
3. **Kalem / grup seçimi** bölümünü poz, kalem veya grup adına göre filtreleyin. Satırlara tıklayarak bir veya daha fazla kalem/grup seçin. Tıklamak veritabanına atama yazmaz.
4. Alttaki **Uygula** düğmesi seçimi tek işlemde kaydeder. Aynı seçimde hem tek başına hem grup içinde bulunan kalem bir kez atanır. Bir kalem geçersizse tüm işlem geri alınır ve seçim korunur. Başarılı işlemden sonra seçim temizlenir.
5. **Mahaldeki atamalar** listesini ayrıca filtreleyebilirsiniz. Mevcut atamayı seçip bölümünü, hesap yöntemini ve tipini değiştirerek **Seçiliyi güncelle** ile kaydedebilir veya silebilirsiniz. Eski kayıtlardaki açıklamalı miktar düşümleri korunur ve bu listeden çıkarılabilir.

**Mahal ölçüsü**, m² kaleminde kapalı alanı, m kaleminde çevreyi, adet kaleminde 1'i kullanır. **Kapalı çevre × yükseklik** ve **Uzunluk × yükseklik** yalnızca m² kalemleri içindir. **Elle miktar**, doğrudan girilen miktarı kullanır. Sınırı geçersiz mahalde çizime dayalı miktarlar geçersiz sayılır.

Kaydedilmemiş çizimlerde de mahal oluşturulabilir. İlk kayıtta SQLite bağlantısı çizim dosyasına taşınır. Çizim kaydedilmeden uygulama kapanırsa sınır yeniden açılışta geçersiz sayılır.

**Kaydet ve kapat** işleminde de çizim bağlantısı sekme kapanmadan önce taşınır. DWG/DXF çizimini kaydetmek gerekir; SQLite, çizimin geometrisinin yerine geçmez. Kaydedilmemiş bir önceki oturumun mahalleri yeni boş çizime otomatik bağlanmaz; mahal, poz ve demir kayıtları veritabanında korunur.

Bu modül uygulama içine kayıtlıdır. OpenCADStudio güncellemeleri uygulanabilir; modülün çekirdek bağlantıları yeni sürüme birleştirilmelidir.

## İcmal ve Excel

Atama panelinin altındaki **İcmal** düğmesi **Metraj İcmali** dock panelini açar. Aynı proje içindeki aynı poz numarası, farklı kalem adları veya mahallerde kullanılmış olsa da tek satırda toplanır. Panel Ekle, Minha ve Net miktarı gösterir. Poz filtresi panel listesini daraltır; **Excel'e aktar (.xlsx)** projenin tüm pozlarını dışarı aktarır. Poz numaraları Excel'de metin olarak tutulur; baştaki sıfırlar korunur.

Poz numarası boş atamalar toplam dışında tutulur ve sayısı gösterilir. Geçersiz çizim kaynağı bulunan atamalar toplamda miktar üretmez ve uyarı sayısı görünür. Aynı poz farklı birimlerle kullanılmışsa toplam tek satırda kalır; panel ve Excel'de karma birim uyarısı gösterilir.

Mahal adı ile poz listesi iki ayrı çizim metnidir. Pozlar adın altında, 3 çizim birimi yüksekliğinde yazılır. İki metin de aynı mahalle bağlıdır. Atama kataloğundaki **⠿** tutamaçları, kalemleri ve grupları mahal ağacına, çizime veya paneldeki **Hedef mahal** alanına sürüklemek için kullanılır. Satırın kendisine tıklamak seçim yapmaya devam eder.

## Çoklu mahal ataması

Çizimden birden fazla mahal seçildiğinde CAD seçimi korunur. Aynı mahalin sınırı, adı, poz yazısı ve taraması birlikte seçilmişse mahal tek hedef sayılır. Atama paneli hedef mahal sayısını gösterir; **Uygula** seçilen kalemleri/grupları tüm hedeflere tek işlemde atar. Mahal ölçüsü ve çevre × yükseklik hesapları her mahalin kendi geometrisini kullanır. Elle miktar veya özel uzunluk × yükseklik seçildiğinde girilen değer her hedef mahale ayrı uygulanır.

Birden fazla hedefte hata oluşursa hiçbir hedefe kısmi kayıt bırakılmaz. Seçili mahallerden birine veya panelin hedef alanına sürüklenen kalem/grup da aynı toplu atamayı yapar. Seçim dışındaki başka bir mahale bırakma yalnızca o mahali hedefler. Atanmış kalem listesi, çoklu seçimde mahal adlarıyla gruplanır. Mevcut bir atamayı düzenlemek için satırı seçmek tek mahal odağına geçer.

**Poz oluştur** formu hem kalemler hem atamalar panelinde aynı açılır bölüm olarak bulunur. İki panel aynı katalog, alan değerleri ve ölçü birimini kullanır. Poz oluşturmak atama yapmaz; atama için kalemi seçip **Uygula** düğmesine basın.

## Mahal komutları ve silme

Metraj proje seçicisinin yanındaki **Sil**, seçili projeyi onay penceresiyle siler. Projenin iş grupları, katları, mahalleri, poz tanımları ve grupları, atamaları, minha ve demir kayıtları birlikte kaldırılır. Diğer projeler korunur. Çizim dosyaları ve çizim nesneleri silinmez. İşlem geri alınamaz; veritabanı hatasında kısmi silme yapılmaz.

**Tüm mahallerin dolgusunu göster**, açık çizimdeki bütün mahal sınırlarının içini seçim kutusuna benzer saydam renkle gösterir. Tek seçenek bütün mahaller için geçerlidir ve uygulama yeniden açıldığında korunur. Dolgu yalnızca ekranda gösterilir; CAD nesnesi oluşturmaz ve çizimi değiştirmez. Gizli katmanlardaki mahaller gösterilmez.

**Dolgu ayarları** açılır bölümünde renk kutusuna tıklayarak uygulamanın renk paletinden ACI veya RGB rengi seçebilirsiniz. **Şeffaflık**, %0'da opak, %100'de tamamen görünmez dolgu üretir. Değişiklikler anında bütün mahallere uygulanır ve ortak SQLite veritabanında saklanır. Dolgu kapalıyken de renk ve şeffaflık ayarlanabilir; CAD seçim kutusunun ayarları etkilenmez.

`NM`, seçili kapalı polyline/rectangle için etkin katta otomatik ve benzersiz bir adla yeni mahal oluşturur: **Mahal 1**, **Mahal 2** gibi. Başka bir mahal seçili olsa da onun sınırını değiştirmez. Aynı sınır ikinci kez dönüştürülemez. Ön seçim yoksa komut nesne seçimi ister.

`MM` ve `NM`, komut bittikten veya iptal edildikten sonra boş komut satırında **Enter** veya **Space** ile tekrar çalıştırılabilir.
`NM` tekrarlandığında seçili sınır zaten bir mahale bağlıysa yeni nesne seçimine geçer; aynı mahal ikinci kez oluşturulmaz.

Mahalin sınırı **Delete**, **ERASE** veya **CUTCLIP** ile kaldırılmadan önce uygulama onay ister. **İptal**, çizimi ve kayıtları korur. **Mahalleri sil**, seçili sınırlarla birlikte ilgili mahal kayıtlarını, poz atamalarını, minha kayıtlarını ve çizimdeki mahal yazılarını/taramalarını siler. Katalogdaki poz tanımları korunur. Veritabanına yazma başarısızsa çizim silinmez. Kilitli katmandaki sınır silinmez.

Mahal ve metraj kayıtlarının silinmesi kalıcıdır; çizimin geri alma işlemi yalnızca çizim elemanlarını geri getirir.

## Demir metrajı

**Metraj → Demir → Demir Metrajı** veya `DEMIR`, diğer metraj panelleriyle aynı dock, yazı ve alan stillerini kullanan paneli açar. Proje seçmek yeterlidir; iş grubu, kat ve mahal oluşturmak gerekmez. Demir verileri aynı SQLite veritabanında proje kimliğiyle ayrılmış tablolarda saklanır. Eski mahal/poz verileri korunur.

Panelde **Çizimden demir alımı**, **Manuel demir girişi**, **Demir kayıtları** ve **Demir icmali** başlıkları bağımsız açılır/kapanır. Alım modu açılır listeden seçilir. **+ Ekle / − Minha** tipi hem çizim alımı hem manuel giriş için geçerlidir.

| Komut | Alım modu | Kural |
| --- | --- | --- |
| `DM1` | Mod1 · Tek metin | `12Ø16 L=250` tek TEXT/MTEXT nesnesindedir. |
| `DM2` | Mod2 · En yakın | `12Ø16` en yakın kullanılmamış `L=250` yazısıyla 3B mesafeye göre eşleşir. |
| `DMY` | Yatay | Sağdaki en yakın boy; Y farkı tolerans içinde olmalıdır. |
| `DMD` | Düşey | Üstteki en yakın boy; X farkı tolerans içinde olmalıdır. |
| `DMA` | Adetsiz | Tam bir `Ø16`, bir `L=250`; ayrı `12 Adet` yazıları varsa toplanır, yoksa adet 1'dir. |

**Demir metrajı al**, seçili modun komutunu çalıştırır. Ön seçim varsa bu seçim okunur; yoksa çizimde nesne seçimi başlar ve **Enter** ile tamamlanır. Başarılı alımdan sonra çizim seçimi temizlenir. Boş komut satırında **Enter** veya **Space**, son çalıştırılan mod komutunu yeniden başlatır. İptal edilen seçim kayıt oluşturmaz.

Boylar `L=` metninden **cm**, çaplar **mm** olarak okunur; donatı çizgisinin geometrisinden boy/adet türetilmez. Yatay/düşey tolerans **çizim birimidir**; varsayılanı 0,01'dir. Bu yönler yazı açısına değil çizimin X/Y koordinatlarına bağlıdır. Mod2'de maksimum mesafe sınırı yoktur. Her boy yazısı bir işlemde en fazla bir kez eşleştirilir; eşleşmeyen adet/çap sayısı bildirilir.

`3x2`, `3X2`, `3 × 2`, `3*2` ve zincir çarpımlar desteklenir. **x adet** ayrıca uygulanır; pozitif tam sayı olmalıdır. Manuel giriş kendi adet alanını kullanır, x adet ayrıca uygulanmaz. Sıfır, negatif, taşan adetler ve geçersiz boylar reddedilir. Nokta/virgül ondalıkları; TEXT çap kontrol kodları ve MTEXT düz metin içeriği okunur. Çap sembolleri alanına mevcut karakterlere ek olarak kullanılan sembol yazılabilir. Mod, semboller, çarpan ve tolerans alım başlatıldığında proje ayarlarına kaydedilir.

Hesap: **toplam boy = adet × boy(cm) / 100**, **birim ağırlık = çap² / 162 kg/m**, **ağırlık = toplam boy × birim ağırlık**. Minha satırında adet/çap pozitif, toplam boy ve ağırlık negatiftir. Çapa göre icmal Ekle, Minha, net metre, kg ve ton değerlerini gösterir. Satır seçmek kayıt eklemez; düzenleme alanlarını doldurur. **Yeni kayıt ekle**, **Güncelle** ve **Sil** açık işlemlerdir. Liste çap, kaynak metin veya handle ile filtrelenebilir; icmal ve Excel tüm proje kayıtlarını içerir.

Demir panelinin üstündeki **Ayarlar** başlığı bağımsız açılır/kapanır. Ayarlar proje bazında saklanır; **Ayarları kaydet** veya alım başlatma ile kaydedilir. Eski projelerin kayıtları ve önceki alım ayarları korunur.

- **Eşleşen metinlerin rengini değiştir**: etkinse yalnızca başarıyla eşleşen yazılar HESAPLANDI katmanına taşınarak boyanır. Kapalıysa kaynak yazıların katmanı ve rengi korunur.
- **Tek renk**: seçilen ACI rengi bütün eşleşmelerde kullanılır.
- **Karma renk**: her eşleşme grubu ayrı renk alır; aynı gruptaki adet/çap, boy ve ayrı adet etiketleri aynı renge boyanır. Örneğin `8Ø20/20` ile `L=200` kırmızı, `7Ø12/12` ile `L=300` sarı olabilir. Renk paleti proje kayıt sayısından devam eder; aynı RGB rengini veren farklı ACI indeksleri yinelenmez. Palet tükenince renkler yeniden kullanılır.
- **Her eşleşme için bounding box çiz**: renk modundan bağımsızdır. Etkinse her demir kaydının kaynak yazılarını kapsayan ayrı bir kapalı polyline çizilir. Kutunun rengi grubun rengiyle aynıdır. Metin boyama kapalıyken yalnız kutu da oluşturulabilir.

Bounding box, yazıların çizimdeki gerçek harf çizgileri ve dolgularından hesaplanır. MTEXT'in boş düzenleme genişliği, seçimin tamamı veya ilgisiz yazılar ölçüye katılmaz. Ek pay **0**'dır. Kutunun yönü minimum alanı verecek şekilde hesaplanır; döndürülmüş yazıda gereksiz büyük bir eksene paralel kutu kullanılmaz. Font, hizalama, satırlar ve geçerli anotasyon ölçeği çizim motoruyla aynı yerleşimden alınır. Bir eşleşmenin yazıları ortak düzlemde değilse kutu çizilmeden ve metraj kaydı oluşturulmadan işlem durdurulur.

Renk değişikliği biçimli MTEXT'in renk kontrollerini de günceller; font ve yerleşim korunur. Eşleşmeyen yazılar boyanmaz ve kutu hesabına katılmaz. Boyanacak kaynak veya değiştirilecek HESAPLANDI katmanı kilitliyse işlem kayıt oluşturmadan reddedilir. Veritabanı yazma hatasında çizim değiştirilmez. İki işaretleme seçeneği de kapalıysa yalnız metraj kaydı oluşur; çizimde renk veya kutu değişikliği yapılmaz. Yeni ayarlar sonraki alımlara uygulanır; eski kayıtların işaretlerini otomatik değiştirmez.

Kaynak metin, özgün katman, çizim ve handle bilgileri saklanır. HESAPLANDI bir tekrar okuma kilidi değildir; aynı yazıları yeniden seçmek yeni kayıt oluşturur. Kaydı silmek özgün katmanı veya rengi geri yüklemez ve kutuyu silmez; çizim geri alma işlemi metraj kayıtlarını geri almaz.

**Excel'e aktar (.xlsx)**, tüm demir satırlarını ve çapa göre icmali sayısal hücrelerle aktarır. Proje yeniden açıldığında demir kayıtları ve ayarları korunur. OpenCADStudio SQLite projesini içe aktarma demir verilerini de taşır; AutoCAD eklentisinin veritabanını doğrudan içe aktarma eklenmemiştir.
