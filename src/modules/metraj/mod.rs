use crate::modules::{CadModule, IconKind, ModuleEvent, RibbonGroup, ToolDef};

pub struct MetrajModule;

impl CadModule for MetrajModule {
    fn id(&self) -> &'static str {
        "metraj"
    }
    fn title(&self) -> &'static str {
        "Metraj"
    }
    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: std::sync::OnceLock<Vec<RibbonGroup>> = std::sync::OnceLock::new();
        GROUPS.get_or_init(|| {
            vec![
                RibbonGroup {
                    title: "Mahal",
                    tools: [
                        ("METRAJ", "Metraj Panelleri"),
                        ("MM", "Mahal Çiz"),
                        ("NM", "Mahal Ata"),
                    ]
                    .into_iter()
                    .map(|(id, label)| {
                        ToolDef {
                            id,
                            label,
                            icon: IconKind::Svg(match id {
                                "METRAJ" => include_bytes!("../../../assets/icons/report.svg"),
                                "MM" => include_bytes!("../../../assets/icons/polyline.svg"),
                                _ => include_bytes!("../../../assets/icons/scale_list.svg"),
                            }),
                            event: ModuleEvent::Command(id.to_string()),
                        }
                        .into()
                    })
                    .collect(),
                },
                RibbonGroup {
                    title: "Demir",
                    tools: [
                        ("DEMIR", "Demir Metrajı"),
                        ("DM1", "Tek Metin"),
                        ("DM2", "Yakınlık"),
                        ("DMY", "Yatay"),
                        ("DMD", "Düşey"),
                        ("DMA", "Adetsiz"),
                    ]
                    .into_iter()
                    .map(|(id, label)| {
                        ToolDef {
                            id,
                            label,
                            icon: IconKind::Svg(include_bytes!(
                                "../../../assets/icons/measure.svg"
                            )),
                            event: ModuleEvent::Command(id.to_string()),
                        }
                        .into()
                    })
                    .collect(),
                },
            ]
        })
    }
}
