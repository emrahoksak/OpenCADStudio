//! Project-local quantity takeoff data. CAD drawings remain the geometry source.

mod project;
pub mod rebar;
mod rebar_store;
mod summary;
pub use rebar_store::export_rebar;
pub use summary::{export_summary, Summary, SummaryRow};

pub use project::{
    Assignment, CatalogItem, Deduction, Measurement, NamedRow, Project, QuantityMode, RoomInfo,
    RoomSection, Unit,
};
