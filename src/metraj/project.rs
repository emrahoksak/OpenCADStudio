use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS project (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    default_height_m REAL NOT NULL DEFAULT 2.80 CHECK (default_height_m >= 0)
);
CREATE TABLE IF NOT EXISTS work_group (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL DEFAULT 1 REFERENCES project(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0)
);
CREATE TABLE IF NOT EXISTS floor (
    id INTEGER PRIMARY KEY,
    group_id INTEGER NOT NULL REFERENCES work_group(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0)
);
CREATE TABLE IF NOT EXISTS room (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL DEFAULT 1 REFERENCES project(id) ON DELETE CASCADE,
    floor_id INTEGER NOT NULL REFERENCES floor(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    drawing_path TEXT,
    entity_handle TEXT,
    label_handle TEXT,
    hatch_handle TEXT,
    area_m2 REAL NOT NULL DEFAULT 0 CHECK (area_m2 >= 0),
    perimeter_m REAL NOT NULL DEFAULT 0 CHECK (perimeter_m >= 0),
    boundary_valid INTEGER NOT NULL DEFAULT 0 CHECK (boundary_valid IN (0, 1)),
    CHECK ((drawing_path IS NULL) = (entity_handle IS NULL)),
    UNIQUE (project_id, drawing_path, entity_handle)
);
CREATE TABLE IF NOT EXISTS item (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL DEFAULT 1 REFERENCES project(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    unit TEXT NOT NULL CHECK (unit IN ('m2', 'm', 'adet')),
    poz TEXT NOT NULL DEFAULT '',
    short_description TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS quantity_group (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL DEFAULT 1 REFERENCES project(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    UNIQUE (project_id, name)
);
CREATE TABLE IF NOT EXISTS quantity_group_item (
    group_id INTEGER NOT NULL REFERENCES quantity_group(id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL REFERENCES item(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, item_id)
);
CREATE TABLE IF NOT EXISTS assignment (
    id INTEGER PRIMARY KEY,
    room_id INTEGER NOT NULL REFERENCES room(id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL REFERENCES item(id),
    section TEXT NOT NULL CHECK (section IN ('wall', 'floor', 'ceiling', 'other')),
    mode TEXT NOT NULL CHECK (mode IN ('room', 'perimeter_height', 'manual')),
    height_m REAL,
    length_m REAL CHECK (length_m IS NULL OR length_m >= 0),
    manual_quantity REAL,
    is_deduction INTEGER NOT NULL DEFAULT 0 CHECK (is_deduction IN (0, 1)),
    CHECK (height_m IS NULL OR height_m >= 0),
    CHECK (manual_quantity IS NULL OR manual_quantity >= 0)
);
CREATE TABLE IF NOT EXISTS deduction (
    id INTEGER PRIMARY KEY,
    assignment_id INTEGER NOT NULL REFERENCES assignment(id) ON DELETE CASCADE,
    description TEXT NOT NULL DEFAULT '',
    quantity REAL NOT NULL CHECK (quantity > 0)
);
CREATE INDEX IF NOT EXISTS floor_group_idx ON floor(group_id);
CREATE INDEX IF NOT EXISTS room_floor_idx ON room(floor_id);
CREATE INDEX IF NOT EXISTS assignment_room_idx ON assignment(room_id);
CREATE INDEX IF NOT EXISTS deduction_assignment_idx ON deduction(assignment_id);
CREATE TABLE IF NOT EXISTS rebar_settings (
    project_id INTEGER PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
    mode INTEGER NOT NULL CHECK(mode BETWEEN 0 AND 4),
    symbols TEXT NOT NULL,
    multiplier INTEGER NOT NULL CHECK(multiplier > 0),
    tolerance REAL NOT NULL CHECK(tolerance >= 0),
    marking_mode INTEGER NOT NULL DEFAULT 0 CHECK(marking_mode BETWEEN 0 AND 1),
    marking_color INTEGER NOT NULL DEFAULT 3 CHECK(marking_color BETWEEN 1 AND 255),
    recolor_text INTEGER NOT NULL DEFAULT 1 CHECK(recolor_text IN (0,1)),
    draw_boxes INTEGER NOT NULL DEFAULT 0 CHECK(draw_boxes IN (0,1))
);
CREATE TABLE IF NOT EXISTS rebar_measurement (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    quantity INTEGER NOT NULL CHECK(quantity > 0),
    diameter_mm INTEGER NOT NULL CHECK(diameter_mm BETWEEN 1 AND 100),
    length_cm REAL NOT NULL CHECK(length_cm > 0),
    deduction INTEGER NOT NULL CHECK(deduction IN (0,1)),
    source TEXT NOT NULL,
    drawing TEXT NOT NULL,
    handles TEXT NOT NULL,
    layer TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS rebar_project_idx ON rebar_measurement(project_id);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    SquareMeter,
    Meter,
    Each,
}

impl Unit {
    fn as_str(self) -> &'static str {
        match self {
            Self::SquareMeter => "m2",
            Self::Meter => "m",
            Self::Each => "adet",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "m2" => Ok(Self::SquareMeter),
            "m" => Ok(Self::Meter),
            "adet" => Ok(Self::Each),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomSection {
    Wall,
    Floor,
    Ceiling,
    Other,
}

impl RoomSection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Wall => "wall",
            Self::Floor => "floor",
            Self::Ceiling => "ceiling",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantityMode {
    Room,
    PerimeterTimesHeight(f64),
    LengthTimesHeight { length_m: f64, height_m: f64 },
    Manual(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub id: i64,
    pub section: RoomSection,
    pub poz: String,
    pub short_description: String,
    pub item_name: String,
    pub unit: Unit,
    pub mode: QuantityMode,
    pub gross: f64,
    pub deductions: f64,
    pub net: f64,
    pub source_valid: bool,
    pub is_deduction: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Deduction {
    pub id: i64,
    pub description: String,
    pub quantity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub area_m2: f64,
    pub perimeter_m: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoomInfo {
    pub measurement: Measurement,
    pub bound: bool,
    pub boundary_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedRow {
    pub id: i64,
    pub name: String,
}

impl std::fmt::Display for NamedRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub id: i64,
    pub name: String,
    pub unit: Unit,
    pub poz: String,
    pub short_description: String,
}

pub struct Project {
    pub(super) db: Connection,
    pub(super) id: i64,
}

impl Project {
    fn ensure_project_height(db: &Connection) -> rusqlite::Result<()> {
        let rebar_columns = {
            let mut columns = db.prepare("PRAGMA table_info(rebar_settings)")?;
            let names = columns
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            names
        };
        if !rebar_columns.iter().any(|name| name == "marking_mode") {
            db.execute_batch("ALTER TABLE rebar_settings ADD COLUMN marking_mode INTEGER NOT NULL DEFAULT 0 CHECK(marking_mode BETWEEN 0 AND 1)")?;
        }
        if !rebar_columns.iter().any(|name| name == "marking_color") {
            db.execute_batch("ALTER TABLE rebar_settings ADD COLUMN marking_color INTEGER NOT NULL DEFAULT 3 CHECK(marking_color BETWEEN 1 AND 255)")?;
        }
        if !rebar_columns.iter().any(|name| name == "recolor_text") {
            db.execute_batch("ALTER TABLE rebar_settings ADD COLUMN recolor_text INTEGER NOT NULL DEFAULT 1 CHECK(recolor_text IN (0,1))")?;
        }
        if !rebar_columns.iter().any(|name| name == "draw_boxes") {
            db.execute_batch("ALTER TABLE rebar_settings ADD COLUMN draw_boxes INTEGER NOT NULL DEFAULT 0 CHECK(draw_boxes IN (0,1))")?;
        }
        let mut columns = db.prepare("PRAGMA table_info(project)")?;
        let names = columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if !names.iter().any(|name| name == "default_height_m") {
            db.execute_batch("ALTER TABLE project ADD COLUMN default_height_m REAL NOT NULL DEFAULT 2.80 CHECK (default_height_m >= 0)")?;
        }
        let mut room_columns = db.prepare("PRAGMA table_info(room)")?;
        let room_names = room_columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if !room_names.iter().any(|name| name == "hatch_handle") {
            db.execute_batch("ALTER TABLE room ADD COLUMN hatch_handle TEXT")?;
        }
        if !room_names.iter().any(|name| name == "poz_label_handle") {
            db.execute_batch("ALTER TABLE room ADD COLUMN poz_label_handle TEXT")?;
        }
        Ok(())
    }
    pub fn open(path: impl AsRef<Path>, name_if_new: &str) -> rusqlite::Result<Self> {
        if name_if_new.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let existed = path.as_ref().exists();
        let db = Connection::open(path)?;
        if existed {
            let is_project: i64 = db.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'project'",
                [],
                |row| row.get(0),
            )?;
            if is_project == 0 {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }
        db.execute_batch(SCHEMA)?;
        Self::ensure_project_height(&db)?;
        let item_column_names = {
            let mut statement = db.prepare("PRAGMA table_info(item)")?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };
        if !item_column_names.iter().any(|column| column == "poz") {
            db.execute_batch("ALTER TABLE item ADD COLUMN poz TEXT NOT NULL DEFAULT ''")?;
        }
        if !item_column_names
            .iter()
            .any(|column| column == "short_description")
        {
            db.execute_batch(
                "ALTER TABLE item ADD COLUMN short_description TEXT NOT NULL DEFAULT ''",
            )?;
        }
        let mut columns = db.prepare("PRAGMA table_info(room)")?;
        let column_names = columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let has_boundary_valid = column_names.iter().any(|column| column == "boundary_valid");
        drop(columns);
        if !has_boundary_valid {
            db.execute_batch("BEGIN; ALTER TABLE room ADD COLUMN boundary_valid INTEGER NOT NULL DEFAULT 0 CHECK (boundary_valid IN (0, 1)); UPDATE room SET boundary_valid = 1 WHERE drawing_path IS NOT NULL; COMMIT")?;
        }
        if !column_names.iter().any(|column| column == "label_handle") {
            db.execute_batch("ALTER TABLE room ADD COLUMN label_handle TEXT")?;
        }
        if !column_names.iter().any(|column| column == "hatch_handle") {
            db.execute_batch("ALTER TABLE room ADD COLUMN hatch_handle TEXT")?;
        }
        let mut columns = db.prepare("PRAGMA table_info(assignment)")?;
        let assignment_columns = columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(columns);
        if !assignment_columns.iter().any(|column| column == "length_m") {
            db.execute_batch("ALTER TABLE assignment ADD COLUMN length_m REAL CHECK (length_m IS NULL OR length_m >= 0)")?;
        }
        if !assignment_columns
            .iter()
            .any(|column| column == "is_deduction")
        {
            db.execute_batch("ALTER TABLE assignment ADD COLUMN is_deduction INTEGER NOT NULL DEFAULT 0 CHECK (is_deduction IN (0, 1))")?;
        }
        db.execute(
            "UPDATE room SET boundary_valid = 0 WHERE drawing_path LIKE 'ocs-unsaved-tab:%'",
            [],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO project(id, name) VALUES (1, ?1)",
            [name_if_new.trim()],
        )?;
        for table in ["work_group", "room", "item", "quantity_group"] {
            let mut columns = db.prepare(&format!("PRAGMA table_info({table})"))?;
            let names = columns
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if !names.iter().any(|name| name == "project_id") {
                db.execute_batch(&format!(
                    "ALTER TABLE {table} ADD COLUMN project_id INTEGER NOT NULL DEFAULT 1"
                ))?;
            }
        }
        Ok(Self { db, id: 1 })
    }

    pub fn list_shared(path: impl AsRef<Path>) -> rusqlite::Result<Vec<NamedRow>> {
        let db = Connection::open(path)?;
        db.execute_batch(SCHEMA)?;
        Self::ensure_project_height(&db)?;
        let mut stmt = db.prepare("SELECT id, name FROM project ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(NamedRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect();
        rows
    }

    pub fn create_shared(path: impl AsRef<Path>, name: &str) -> rusqlite::Result<Self> {
        if name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let db = Connection::open(path)?;
        db.execute_batch(SCHEMA)?;
        Self::ensure_project_height(&db)?;
        db.execute("INSERT INTO project(name) VALUES (?1)", [name.trim()])?;
        let id = db.last_insert_rowid();
        Ok(Self { db, id })
    }

    pub fn open_shared(path: impl AsRef<Path>, id: i64) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch(SCHEMA)?;
        Self::ensure_project_height(&db)?;
        db.query_row("SELECT id FROM project WHERE id = ?1", [id], |row| {
            row.get::<_, i64>(0)
        })?;
        db.execute("UPDATE room SET boundary_valid = 0 WHERE project_id = ?1 AND drawing_path LIKE 'ocs-unsaved-tab:%'", [id])?;
        Ok(Self { db, id })
    }

    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn summary_revision(&self) -> u64 {
        self.db.total_changes()
    }

    /// Copy a standalone metraj file into the shared database. The source file is read only.
    pub fn import_shared(
        shared: impl AsRef<Path>,
        source: impl AsRef<Path>,
    ) -> rusqlite::Result<i64> {
        let source_db = Connection::open_with_flags(
            source.as_ref(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let name: String =
            source_db.query_row("SELECT name FROM project WHERE id = 1", [], |row| {
                row.get(0)
            })?;
        drop(source_db);
        let mut db = Connection::open(shared)?;
        db.execute_batch(SCHEMA)?;
        Self::ensure_project_height(&db)?;
        db.execute(
            "ATTACH DATABASE ?1 AS legacy",
            [source.as_ref().to_string_lossy().as_ref()],
        )?;
        let result = (|| {
            let tx = db.transaction()?;
            tx.execute("INSERT INTO project(name) VALUES (?1)", [name])?;
            let project_id = tx.last_insert_rowid();
            let offset = |table: &str| -> rusqlite::Result<i64> {
                tx.query_row(
                    &format!("SELECT COALESCE(MAX(id), 0) FROM {table}"),
                    [],
                    |row| row.get(0),
                )
            };
            let g = offset("work_group")?;
            let f = offset("floor")?;
            let r = offset("room")?;
            let i = offset("item")?;
            let q = offset("quantity_group")?;
            let a = offset("assignment")?;
            let d = offset("deduction")?;
            tx.execute("INSERT INTO work_group(id, project_id, name) SELECT id + ?1, ?2, name FROM legacy.work_group", params![g, project_id])?;
            tx.execute("INSERT INTO floor(id, group_id, name) SELECT id + ?1, group_id + ?2, name FROM legacy.floor", params![f, g])?;
            tx.execute("INSERT INTO room(id, project_id, floor_id, name, drawing_path, entity_handle, label_handle, area_m2, perimeter_m, boundary_valid) SELECT id + ?1, ?2, floor_id + ?3, name, drawing_path, entity_handle, label_handle, area_m2, perimeter_m, boundary_valid FROM legacy.room", params![r, project_id, f])?;
            tx.execute("INSERT INTO item(id, project_id, name, unit, poz, short_description) SELECT id + ?1, ?2, name, unit, poz, short_description FROM legacy.item", params![i, project_id])?;
            tx.execute("INSERT INTO quantity_group(id, project_id, name) SELECT id + ?1, ?2, name FROM legacy.quantity_group", params![q, project_id])?;
            tx.execute("INSERT INTO quantity_group_item(group_id, item_id) SELECT group_id + ?1, item_id + ?2 FROM legacy.quantity_group_item", params![q, i])?;
            tx.execute("INSERT INTO assignment(id, room_id, item_id, section, mode, height_m, length_m, manual_quantity, is_deduction) SELECT id + ?1, room_id + ?2, item_id + ?3, section, mode, height_m, length_m, manual_quantity, is_deduction FROM legacy.assignment", params![a, r, i])?;
            tx.execute("INSERT INTO deduction(id, assignment_id, description, quantity) SELECT id + ?1, assignment_id + ?2, description, quantity FROM legacy.deduction", params![d, a])?;
            let has_rebar: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM legacy.sqlite_master WHERE type='table' AND name='rebar_measurement')", [], |r| r.get(0))?;
            if has_rebar {
                tx.execute("INSERT INTO rebar_measurement(project_id,quantity,diameter_mm,length_cm,deduction,source,drawing,handles,layer) SELECT ?1,quantity,diameter_mm,length_cm,deduction,source,drawing,handles,layer FROM legacy.rebar_measurement WHERE project_id=1", [project_id])?;
                tx.execute("INSERT INTO rebar_settings(project_id,mode,symbols,multiplier,tolerance) SELECT ?1,mode,symbols,multiplier,tolerance FROM legacy.rebar_settings WHERE project_id=1", [project_id])?;
                let columns = {
                    let mut statement = tx.prepare("PRAGMA legacy.table_info(rebar_settings)")?;
                    let names = statement
                        .query_map([], |row| row.get::<_, String>(1))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    names
                };
                if columns.iter().any(|c| c == "marking_mode")
                    && columns.iter().any(|c| c == "marking_color")
                {
                    tx.execute("UPDATE rebar_settings SET marking_mode=(SELECT marking_mode FROM legacy.rebar_settings WHERE project_id=1), marking_color=(SELECT marking_color FROM legacy.rebar_settings WHERE project_id=1) WHERE project_id=?1", [project_id])?;
                }
                if columns.iter().any(|c| c == "recolor_text")
                    && columns.iter().any(|c| c == "draw_boxes")
                {
                    tx.execute("UPDATE rebar_settings SET recolor_text=(SELECT recolor_text FROM legacy.rebar_settings WHERE project_id=1), draw_boxes=(SELECT draw_boxes FROM legacy.rebar_settings WHERE project_id=1) WHERE project_id=?1", [project_id])?;
                }
            }
            tx.commit()?;
            Ok(project_id)
        })();
        db.execute_batch("DETACH DATABASE legacy")?;
        result
    }

    pub fn name(&self) -> rusqlite::Result<String> {
        self.db
            .query_row("SELECT name FROM project WHERE id = ?1", [self.id], |r| {
                r.get(0)
            })
    }

    pub fn default_height_m(&self) -> rusqlite::Result<f64> {
        self.db.query_row(
            "SELECT default_height_m FROM project WHERE id = ?1",
            [self.id],
            |row| row.get(0),
        )
    }

    pub fn set_default_height_m(&self, height: f64) -> rusqlite::Result<()> {
        if !valid_nonnegative(height) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.db.execute(
            "UPDATE project SET default_height_m = ?1 WHERE id = ?2",
            params![height, self.id],
        )?;
        Ok(())
    }

    pub fn groups(&self) -> rusqlite::Result<Vec<NamedRow>> {
        self.named_rows(
            "SELECT id, name FROM work_group WHERE project_id = ?1 ORDER BY id",
            Some(self.id),
        )
    }

    pub fn floors(&self, group_id: i64) -> rusqlite::Result<Vec<NamedRow>> {
        self.db.query_row(
            "SELECT id FROM work_group WHERE id = ?1 AND project_id = ?2",
            params![group_id, self.id],
            |row| row.get::<_, i64>(0),
        )?;
        self.named_rows(
            "SELECT id, name FROM floor WHERE group_id = ?1 ORDER BY id",
            Some(group_id),
        )
    }

    pub fn rooms(&self, floor_id: i64) -> rusqlite::Result<Vec<NamedRow>> {
        self.db.query_row("SELECT floor.id FROM floor JOIN work_group ON work_group.id = floor.group_id WHERE floor.id = ?1 AND work_group.project_id = ?2",
            params![floor_id, self.id], |row| row.get::<_, i64>(0))?;
        self.named_rows(
            "SELECT id, name FROM room WHERE floor_id = ?1 ORDER BY id",
            Some(floor_id),
        )
    }

    fn named_rows(&self, sql: &str, parent: Option<i64>) -> rusqlite::Result<Vec<NamedRow>> {
        let mut stmt = self.db.prepare(sql)?;
        let map = |row: &rusqlite::Row<'_>| {
            Ok(NamedRow {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        };
        let rows = if let Some(id) = parent {
            stmt.query_map([id], map)?
        } else {
            stmt.query_map([], map)?
        };
        rows.collect()
    }

    pub fn add_group(&self, name: &str) -> rusqlite::Result<i64> {
        self.db.execute(
            "INSERT INTO work_group(project_id, name) VALUES (?1, ?2)",
            params![self.id, name.trim()],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn add_floor(&self, group_id: i64, name: &str) -> rusqlite::Result<i64> {
        let changed = self.db.execute(
            "INSERT INTO floor(group_id, name) SELECT id, ?2 FROM work_group WHERE id = ?1 AND project_id = ?3",
            params![group_id, name.trim(), self.id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(self.db.last_insert_rowid())
    }

    pub fn add_room(&self, floor_id: i64, name: &str) -> rusqlite::Result<i64> {
        let changed = self.db.execute(
            "INSERT INTO room(project_id, floor_id, name) SELECT ?1, floor.id, ?3 FROM floor JOIN work_group ON work_group.id = floor.group_id WHERE floor.id = ?2 AND work_group.project_id = ?1",
            params![self.id, floor_id, name.trim()],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(self.db.last_insert_rowid())
    }

    pub fn next_room_name(&self, floor_id: i64) -> rusqlite::Result<String> {
        let names = self.rooms(floor_id)?;
        let mut number = 1;
        loop {
            let name = format!("Mahal {number}");
            if !names
                .iter()
                .any(|room| room.name.eq_ignore_ascii_case(&name))
            {
                return Ok(name);
            }
            number += 1;
        }
    }

    pub fn remove_rooms(&self, room_ids: &[i64]) -> rusqlite::Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        for room in room_ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            if self.db.execute(
                "DELETE FROM room WHERE id = ?1 AND project_id = ?2",
                params![room, self.id],
            )? != 1
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        transaction.commit()
    }

    pub fn add_bound_room(
        &self,
        floor_id: i64,
        name: &str,
        drawing_path: &str,
        handle: &str,
        measurement: Measurement,
    ) -> rusqlite::Result<i64> {
        if drawing_path.is_empty()
            || handle.is_empty()
            || !valid_nonnegative(measurement.area_m2)
            || !valid_nonnegative(measurement.perimeter_m)
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let changed = self.db.execute(
            "INSERT INTO room(project_id, floor_id, name, drawing_path, entity_handle, area_m2, perimeter_m, boundary_valid) SELECT ?1, floor.id, ?3, ?4, ?5, ?6, ?7, 1 FROM floor JOIN work_group ON work_group.id = floor.group_id WHERE floor.id = ?2 AND work_group.project_id = ?1",
            params![self.id, floor_id, name.trim(), drawing_path, handle, measurement.area_m2, measurement.perimeter_m],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(self.db.last_insert_rowid())
    }

    pub fn room_location(&self, room_id: i64) -> rusqlite::Result<Option<(i64, i64)>> {
        self.db.query_row(
            "SELECT floor.id, floor.group_id FROM room JOIN floor ON floor.id = room.floor_id WHERE room.id = ?1",
            [room_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()
    }

    pub fn room_binding(
        &self,
        room_id: i64,
    ) -> rusqlite::Result<Option<(String, String, Option<String>, String)>> {
        self.db.query_row(
            "SELECT drawing_path, entity_handle, label_handle, name FROM room WHERE id = ?1 AND drawing_path IS NOT NULL",
            [room_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional()
    }

    pub fn set_label_handle(&self, room_id: i64, handle: &str) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE room SET label_handle = ?1 WHERE id = ?2",
            params![handle, room_id],
        )?;
        Ok(())
    }

    pub fn room_hatch_handle(&self, room_id: i64) -> rusqlite::Result<Option<String>> {
        self.db.query_row(
            "SELECT hatch_handle FROM room WHERE id = ?1 AND project_id = ?2",
            params![room_id, self.id],
            |row| row.get(0),
        )
    }

    pub fn set_room_hatch_handle(&self, room_id: i64, handle: &str) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE room SET hatch_handle = ?1 WHERE id = ?2 AND project_id = ?3",
            params![handle, room_id, self.id],
        )?;
        Ok(())
    }

    pub fn rename_room(&self, room_id: i64, name: &str) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE room SET name = ?1 WHERE id = ?2",
            params![name.trim(), room_id],
        )?;
        Ok(())
    }

    pub fn room_poz_label(&self, room_id: i64) -> rusqlite::Result<Option<String>> {
        self.db.query_row(
            "SELECT poz_label_handle FROM room WHERE id=?1 AND project_id=?2",
            params![room_id, self.id],
            |row| row.get(0),
        )
    }
    pub fn set_room_poz_label(&self, room_id: i64, handle: &str) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE room SET poz_label_handle=?1 WHERE id=?2 AND project_id=?3",
            params![handle, room_id, self.id],
        )?;
        Ok(())
    }

    pub fn room_for_entity(
        &self,
        drawing_path: &str,
        handle: &str,
    ) -> rusqlite::Result<Option<i64>> {
        self.db.query_row(
            "SELECT id FROM room WHERE project_id = ?1 AND drawing_path = ?2 AND (entity_handle = ?3 OR label_handle = ?3 OR poz_label_handle = ?3 OR hatch_handle = ?3)",
            params![self.id, drawing_path, handle], |row| row.get(0),
        ).optional()
    }

    pub fn set_room_boundary(
        &self,
        room_id: i64,
        drawing_path: &str,
        entity_handle: &str,
        measurement: Measurement,
    ) -> rusqlite::Result<()> {
        if drawing_path.trim().is_empty()
            || entity_handle.trim().is_empty()
            || !valid_nonnegative(measurement.area_m2)
            || !valid_nonnegative(measurement.perimeter_m)
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let changed = self.db.execute(
            "UPDATE room SET drawing_path = ?1, entity_handle = ?2, area_m2 = ?3, perimeter_m = ?4, boundary_valid = 1 WHERE id = ?5",
            params![drawing_path, entity_handle, measurement.area_m2, measurement.perimeter_m, room_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn add_item(
        &self,
        name: &str,
        unit: Unit,
        poz: &str,
        short_description: &str,
    ) -> rusqlite::Result<i64> {
        self.db.execute(
            "INSERT INTO item(project_id, name, unit, poz, short_description) VALUES (?5, ?1, ?2, ?3, ?4)",
            params![
                name.trim(),
                unit.as_str(),
                poz.trim(),
                short_description.trim(), self.id
            ],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn items(&self) -> rusqlite::Result<Vec<CatalogItem>> {
        let mut stmt = self
            .db
            .prepare("SELECT id, name, unit, poz, short_description FROM item WHERE project_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([self.id], |r| {
            Ok(CatalogItem {
                id: r.get(0)?,
                name: r.get(1)?,
                unit: Unit::parse(&r.get::<_, String>(2)?)?,
                poz: r.get(3)?,
                short_description: r.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn quantity_groups(&self) -> rusqlite::Result<Vec<NamedRow>> {
        self.named_rows(
            "SELECT id, name FROM quantity_group WHERE project_id = ?1 ORDER BY id",
            Some(self.id),
        )
    }

    pub fn add_quantity_group(&self, name: &str) -> rusqlite::Result<i64> {
        self.db.execute(
            "INSERT INTO quantity_group(project_id, name) VALUES (?1, ?2)",
            params![self.id, name.trim()],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn add_group_item(&self, group_id: i64, item_id: i64) -> rusqlite::Result<()> {
        let belongs: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM quantity_group q JOIN item i ON i.id = ?2 WHERE q.id = ?1 AND q.project_id = ?3 AND i.project_id = ?3",
            params![group_id, item_id, self.id], |row| row.get(0))?;
        if belongs == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        self.db.execute(
            "INSERT OR IGNORE INTO quantity_group_item(group_id, item_id) VALUES (?1, ?2)",
            params![group_id, item_id],
        )?;
        Ok(())
    }

    pub fn remove_group_item(&self, group_id: i64, item_id: i64) -> rusqlite::Result<()> {
        self.db.execute(
            "DELETE FROM quantity_group_item WHERE group_id = ?1 AND item_id = ?2",
            params![group_id, item_id],
        )?;
        Ok(())
    }

    pub fn group_items(&self, group_id: i64) -> rusqlite::Result<Vec<CatalogItem>> {
        let mut stmt = self.db.prepare("SELECT i.id, i.name, i.unit, i.poz, i.short_description FROM item i JOIN quantity_group_item gi ON gi.item_id = i.id JOIN quantity_group q ON q.id = gi.group_id WHERE gi.group_id = ?1 AND q.project_id = ?2 AND i.project_id = ?2 ORDER BY gi.rowid")?;
        let rows = stmt
            .query_map(params![group_id, self.id], |r| {
                Ok(CatalogItem {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    unit: Unit::parse(&r.get::<_, String>(2)?)?,
                    poz: r.get(3)?,
                    short_description: r.get(4)?,
                })
            })?
            .collect();
        rows
    }

    pub fn assign_group(
        &self,
        room_id: i64,
        section: RoomSection,
        group_id: i64,
        mode: QuantityMode,
        is_deduction: bool,
    ) -> rusqlite::Result<Vec<i64>> {
        self.assign_selection(room_id, section, &[], &[group_id], mode, is_deduction)
    }

    /// Apply one catalog selection atomically, counting overlapping group members once.
    pub fn assign_selection(
        &self,
        room_id: i64,
        section: RoomSection,
        item_ids: &[i64],
        group_ids: &[i64],
        mode: QuantityMode,
        is_deduction: bool,
    ) -> rusqlite::Result<Vec<i64>> {
        Ok(self
            .assign_rooms(&[room_id], section, item_ids, group_ids, mode, is_deduction)?
            .into_iter()
            .map(|(_, id)| id)
            .collect())
    }

    /// Apply one selection to all target rooms in one transaction.
    pub fn assign_rooms(
        &self,
        room_ids: &[i64],
        section: RoomSection,
        item_ids: &[i64],
        group_ids: &[i64],
        mode: QuantityMode,
        is_deduction: bool,
    ) -> rusqlite::Result<Vec<(i64, i64)>> {
        let tx = self.db.unchecked_transaction()?;
        let rooms: std::collections::BTreeSet<_> = room_ids.iter().copied().collect();
        if rooms.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let mut items: std::collections::BTreeSet<i64> = item_ids.iter().copied().collect();
        for group in group_ids {
            let members = self.group_items(*group)?;
            if members.is_empty() {
                return Err(rusqlite::Error::InvalidQuery);
            }
            items.extend(members.into_iter().map(|item| item.id));
        }
        if items.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let mut ids = Vec::new();
        for room in rooms {
            for item in &items {
                ids.push((
                    room,
                    self.assign_with_deduction(room, section, *item, mode, is_deduction)?,
                ));
            }
        }
        tx.commit()?;
        Ok(ids)
    }

    pub fn assign(
        &self,
        room_id: i64,
        section: RoomSection,
        item_id: i64,
        mode: QuantityMode,
    ) -> rusqlite::Result<i64> {
        self.assign_with_deduction(room_id, section, item_id, mode, false)
    }

    pub fn assign_with_deduction(
        &self,
        room_id: i64,
        section: RoomSection,
        item_id: i64,
        mode: QuantityMode,
        is_deduction: bool,
    ) -> rusqlite::Result<i64> {
        let (mode_name, height, length, manual) = quantity_mode_parts(mode)?;
        let unit: String = self.db.query_row(
            "SELECT unit FROM item WHERE id = ?1 AND project_id = ?2",
            params![item_id, self.id],
            |row| row.get(0),
        )?;
        self.db.query_row(
            "SELECT id FROM room WHERE id = ?1 AND project_id = ?2",
            params![room_id, self.id],
            |row| row.get::<_, i64>(0),
        )?;
        if mode_name == "perimeter_height" && unit != "m2" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.db.execute(
            "INSERT INTO assignment(room_id, section, item_id, mode, height_m, length_m, manual_quantity, is_deduction) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![room_id, section.as_str(), item_id, mode_name, height, length, manual, is_deduction],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn set_assignment_deduction(
        &self,
        assignment_id: i64,
        is_deduction: bool,
    ) -> rusqlite::Result<()> {
        let changed = self.db.execute(
            "UPDATE assignment SET is_deduction = ?1 WHERE id = ?2",
            params![is_deduction, assignment_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn update_assignment_mode(
        &self,
        assignment_id: i64,
        mode: QuantityMode,
    ) -> rusqlite::Result<()> {
        let (mode_name, height, length, manual) = quantity_mode_parts(mode)?;
        let unit: String = self.db.query_row(
            "SELECT item.unit FROM assignment JOIN item ON item.id = assignment.item_id WHERE assignment.id = ?1",
            [assignment_id], |row| row.get(0),
        )?;
        if mode_name == "perimeter_height" && unit != "m2" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.db.execute(
            "UPDATE assignment SET mode = ?1, height_m = ?2, length_m = ?3, manual_quantity = ?4 WHERE id = ?5",
            params![mode_name, height, length, manual, assignment_id],
        )?;
        Ok(())
    }

    pub fn update_assignment_details(
        &self,
        assignment_id: i64,
        section: RoomSection,
        mode: QuantityMode,
        is_deduction: bool,
    ) -> rusqlite::Result<()> {
        let (mode_name, height, length, manual) = quantity_mode_parts(mode)?;
        let unit: String = self.db.query_row(
            "SELECT i.unit FROM assignment a JOIN item i ON i.id = a.item_id JOIN room r ON r.id = a.room_id WHERE a.id = ?1 AND i.project_id = ?2 AND r.project_id = ?2",
            params![assignment_id, self.id], |row| row.get(0),
        )?;
        if mode_name == "perimeter_height" && unit != "m2" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.db.execute(
            "UPDATE assignment SET section = ?1, mode = ?2, height_m = ?3, length_m = ?4, manual_quantity = ?5, is_deduction = ?6 WHERE id = ?7",
            params![section.as_str(), mode_name, height, length, manual, is_deduction, assignment_id],
        )?;
        Ok(())
    }

    pub fn add_deduction(
        &self,
        assignment_id: i64,
        description: &str,
        quantity: f64,
    ) -> rusqlite::Result<i64> {
        if !quantity.is_finite() || quantity <= 0.0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.db.execute(
            "INSERT INTO deduction(assignment_id, description, quantity) VALUES (?1, ?2, ?3)",
            params![assignment_id, description.trim(), quantity],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn deductions(&self, assignment_id: i64) -> rusqlite::Result<Vec<Deduction>> {
        let mut stmt = self.db.prepare(
            "SELECT id, description, quantity FROM deduction WHERE assignment_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([assignment_id], |row| {
            Ok(Deduction {
                id: row.get(0)?,
                description: row.get(1)?,
                quantity: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn remove_deduction(&self, deduction_id: i64) -> rusqlite::Result<()> {
        let changed = self
            .db
            .execute("DELETE FROM deduction WHERE id = ?1", [deduction_id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn remove_assignment(&self, assignment_id: i64) -> rusqlite::Result<()> {
        let changed = self
            .db
            .execute("DELETE FROM assignment WHERE id = ?1", [assignment_id])?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn room_assignments(&self, room_id: i64) -> rusqlite::Result<Vec<Assignment>> {
        let mut stmt = self.db.prepare(
            "SELECT a.id, a.section, i.poz, i.short_description, i.name, i.unit, a.mode, a.height_m, a.manual_quantity,
                    r.area_m2, r.perimeter_m,
                    COALESCE((SELECT SUM(d.quantity) FROM deduction d WHERE d.assignment_id = a.id), 0),
                    r.boundary_valid, a.length_m, a.is_deduction
             FROM assignment a JOIN item i ON i.id = a.item_id
             JOIN room r ON r.id = a.room_id WHERE a.room_id = ?1 ORDER BY a.id",
        )?;
        let rows = stmt.query_map([room_id], |r| {
            let unit = Unit::parse(&r.get::<_, String>(5)?)?;
            let mode_name: String = r.get(6)?;
            let custom_length: Option<f64> = r.get(13)?;
            let mode = match mode_name.as_str() {
                "room" => QuantityMode::Room,
                "perimeter_height" => {
                    let height_m = r.get(7)?;
                    match custom_length {
                        Some(length_m) => QuantityMode::LengthTimesHeight { length_m, height_m },
                        None => QuantityMode::PerimeterTimesHeight(height_m),
                    }
                }
                "manual" => QuantityMode::Manual(r.get(8)?),
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let source_valid = (matches!(mode, QuantityMode::Manual(_))
                || (mode == QuantityMode::Room && unit == Unit::Each)
                || matches!(mode, QuantityMode::LengthTimesHeight { .. }))
                || r.get::<_, i64>(12)? != 0;
            let gross = match mode {
                QuantityMode::Room => match unit {
                    Unit::SquareMeter => r.get(9)?,
                    Unit::Meter => r.get(10)?,
                    Unit::Each => 1.0,
                },
                QuantityMode::PerimeterTimesHeight(height_m) => r.get::<_, f64>(10)? * height_m,
                QuantityMode::LengthTimesHeight { length_m, height_m } => length_m * height_m,
                QuantityMode::Manual(quantity) => quantity,
            };
            let section = match r.get::<_, String>(1)?.as_str() {
                "wall" => RoomSection::Wall,
                "floor" => RoomSection::Floor,
                "ceiling" => RoomSection::Ceiling,
                "other" => RoomSection::Other,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let deductions: f64 = r.get(11)?;
            let is_deduction = r.get::<_, i64>(14)? != 0;
            Ok(Assignment {
                id: r.get(0)?,
                section,
                poz: r.get(2)?,
                short_description: r.get(3)?,
                item_name: r.get(4)?,
                unit,
                mode,
                gross,
                deductions,
                net: if source_valid {
                    (gross - deductions) * if is_deduction { -1.0 } else { 1.0 }
                } else {
                    0.0
                },
                source_valid,
                is_deduction,
            })
        })?;
        rows.collect()
    }

    pub fn room_measurement(&self, room_id: i64) -> rusqlite::Result<Option<Measurement>> {
        self.db
            .query_row(
                "SELECT area_m2, perimeter_m FROM room WHERE id = ?1",
                [room_id],
                |r| {
                    Ok(Measurement {
                        area_m2: r.get(0)?,
                        perimeter_m: r.get(1)?,
                    })
                },
            )
            .optional()
    }

    pub fn room_info(&self, room_id: i64) -> rusqlite::Result<Option<RoomInfo>> {
        self.db.query_row(
            "SELECT area_m2, perimeter_m, drawing_path IS NOT NULL, boundary_valid FROM room WHERE id = ?1",
            [room_id],
            |row| Ok(RoomInfo {
                measurement: Measurement { area_m2: row.get(0)?, perimeter_m: row.get(1)? },
                bound: row.get::<_, i64>(2)? != 0,
                boundary_valid: row.get::<_, i64>(3)? != 0,
            }),
        ).optional()
    }

    pub fn mark_boundary_missing(&self, room_id: i64) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE room SET boundary_valid = 0 WHERE id = ?1",
            [room_id],
        )?;
        Ok(())
    }

    pub fn relink_drawing(&self, previous_path: &str, new_path: &str) -> rusqlite::Result<usize> {
        if previous_path.is_empty() || new_path.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let transaction = self.db.unchecked_transaction()?;
        let changed = self.db.execute(
            "UPDATE room SET drawing_path = ?1 WHERE drawing_path = ?2 AND project_id = ?3",
            params![new_path, previous_path, self.id],
        )?;
        self.db.execute(
            "UPDATE rebar_measurement SET drawing = ?1 WHERE drawing = ?2 AND project_id = ?3",
            params![new_path, previous_path, self.id],
        )?;
        transaction.commit()?;
        Ok(changed)
    }

    pub fn bound_rooms(&self, drawing_path: &str) -> rusqlite::Result<Vec<(i64, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT id, entity_handle FROM room WHERE project_id = ?2 AND drawing_path = ?1 AND entity_handle IS NOT NULL",
        )?;
        let rows = stmt.query_map(params![drawing_path, self.id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
        rows.collect()
    }

    pub fn bound_room_labels(
        &self,
        drawing_path: &str,
    ) -> rusqlite::Result<Vec<(i64, String, Option<String>, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT id, entity_handle, label_handle, name FROM room WHERE project_id = ?2 AND drawing_path = ?1 AND entity_handle IS NOT NULL",
        )?;
        let rows = stmt.query_map(params![drawing_path, self.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;
        rows.collect()
    }

    pub fn room_for_boundary(
        &self,
        drawing_path: &str,
        entity_handle: &str,
    ) -> rusqlite::Result<Option<i64>> {
        self.db
            .query_row(
                "SELECT id FROM room WHERE project_id = ?3 AND drawing_path = ?1 AND entity_handle = ?2",
                params![drawing_path, entity_handle, self.id],
                |row| row.get(0),
            )
            .optional()
    }
}

fn valid_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn quantity_mode_parts(
    mode: QuantityMode,
) -> rusqlite::Result<(&'static str, Option<f64>, Option<f64>, Option<f64>)> {
    match mode {
        QuantityMode::Room => Ok(("room", None, None, None)),
        QuantityMode::PerimeterTimesHeight(h) if valid_nonnegative(h) => {
            Ok(("perimeter_height", Some(h), None, None))
        }
        QuantityMode::LengthTimesHeight { length_m, height_m }
            if valid_nonnegative(length_m)
                && valid_nonnegative(height_m)
                && valid_nonnegative(length_m * height_m) =>
        {
            Ok(("perimeter_height", Some(height_m), Some(length_m), None))
        }
        QuantityMode::Manual(q) if valid_nonnegative(q) => Ok(("manual", None, None, Some(q))),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_rooms_is_atomic_and_cascades_only_room_data() {
        let project = Project::open(":memory:", "Delete").unwrap();
        let group = project.add_group("G").unwrap();
        let floor = project.add_floor(group, "F").unwrap();
        let a = project.add_room(floor, "A").unwrap();
        let b = project.add_room(floor, "B").unwrap();
        let item = project.add_item("Poz", Unit::SquareMeter, "1", "").unwrap();
        let assignment = project
            .assign(a, RoomSection::Floor, item, QuantityMode::Manual(5.))
            .unwrap();
        project.add_deduction(assignment, "Minha", 1.).unwrap();
        assert!(project.remove_rooms(&[a, 999999]).is_err());
        assert!(project.room_info(a).unwrap().is_some());
        assert_eq!(project.deductions(assignment).unwrap().len(), 1);
        project.db.pragma_update(None, "query_only", true).unwrap();
        assert!(project.remove_rooms(&[a]).is_err());
        assert!(project.room_info(a).unwrap().is_some());
        project.db.pragma_update(None, "query_only", false).unwrap();
        project.remove_rooms(&[a, a]).unwrap();
        assert!(project.room_info(a).unwrap().is_none());
        assert!(project.room_info(b).unwrap().is_some());
        assert!(project.room_assignments(a).unwrap().is_empty());
        assert!(project.deductions(assignment).unwrap().is_empty());
        assert_eq!(project.items().unwrap().len(), 1);
    }

    #[test]
    fn catalog_selection_is_atomic_and_deduplicates_group_members() {
        let project = Project::open(":memory:", "Test").unwrap();
        let group = project.add_group("İnşaat").unwrap();
        let floor = project.add_floor(group, "Zemin").unwrap();
        let room = project.add_room(floor, "Banyo").unwrap();
        let tile = project
            .add_item("Seramik", Unit::SquareMeter, "01", "")
            .unwrap();
        let paint = project
            .add_item("Boya", Unit::SquareMeter, "02", "")
            .unwrap();
        let count = project.add_item("Kapı", Unit::Each, "03", "").unwrap();
        let bundle = project.add_quantity_group("Kaplamalar").unwrap();
        project.add_group_item(bundle, tile).unwrap();
        project.add_group_item(bundle, paint).unwrap();
        let ids = project
            .assign_selection(
                room,
                RoomSection::Floor,
                &[tile],
                &[bundle],
                QuantityMode::Manual(4.0),
                true,
            )
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert!(project
            .room_assignments(room)
            .unwrap()
            .iter()
            .all(|row| row.is_deduction && row.net == -4.0));
        assert!(project
            .assign_selection(
                room,
                RoomSection::Wall,
                &[tile, count],
                &[],
                QuantityMode::PerimeterTimesHeight(2.8),
                false
            )
            .is_err());
        assert_eq!(
            project.room_assignments(room).unwrap().len(),
            2,
            "failed batch must leave no partial assignments"
        );
        project
            .update_assignment_details(
                ids[0],
                RoomSection::Ceiling,
                QuantityMode::Manual(7.0),
                false,
            )
            .unwrap();
        let rows = project.room_assignments(room).unwrap();
        let updated = rows.iter().find(|row| row.id == ids[0]).unwrap();
        assert_eq!(updated.section, RoomSection::Ceiling);
        assert!(!updated.is_deduction);
        assert_eq!(updated.net, 7.0);
    }

    #[test]
    fn shared_database_separates_projects_and_imports_standalone_file() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let shared =
            std::env::temp_dir().join(format!("ocs-shared-{}-{nonce}.sqlite", std::process::id()));
        let old =
            std::env::temp_dir().join(format!("ocs-old-{}-{nonce}.sqlite", std::process::id()));
        let a = Project::create_shared(&shared, "A").unwrap();
        let b = Project::create_shared(&shared, "B").unwrap();
        a.set_default_height_m(3.10).unwrap();
        assert_eq!(a.default_height_m().unwrap(), 3.10);
        assert_eq!(b.default_height_m().unwrap(), 2.80);
        let add = |project: &Project| {
            let group = project.add_group("İnşaat").unwrap();
            let floor = project.add_floor(group, "Zemin").unwrap();
            let room = project
                .add_bound_room(
                    floor,
                    "Banyo",
                    "plan.dwg",
                    "A1",
                    Measurement {
                        area_m2: 12.0,
                        perimeter_m: 14.0,
                    },
                )
                .unwrap();
            let item = project
                .add_item("Seramik", Unit::SquareMeter, "01", "Kaplama")
                .unwrap();
            let bundle = project.add_quantity_group("Kaplamalar").unwrap();
            project.add_group_item(bundle, item).unwrap();
            project
                .assign(room, RoomSection::Floor, item, QuantityMode::Room)
                .unwrap();
            room
        };
        let a_room = add(&a);
        let b_room = add(&b);
        assert_ne!(a_room, b_room);
        assert_eq!(a.room_for_entity("plan.dwg", "A1").unwrap(), Some(a_room));
        assert_eq!(b.room_for_entity("plan.dwg", "A1").unwrap(), Some(b_room));
        assert!(a.add_floor(b.groups().unwrap()[0].id, "Yanlış").is_err());
        assert!(a
            .add_group_item(b.quantity_groups().unwrap()[0].id, a.items().unwrap()[0].id)
            .is_err());
        assert!(a
            .assign(
                b_room,
                RoomSection::Floor,
                a.items().unwrap()[0].id,
                QuantityMode::Room
            )
            .is_err());
        assert_eq!(Project::list_shared(&shared).unwrap().len(), 2);
        let legacy = Project::open(&old, "Eski").unwrap();
        add(&legacy);
        drop(legacy);
        let old_size = std::fs::metadata(&old).unwrap().len();
        let imported_id = Project::import_shared(&shared, &old).unwrap();
        assert_eq!(std::fs::metadata(&old).unwrap().len(), old_size);
        let imported = Project::open_shared(&shared, imported_id).unwrap();
        assert_eq!(imported.name().unwrap(), "Eski");
        assert_eq!(imported.groups().unwrap().len(), 1);
        assert_eq!(imported.items().unwrap().len(), 1);
        let room = imported.room_for_entity("plan.dwg", "A1").unwrap().unwrap();
        assert_eq!(imported.room_assignments(room).unwrap()[0].net, 12.0);
        assert_eq!(a.groups().unwrap().len(), 1);
        drop((a, b, imported));
        let _ = std::fs::remove_file(shared);
        let _ = std::fs::remove_file(old);
    }

    #[test]
    fn independent_projects_and_quantity_modes() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        db.execute("INSERT INTO project(id, name) VALUES (1, 'Test')", [])
            .unwrap();
        let project = Project { db, id: 1 };
        let group = project.add_group("İnşaat").unwrap();
        let floor = project.add_floor(group, "Zemin").unwrap();
        let room = project.add_room(floor, "Banyo").unwrap();
        project
            .set_room_boundary(
                room,
                "plan.dwg",
                "A1",
                Measurement {
                    area_m2: 12.0,
                    perimeter_m: 14.0,
                },
            )
            .unwrap();
        let tile = project
            .add_item("Seramik", Unit::SquareMeter, "", "")
            .unwrap();
        let paint = project.add_item("Boya", Unit::SquareMeter, "", "").unwrap();
        let floor_item = project
            .assign(room, RoomSection::Floor, tile, QuantityMode::Room)
            .unwrap();
        let wall_item = project
            .assign(
                room,
                RoomSection::Wall,
                paint,
                QuantityMode::PerimeterTimesHeight(2.8),
            )
            .unwrap();
        project.add_deduction(wall_item, "Kapı", 2.0).unwrap();
        let rows = project.room_assignments(room).unwrap();
        assert_eq!(rows[0].id, floor_item);
        assert_eq!(rows[0].net, 12.0);
        assert!((rows[1].net - 37.2).abs() < 1e-10);
        project
            .set_room_boundary(
                room,
                "plan.dwg",
                "A1",
                Measurement {
                    area_m2: 15.0,
                    perimeter_m: 16.0,
                },
            )
            .unwrap();
        let updated = project.room_assignments(room).unwrap();
        assert_eq!(updated[0].net, 15.0);
        assert!((updated[1].net - 42.8).abs() < 1e-10);
        let deduction_item = project
            .assign_with_deduction(
                room,
                RoomSection::Wall,
                paint,
                QuantityMode::LengthTimesHeight {
                    length_m: 3.0,
                    height_m: 2.0,
                },
                true,
            )
            .unwrap();
        let with_item_deduction = project.room_assignments(room).unwrap();
        assert_eq!(with_item_deduction[2].id, deduction_item);
        assert_eq!(with_item_deduction[2].net, -6.0);
        assert!(with_item_deduction[2].is_deduction);
        project
            .set_assignment_deduction(deduction_item, false)
            .unwrap();
        assert_eq!(project.room_assignments(room).unwrap()[2].net, 6.0);
        project.mark_boundary_missing(room).unwrap();
        let missing_boundary = project.room_assignments(room).unwrap();
        assert!(!missing_boundary[0].source_valid);
        assert!(missing_boundary[2].source_valid);
        assert!(project
            .assign(
                room,
                RoomSection::Other,
                tile,
                QuantityMode::Manual(f64::NAN)
            )
            .is_err());
        assert!(project
            .set_room_boundary(
                room,
                "plan.dwg",
                "A1",
                Measurement {
                    area_m2: f64::INFINITY,
                    perimeter_m: 1.0
                }
            )
            .is_err());
    }

    #[test]
    fn project_file_reopens_without_mixing_projects() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let first = std::env::temp_dir().join(format!(
            "ocs-metraj-{}-{}-a.sqlite",
            std::process::id(),
            nonce
        ));
        let second = first.with_file_name(format!(
            "ocs-metraj-{}-{}-b.sqlite",
            std::process::id(),
            nonce
        ));
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);
        {
            let a = Project::open(&first, "Proje A").unwrap();
            let group = a.add_group("İnşaat").unwrap();
            let floor = a.add_floor(group, "Zemin").unwrap();
            let room = a.add_room(floor, "Banyo").unwrap();
            a.set_room_boundary(
                room,
                "ocs-unsaved-tab:42",
                "A1",
                Measurement {
                    area_m2: 12.0,
                    perimeter_m: 14.0,
                },
            )
            .unwrap();
            let b = Project::open(&second, "Proje B").unwrap();
            assert_eq!(b.name().unwrap(), "Proje B");
        }
        let a = Project::open(&first, "Değişmemeli").unwrap();
        assert_eq!(a.name().unwrap(), "Proje A");
        let room_id = a
            .rooms(a.floors(a.groups().unwrap()[0].id).unwrap()[0].id)
            .unwrap()[0]
            .id;
        assert!(!a.room_info(room_id).unwrap().unwrap().boundary_valid);
        assert_eq!(
            a.db.query_row("SELECT COUNT(*) FROM work_group", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(a);
        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn legacy_project_preserves_assignments_during_schema_upgrade() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ocs-metraj-legacy-{}-{nonce}.sqlite",
            std::process::id()
        ));
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                "CREATE TABLE project (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE room (id INTEGER PRIMARY KEY, floor_id INTEGER NOT NULL, name TEXT NOT NULL,
                    drawing_path TEXT, entity_handle TEXT, area_m2 REAL NOT NULL, perimeter_m REAL NOT NULL);
                 CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL, unit TEXT NOT NULL);
                 CREATE TABLE assignment (id INTEGER PRIMARY KEY, room_id INTEGER NOT NULL, item_id INTEGER NOT NULL,
                    section TEXT NOT NULL, mode TEXT NOT NULL, height_m REAL, manual_quantity REAL);
                 INSERT INTO project VALUES (1, 'Eski Proje');
                 INSERT INTO room VALUES (1, 1, 'Banyo', 'plan.dwg', 'A1', 12, 14);
                 INSERT INTO item VALUES (1, 'Seramik', 'm2');
                 INSERT INTO assignment VALUES (1, 1, 1, 'floor', 'room', NULL, NULL);",
            )
            .unwrap();
        }
        let project = Project::open(&path, "Değişmemeli").unwrap();
        assert_eq!(project.name().unwrap(), "Eski Proje");
        assert!(project.room_info(1).unwrap().unwrap().boundary_valid);
        let assignments = project.room_assignments(1).unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].net, 12.0);
        assert!(!assignments[0].is_deduction);
        drop(project);
        let _ = std::fs::remove_file(path);
    }
}
