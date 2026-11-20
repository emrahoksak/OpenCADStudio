use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
pub struct Store {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub imported_projects: std::collections::BTreeSet<String>,
    #[serde(skip)]
    original: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub groups: Vec<Group>,
    #[serde(default)]
    pub settings: RebarSettings,
}

#[derive(Serialize, Deserialize)]
pub struct RebarSettings {
    pub mode: usize,
    pub symbols: String,
    pub mixed: bool,
    pub color: [u8; 3],
    pub bounding_box: bool,
    pub tolerance: f64,
}
impl Default for RebarSettings {
    fn default() -> Self {
        Self {
            mode: 0,
            symbols: "ΦØøϕ".into(),
            mixed: false,
            color: [0, 200, 0],
            bounding_box: false,
            tolerance: 0.01,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub rebar: Vec<RebarRow>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub poz: String,
    pub unit: String,
    #[serde(default)]
    pub display_unit: Option<String>,
    pub records: Vec<Record>,
}

impl Item {
    pub fn unit_label(&self) -> &str {
        self.display_unit.as_deref().unwrap_or(&self.unit)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub method: String,
    pub quantity: f64,
    pub source: String,
    #[serde(default)]
    pub handles: Vec<u64>,
    #[serde(default)]
    pub original_layers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RebarRow {
    pub quantity: u32,
    pub diameter: u32,
    pub length_cm: f64,
    pub weight_kg: f64,
    pub source: String,
    #[serde(default)]
    pub handles: Vec<u64>,
    #[serde(default)]
    pub original_layers: Vec<String>,
}

fn path() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .or_else(|| {
            std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share").into_os_string())
        })
        .ok_or("Uygulama veri klasörü bulunamadı")?;
    Ok(PathBuf::from(base)
        .join("OpenCADStudio")
        .join("metraj")
        .join("projects.json"))
}

fn read_existing(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Metraj verileri okunamadı: {error}")),
    }
}

impl Store {
    pub fn revision() -> Result<Option<Vec<u8>>, String> {
        read_existing(&path()?)
    }

    pub fn load() -> Result<Self, String> {
        let Some(bytes) = Self::revision()? else {
            return Ok(Self::default());
        };
        let mut store: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Proje dosyası okunamadı: {e}"))?;
        store.original = Some(bytes);
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path()?;
        fs::create_dir_all(path.parent().ok_or("Veri klasörü bulunamadı")?)
            .map_err(|e| e.to_string())?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .map_err(|e| e.to_string())?;
        lock.lock().map_err(|e| e.to_string())?;
        if read_existing(&path)? != self.original {
            return Err("Metraj verileri değişti. İşlemi yeniden başlatın.".into());
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut temp =
            tempfile::NamedTempFile::new_in(path.parent().ok_or("Veri klasörü bulunamadı")?)
                .map_err(|e| e.to_string())?;
        temp.write_all(&bytes)
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        if path.exists() {
            fs::copy(&path, path.with_extension("json.bak")).map_err(|e| e.to_string())?;
        }
        temp.persist(&path).map_err(|e| e.error.to_string())?;
        Ok(())
    }
}
