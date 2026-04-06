use sparmos_engine::{
    egui::{self, Color32},
    helpers::animation::castaljau_point,
};

#[derive(Default)]
pub struct GuiState {
    pub bezier_toggled: bool,
    pub bezier_editor: BezierEditor,
}

pub struct BezierEditor {
    pub points: Vec<egui::Pos2>,
    pub selected: Option<usize>, // which point is being dragged
}

impl Default for BezierEditor {
    fn default() -> Self {
        Self {
            points: vec![egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)],
            selected: Default::default(),
        }
    }
}

impl BezierEditor {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Color32::GRAY)
            .inner_margin(10)
            .show(ui, |ui| {
                let height = 300.0;
                let size = egui::vec2(ui.available_size().x, height);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);

                let to_screen = egui::emath::RectTransform::from_to(
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    rect,
                );

                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let pointer_pos = to_screen.inverse().transform_pos(pointer_pos);

                    if response.double_clicked() {
                        let index = self
                            .points
                            .iter()
                            .position(|e| e.x > pointer_pos.x)
                            .unwrap();
                        self.points.insert(index, pointer_pos);
                    }

                    if response.drag_started() {
                        self.selected = self
                            .points
                            .iter()
                            .enumerate()
                            .find(|(i, p)| {
                                p.distance(pointer_pos) < 0.05
                                    && *i != 0
                                    && *i != self.points.len() - 1
                            })
                            .map(|(i, _)| i);
                    }

                    if response.dragged()
                        && let Some(i) = self.selected
                    {
                        self.points[i] = pointer_pos;
                    }

                    if response.drag_stopped() {
                        self.selected = None;
                    }
                }
                if self.points.is_empty() {
                    return;
                }

                let mut curve_points = Vec::new();

                for i in 0..=64 {
                    let t = i as f32 / 64.0;
                    let p = castaljau_point(&self.points, t);
                    curve_points.push(to_screen.transform_pos(p));
                }

                painter.add(egui::Shape::line(
                    curve_points,
                    egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                ));

                for p in self.points.iter() {
                    painter.circle_filled(to_screen.transform_pos(*p), 5.0, egui::Color32::WHITE);
                }

                // painter.line_segment(
                //     [
                //         to_screen.transform_pos(self.p0),
                //         to_screen.transform_pos(self.p1),
                //     ],
                //     egui::Stroke::new(1.0, egui::Color32::GRAY),
                // );
                //
                // painter.line_segment(
                //     [
                //         to_screen.transform_pos(self.p2),
                //         to_screen.transform_pos(self.p3),
                //     ],
                //     egui::Stroke::new(1.0, egui::Color32::GRAY),
                // );
            });
    }
}
