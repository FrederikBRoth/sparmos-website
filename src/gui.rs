use std::ops::{Deref, DerefMut};

use sparmos_engine::{
    cgmath::{Vector2, vec2},
    egui::{self, Color32, Painter, Pos2, Response, emath::RectTransform},
    entity::{
        audio::{
            audio_handler::{AudioCommand, AudioTrigger},
            synth::{Envelope, EnvelopeSegment, Sound, Waveform},
        },
        core::engine::Engine,
    },
    helpers::animation::{Interpolation, castaljau_point},
};

#[derive(Default)]
pub struct GuiState {
    pub bezier_toggled: bool,
    pub waveform_visualizer_toggled: bool,
    pub bezier_editor: BezierEditor,
    pub waveform_visualizer: WaveformVisualizer,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ratio {
    AttackDecayBoundary,
    DecayRefrainBoundary,
}

pub struct RatioHandle {
    pub kind: Ratio,
    pub ratio: f32,
}

#[derive(Debug)]
pub enum EnvelopeType {
    Attack,
    Decay,
    Refrain,
}
pub struct EnvelopeHandle {
    kind: EnvelopeType,
    scale: f32,
    offset: f32,
    lerp: fn(&Sound, f32) -> f32,
}
impl EnvelopeHandle {
    pub fn attack(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Attack,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.attack.interpolation.lerp(t),
        }
    }

    pub fn decay(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Decay,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| 1.0 - sound.envelope.decay.interpolation.lerp(t),
        }
    }

    pub fn release(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Refrain,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.refrain.interpolation.lerp(t),
        }
    }

    pub fn lerp_envelope(&self, sound: &Sound, t: f32) -> egui::Pos2 {
        let y = (self.lerp)(sound, t);
        let x = self.offset + (t * self.scale);
        [x, y].into()
    }

    pub fn get_bounds(&self) -> Vector2<f32> {
        let max = self.offset + self.scale;
        let min = self.offset;
        vec2(min, max)
    }
}
pub fn drag_handle(
    response: &egui::Response,
    to_screen: &egui::emath::RectTransform,
    selected: &mut Option<Ratio>,
    handle: &mut RatioHandle,
) -> bool {
    let mut changed = false;

    if let Some(pointer_pos) = response.interact_pointer_pos() {
        let pointer = to_screen.inverse().transform_pos(pointer_pos);

        if response.drag_started() {
            if (handle.ratio - pointer.x).abs() < 0.03 {
                *selected = Some(handle.kind);
            }
        }

        if response.dragged() && *selected == Some(handle.kind) {
            handle.ratio = pointer.x;
            changed = true;
        }

        if response.drag_stopped() && *selected == Some(handle.kind) {
            *selected = None;
        }
    }

    changed
}
pub fn draw_handle(
    to_screen: &egui::emath::RectTransform,
    painter: &egui::Painter,
    x: f32,
    selected: bool,
) {
    let color = if selected {
        egui::Color32::LIGHT_GRAY
    } else {
        egui::Color32::GRAY
    };

    let top = to_screen.transform_pos(egui::pos2(x, 1.0));
    let bottom = to_screen.transform_pos(egui::pos2(x, -1.0));

    painter.line_segment([top, bottom], egui::Stroke::new(2.0, color));
}
#[derive(Default)]
pub struct WaveformVisualizer {
    pub sample_time: f32,
    pub sound: Vec<Sound>,
    pub selected_sound: Option<usize>,
    pub play_sound: bool,
    pub envelope_edge_points: Vec<egui::Pos2>,
    pub handles: Vec<RatioHandle>,
    pub selected_point: Option<Ratio>,
    pub selected_envelope: Option<EnvelopeType>,
}

impl WaveformVisualizer {
    pub fn ui(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut egui::Ui) {
        egui::Window::new("Sound editor")
            .resizable(true)
            .min_width(700.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui, |ui| {
                let sample_rate = engine.get_audio_handler().sample_rate;

                if self.play_sound {
                    let dt_sec = dt.as_secs_f32();

                    // advance time in samples
                    self.sample_time += dt_sec * sample_rate;
                }

                let (rect, response) = ui.allocate_exact_size(
                    [ui.available_size().x, 400.0].into(),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);

                // 2. Draw the Playhead Bar
                if let Some(selected_sound) = self.selected_sound
                    && let Some(sound) = self.sound.get_mut(selected_sound)
                {
                    let duration = sound.duration();
                    let to_screen = egui::emath::RectTransform::from_to(
                        egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, -1.0)),
                        rect,
                    );

                    if self.play_sound {
                        // Calculate current X position in the 0.0..2.0 coordinate space
                        let current_x = self.sample_time / sample_rate;
                        if current_x >= duration && sound.envelope.sustain == 0.0 {
                            self.play_sound = false;
                        } else {
                            // Wrap the bar if it exceeds the 2.0s view, or stop it
                            let visible_x = current_x % duration;

                            let line_top = to_screen.transform_pos(egui::pos2(visible_x, 1.0));
                            let line_bottom = to_screen.transform_pos(egui::pos2(visible_x, -1.0));

                            painter.line_segment(
                                [line_top, line_bottom],
                                egui::Stroke::new(2.0, egui::Color32::GRAY),
                            );
                        }
                    }

                    let mut changed = false;

                    for handle in self.handles.iter_mut() {
                        changed |=
                            drag_handle(&response, &to_screen, &mut self.selected_point, handle);

                        draw_handle(
                            &to_screen,
                            &painter,
                            handle.ratio,
                            self.selected_point == Some(handle.kind),
                        );
                    }

                    let attack = EnvelopeHandle::attack(self.handles[0].ratio, 0.0);

                    let decay = EnvelopeHandle::decay(
                        self.handles[1].ratio - self.handles[0].ratio,
                        attack.scale,
                    );
                    let refrain =
                        EnvelopeHandle::release(1.0 - self.handles[1].ratio, self.handles[1].ratio);

                    let envelopes = [attack, decay, refrain];
                    for envelope in envelopes {
                        let mut curve_points = Vec::new();
                        for i in 0..=64 {
                            let var_name = 64.0;
                            let t = i as f32 / var_name;
                            let p = envelope.lerp_envelope(sound, t);
                            curve_points.push(to_screen.transform_pos(p));
                        }

                        painter.add(egui::Shape::line(
                            curve_points.clone(),
                            egui::Stroke::new(2.0, egui::Color32::RED),
                        ));
                        curve_points.clear();
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            let pointer = to_screen.inverse().transform_pos(pointer_pos);
                            let bounds = envelope.get_bounds();
                            if response.clicked() {
                                println!("{}", pointer);
                                if pointer.x > bounds.x && pointer.x < bounds.y {
                                    self.selected_envelope = Some(envelope.kind)
                                }
                            }
                        }
                        if let Some(selected) = self.selected_envelope.as_ref() {
                            let delta = ui.input(|i| {
                                i.events.iter().find_map(|e| match e {
                                    egui::Event::MouseWheel { delta, .. } => {
                                        println!("{:?}", selected);
                                        Some(*delta)
                                    }
                                    _ => None,
                                })
                            });
                        }
                        // ui.input(|i| {
                        //     i.events.iter().find(|e| match e {
                        //         egui::Event::MouseWheel { delta, .. } => {
                        //             if let Some(selected) = self.selected_envelope {
                        //                 println!("{:?}, scrolling :)", selected);
                        //                 false
                        //             } else {
                        //                 false
                        //             }
                        //         }
                        //         _ => None,
                        //     })
                        // });
                    }

                    if changed {
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Edit(
                                AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                sound.clone(),
                            ));
                    }
                }
                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    let play_response = ui.button("Play Sound!");
                    let stop_response = ui.button("Stop Sound!");

                    let new_response = ui.button("New Sound!");

                    if play_response.clicked() {
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Play(AudioTrigger::gamelogic(
                                "waveform_visualizer_sound",
                            )));
                        println!("Played!");
                        self.play_sound = true;
                        self.sample_time = 0.0;
                    }
                    if stop_response.clicked() {
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Stop(AudioTrigger::gamelogic(
                                "waveform_visualizer_sound",
                            )));
                        self.play_sound = false;

                        println!("Stopped!");
                    }
                    if new_response.clicked() {
                        let sound = Sound::new(
                            [1.0].into(),
                            440.0,
                            0.0,
                            Waveform::SineWave,
                            EnvelopeSegment {
                                length: 0.1,
                                interpolation: Interpolation::EaseInEaseOut,
                            },
                            EnvelopeSegment {
                                length: 1.98,
                                interpolation: Interpolation::EaseInEaseOut,
                            },
                            EnvelopeSegment {
                                length: 0.1,
                                ..Default::default()
                            },
                        );
                        self.sound.push(sound.clone());
                        self.selected_sound = Some(0);
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Add(
                                AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                sound,
                            ));
                    }
                });
                ui.allocate_space(ui.available_size());
            });
    }
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
