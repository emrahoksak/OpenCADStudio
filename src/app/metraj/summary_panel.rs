use super::{send, Chrome, Message, MetrajMsg, MetrajUi, PanelId};
use crate::ui::style::{
    common::muted_style,
    form::{button_style, field_style},
};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Fill};
impl MetrajUi {
    pub(crate) fn summary_view(&self, width: f32, chrome: Chrome) -> Element<'_, Message> {
        let mut body = column![].spacing(3);
        if let Some(project) = &self.project {
            let mut cache = self.summary_cache.borrow_mut();
            let key = (project.id(), project.summary_revision());
            if cache
                .as_ref()
                .is_none_or(|(id, revision, _)| (*id, *revision) != key)
            {
                match project.summary() {
                    Ok(summary) => *cache = Some((key.0, key.1, summary)),
                    Err(error) => {
                        body = body.push(text(error.to_string()).size(11));
                    }
                }
            }
            if let Some((_, _, summary)) = cache.as_ref() {
                body = body.push(text(project.name().unwrap_or_default()).size(11));
                for entry in summary.rows.iter().filter(|r| {
                    r.poz
                        .to_lowercase()
                        .contains(&self.summary_filter.trim().to_lowercase())
                }) {
                    body = body.push(
                        container(
                            column![
                                row![
                                    text(entry.poz.clone()).size(11).width(Fill),
                                    text(format!("{:.3} {}", entry.net(), entry.unit())).size(11)
                                ]
                                .spacing(5),
                                text(format!(
                                    "{} · Ekle {:.3} · Minha {:.3}{}{}",
                                    entry.description,
                                    entry.added,
                                    entry.deducted,
                                    if entry.invalid > 0 {
                                        format!(" · {} geçersiz kaynak", entry.invalid)
                                    } else {
                                        String::new()
                                    },
                                    if entry.units.len() > 1 {
                                        " · Karma birim"
                                    } else {
                                        ""
                                    }
                                ))
                                .size(10)
                                .style(muted_style),
                            ]
                            .spacing(2),
                        )
                        .padding([4, 5])
                        .width(Fill),
                    );
                }
                if summary.without_poz > 0 {
                    body = body.push(
                        text(format!(
                            "Poz numarası olmayan {} atama toplam dışında.",
                            summary.without_poz
                        ))
                        .size(10)
                        .style(muted_style),
                    );
                }
            }
        } else {
            body = body.push(text("İcmal için proje seçin.").size(11));
        }
        let content = column![
            text_input("Poz no filtresi", &self.summary_filter)
                .on_input(|v| send(MetrajMsg::SummaryFilter(v)))
                .size(11)
                .padding([4, 6])
                .style(field_style),
            text("Aynı poz numarası tek satırda toplanır. Net = Ekle − Minha.")
                .size(10)
                .style(muted_style),
            scrollable(body).height(Fill),
            button(text("Excel'e aktar (.xlsx)").size(11))
                .on_press_maybe(
                    self.project
                        .as_ref()
                        .map(|_| send(MetrajMsg::ExportSummary))
                )
                .width(Fill)
                .padding([4, 6])
                .style(button_style(true)),
            text("Excel aktarımı projenin tüm pozlarını içerir.")
                .size(10)
                .style(muted_style),
            text(&self.status).size(10),
        ]
        .spacing(5);
        crate::ui::dock::frame(
            crate::ui::dock::title_bar(PanelId::MetrajSummary, "Metraj İcmali".into(), chrome),
            content,
            width,
        )
    }
}
