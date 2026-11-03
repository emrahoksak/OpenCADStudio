use super::{drawing_key, MetrajUi, OpenCADStudio};
use crate::app::document::DocumentTab;
use crate::app::Message;
use crate::scene::Scene;
use glam::{DVec3, Mat4};
use iced::widget::{canvas, Space};
use iced::{Element, Fill, Point, Rectangle, Theme};
use std::sync::Arc;

#[derive(PartialEq)]
struct SourceKey {
    project: i64,
    revision: u64,
    tab: u64,
    geometry: u64,
    drawing: String,
    space: u64,
    viewport: Option<codec::Handle>,
}

pub(super) struct RoomFillCache {
    key: SourceKey,
    polygons: Arc<Vec<Vec<DVec3>>>,
}

impl MetrajUi {
    pub(super) fn room_fill_polygons(&self, tab: &DocumentTab) -> Arc<Vec<Vec<DVec3>>> {
        let Some(project) = self
            .project
            .as_ref()
            .filter(|_| self.room_fill_enabled && !tab.is_start)
        else {
            return Arc::default();
        };
        let key = SourceKey {
            project: project.id(),
            revision: project.summary_revision(),
            tab: tab.id,
            geometry: tab.scene.geometry_epoch,
            drawing: drawing_key(tab.current_path.as_deref(), tab.id),
            space: tab.scene.interaction_block_handle().value(),
            viewport: tab.scene.active_viewport,
        };
        let mut cache = self.room_fill_cache.borrow_mut();
        if let Some(cached) = cache.as_ref().filter(|cached| cached.key == key) {
            return cached.polygons.clone();
        }
        let handles: Vec<_> = project
            .bound_rooms(&key.drawing)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_, handle)| {
                u64::from_str_radix(&handle, 16)
                    .ok()
                    .map(codec::Handle::new)
            })
            .collect();
        let polygons: Vec<Vec<DVec3>> = tab
            .scene
            .visible_overlay_handles(&handles)
            .into_iter()
            .filter_map(|handle| {
                let entity = tab.scene.document.get_entity(handle)?;
                let closed = match entity {
                    codec::EntityType::LwPolyline(p) => p.is_closed && p.vertices.len() >= 3,
                    codec::EntityType::Polyline2D(p) => p.is_closed() && p.vertices.len() >= 3,
                    _ => false,
                };
                if !closed {
                    return None;
                }
                let curve = crate::entities::curve::entity_curve(entity)?;
                let points: Vec<_> = curve
                    .curve
                    .tessellate(8.)
                    .into_iter()
                    .map(|p| {
                        let [x, y, z] = curve.plane.point_at(p);
                        DVec3::new(x, y, z)
                    })
                    .collect();
                (points.len() >= 3 && points.iter().all(|point| point.is_finite()))
                    .then_some(points)
            })
            .collect();
        let polygons = Arc::new(polygons);
        *cache = Some(RoomFillCache {
            key,
            polygons: polygons.clone(),
        });
        polygons
    }
}

impl OpenCADStudio {
    pub(in crate::app) fn metraj_room_fill_overlay<'a>(
        &'a self,
        tab: &'a DocumentTab,
    ) -> Element<'a, Message> {
        let polygons = self.metraj.room_fill_polygons(tab);
        if polygons.is_empty() {
            return Space::new().width(Fill).height(Fill).into();
        }
        canvas(RoomFillCanvas {
            scene: &tab.scene,
            polygons,
            color: iced::Color::from_rgb8(
                self.metraj.room_fill_color[0],
                self.metraj.room_fill_color[1],
                self.metraj.room_fill_color[2],
            )
            .scale_alpha(1.0 - f32::from(self.metraj.room_fill_transparency) / 100.0),
        })
        .width(Fill)
        .height(Fill)
        .into()
    }
}

struct RoomFillCanvas<'a> {
    scene: &'a Scene,
    polygons: Arc<Vec<Vec<DVec3>>>,
    color: iced::Color,
}

#[derive(Clone)]
pub(crate) struct FrameKey {
    source: Arc<Vec<Vec<DVec3>>>,
    views: Vec<(Rectangle, Mat4, DVec3)>,
    color: iced::Color,
}

impl PartialEq for FrameKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source, &other.source)
            && self.views == other.views
            && self.color == other.color
    }
}

impl canvas::Program<Message> for RoomFillCanvas<'_> {
    type State = crate::ui::overlay::CachedOverlayLayer<FrameKey>;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = (bounds.width, bounds.height);
        let views = if self.scene.current_layout == "Model" {
            self.scene
                .model_tiles
                .borrow()
                .iter()
                .enumerate()
                .map(|(index, tile)| {
                    let rect = Rectangle {
                        x: tile.rect.x * size.0,
                        y: tile.rect.y * size.1,
                        width: tile.rect.width * size.0,
                        height: tile.rect.height * size.1,
                    };
                    let camera = if index == self.scene.active_model_tile.get() {
                        self.scene.camera.borrow().clone()
                    } else {
                        tile.camera.clone()
                    };
                    (camera, rect)
                })
                .collect::<Vec<_>>()
        } else {
            vec![self.scene.viewport_edit_frame(size).unwrap_or_else(|| {
                (
                    self.scene.camera.borrow().clone(),
                    Rectangle::with_size(bounds.size()),
                )
            })]
        };
        let color = self.color;
        let key = FrameKey {
            source: self.polygons.clone(),
            views: views
                .iter()
                .map(|(cam, rect)| (*rect, cam.view_proj_rte(*rect), cam.eye()))
                .collect(),
            color,
        };
        let frame_bounds = Rectangle::with_size(bounds.size());
        vec![state.draw(renderer, frame_bounds, &key, |frame| {
            for (camera, clip) in views {
                let Some(visible_clip) = clip.intersection(&frame_bounds) else {
                    continue;
                };
                frame.with_clip(visible_clip, |frame| {
                    for polygon in self.polygons.iter() {
                        let Some(points) = polygon
                            .iter()
                            .map(|point| {
                                camera
                                    .project(*point, clip)
                                    .map(|p| Point::new(clip.x + p.x, clip.y + p.y))
                            })
                            .collect::<Option<Vec<Point>>>()
                        else {
                            continue;
                        };
                        let path = canvas::Path::new(|builder| {
                            builder.move_to(points[0]);
                            for point in &points[1..] {
                                builder.line_to(*point);
                            }
                            builder.close();
                        });
                        frame.fill(&path, color);
                    }
                });
            }
        })]
    }
}
