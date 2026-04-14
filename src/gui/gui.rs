use std::fmt;

use sparmos_engine::{
    cgmath::{Vector2, vec2},
    egui::{self, Color32},
    entity::{
        audio::{
            audio_handler::{AudioCommand, AudioTrigger},
            synth::{EnvelopeSegment, Sound, Waveform},
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum EnvelopeType {
    Attack,
    Decay,
    Refrain,
}
impl fmt::Display for EnvelopeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Attack => "Attack",
            Self::Decay => "Decay",
            Self::Refrain => "Refrain",
        };
        write!(f, "{}", s)
    }
}
pub struct EnvelopeHandle {
    kind: EnvelopeType,
    scale: f32,
    offset: f32,
    lerp: fn(&Sound, f32) -> (f32, f32),
    update_length: fn(&mut Sound, f32),
    get_length: fn(&Sound) -> f32,
    get_envelope_interp: fn(&mut Sound) -> &mut Interpolation,
}
impl EnvelopeHandle {
    pub fn attack(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Attack,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.attack.interpolation.lerp(t, false),
            update_length: |sound: &mut Sound, t: f32| {
                let new_length = sound.envelope.attack.length + t;
                sound.envelope.attack.length = round_to_step(new_length, 0.1);
            },
            get_length: |sound: &Sound| sound.envelope.attack.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.attack.interpolation,
        }
    }

    pub fn decay(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Decay,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.decay.interpolation.lerp(t, true),
            update_length: |sound: &mut Sound, t: f32| {
                let new_length = sound.envelope.decay.length + t;
                sound.envelope.decay.length = round_to_step(new_length, 0.1);
            },
            get_length: |sound: &Sound| sound.envelope.decay.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.decay.interpolation,
        }
    }

    pub fn release(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Refrain,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.refrain.interpolation.lerp(t, true),
            update_length: |sound: &mut Sound, t: f32| {
                let new_length = sound.envelope.refrain.length + t;
                sound.envelope.refrain.length = round_to_step(new_length, 0.1);
            },
            get_length: |sound: &Sound| sound.envelope.refrain.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.refrain.interpolation,
        }
    }

    pub fn lerp_envelope(&self, sound: &Sound, t: f32) -> egui::Pos2 {
        let (y, mut x) = (self.lerp)(sound, t);
        if x == 0.0 {
            x = self.offset + (t * self.scale);
        } else {
            x = self.offset + (x * self.scale);
        }
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

        if response.drag_started() && (handle.ratio - pointer.x).abs() < 0.03 {
            *selected = Some(handle.kind);
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
    pub stopping_sound: bool,
    pub envelope_edge_points: Vec<egui::Pos2>,
    pub handles: Vec<RatioHandle>,
    pub selected_point: Option<Ratio>,
    pub selected_envelope: Option<EnvelopeType>,
    pub attack_interp: Interpolation,
    pub decay_interp: Interpolation,
    pub release_interp: Interpolation,
    pub bezier_editor: BezierComponent,
}

impl WaveformVisualizer {
    pub fn ui(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut egui::Ui) {
        egui::Window::new("Sound editor")
            .resizable(true)
            .min_width(700.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui, |ui| {
                let sample_rate = engine.get_audio_handler().sample_rate;

                if self.play_sound || self.stopping_sound {
                    let dt_sec = dt.as_secs_f32();

                    self.sample_time += dt_sec * sample_rate;
                }

                let (rect, response) = ui.allocate_exact_size(
                    [ui.available_size().x, 400.0].into(),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);

                if let Some(selected_sound) = self.selected_sound
                    && let Some(sound) = self.sound.get_mut(selected_sound)
                {
                    let to_screen = egui::emath::RectTransform::from_to(
                        egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)),
                        rect,
                    );

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

                    //Audio bar tracking
                    if self.play_sound {
                        let attack_len = (attack.get_length)(sound);
                        let decay_len = (decay.get_length)(sound);

                        let total_len = attack_len + decay_len;

                        let t = self.sample_time / sample_rate;

                        if t >= total_len {
                            engine
                                .get_audio_handler()
                                .update_from_gamelogic(AudioCommand::Stop(
                                    AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                ));
                            self.play_sound = false;
                        }

                        let visible_x = if t < attack_len {
                            let local = t / attack_len; // 0 → 1

                            attack.offset + local * attack.scale
                        } else {
                            let local_time = t - attack_len;
                            let local = local_time / decay_len; // 0 → 1

                            decay.offset + local * decay.scale
                        };

                        let line_top = to_screen.transform_pos(egui::pos2(visible_x, 1.0));
                        let line_bottom = to_screen.transform_pos(egui::pos2(visible_x, -1.0));

                        painter.line_segment(
                            [line_top, line_bottom],
                            egui::Stroke::new(2.0, egui::Color32::GRAY),
                        );
                    }
                    if self.stopping_sound {
                        let length = (refrain.get_length)(sound);
                        let t = self.sample_time / sample_rate;
                        let local = t / length;

                        let visible_x = refrain.offset + local * refrain.scale;

                        let line_top = to_screen.transform_pos(egui::pos2(visible_x, 1.0));
                        let line_bottom = to_screen.transform_pos(egui::pos2(visible_x, -1.0));

                        println!("{:?}", visible_x);

                        painter.line_segment(
                            [line_top, line_bottom],
                            egui::Stroke::new(2.0, egui::Color32::GRAY),
                        );

                        if t >= length {
                            self.stopping_sound = false;
                        }
                    }
                    //Envelope handling
                    let envelopes = [attack, decay, refrain];
                    for envelope in envelopes.iter() {
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
                        if let Some(pointer_pos) = response.hover_pos() {
                            let pointer = to_screen.inverse().transform_pos(pointer_pos);
                            let bounds = envelope.get_bounds();
                            if pointer.x > bounds.x && pointer.x <= bounds.y {
                                self.selected_envelope = Some(envelope.kind)
                            }
                        }
                        CustomInterpolationEditor::ui(
                            ui,
                            &painter,
                            &response,
                            &to_screen,
                            &envelope,
                            sound,
                            &mut self.bezier_editor,
                            &mut changed,
                        );
                        // let painter = ui.painter_at(rect);

                        match envelope.kind {
                            EnvelopeType::Attack => {
                                painter.text(
                                    rect.left_bottom() + egui::vec2(5.0, -5.0),
                                    egui::Align2::LEFT_BOTTOM,
                                    format!("l: {:.1} secs", (envelope.get_length)(sound)),
                                    egui::FontId::proportional(16.0),
                                    egui::Color32::RED,
                                );
                            }
                            EnvelopeType::Decay => {
                                painter.text(
                                    rect.left_bottom()
                                        + egui::vec2(envelope.offset * rect.width(), -5.0),
                                    egui::Align2::LEFT_BOTTOM,
                                    format!("l: {:.1} secs", (envelope.get_length)(sound)),
                                    egui::FontId::proportional(16.0),
                                    egui::Color32::RED,
                                );
                            }
                            EnvelopeType::Refrain => {
                                painter.text(
                                    rect.left_bottom()
                                        + egui::vec2(envelope.offset * rect.width(), -5.0),
                                    egui::Align2::LEFT_BOTTOM,
                                    format!("l: {:.1} secs", (envelope.get_length)(sound)),
                                    egui::FontId::proportional(16.0),
                                    egui::Color32::RED,
                                );
                            }
                        }
                    }

                    if changed {
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Edit(
                                AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                sound.clone(),
                            ));
                    }

                    ui.horizontal_wrapped(|ui| {
                        for envelope in envelopes {
                            let interp = (envelope.get_envelope_interp)(sound);
                            ui.collapsing(format!("{} Interpolation", envelope.kind), |ui| {
                                ui.radio_value(interp, Interpolation::EaseOut, "Ease Out");
                                ui.radio_value(
                                    interp,
                                    Interpolation::EaseInEaseOut,
                                    "Ease In Ease Out",
                                );
                                ui.radio_value(interp, Interpolation::Linear, "Linear");

                                ui.radio_value(
                                    interp,
                                    Interpolation::Custom(
                                        [[0.0, 0.0].into(), [1.0, 1.0].into()].into(),
                                    ),
                                    "Custom Bezier",
                                )
                            });
                        }
                    });
                }
                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    let play_response = ui.button("Play Sound!");
                    let stop_response = ui.button("Stop Sound!");

                    let new_response = ui.button("New Sound!");

                    if play_response.clicked() {
                        self.start_sound(engine);
                    }
                    if stop_response.clicked() {
                        self.stop_sound(engine);
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

    fn stop_sound(&mut self, engine: &mut Engine) {
        engine
            .get_audio_handler()
            .update_from_gamelogic(AudioCommand::Stop(AudioTrigger::gamelogic(
                "waveform_visualizer_sound",
            )));
        self.sample_time = 0.0;
        self.play_sound = false;
        self.stopping_sound = true;

        println!("Stopped!");
    }
    fn start_sound(&mut self, engine: &mut Engine) {
        engine
            .get_audio_handler()
            .update_from_gamelogic(AudioCommand::Play(AudioTrigger::gamelogic(
                "waveform_visualizer_sound",
            )));
        println!("Played!");
        self.play_sound = true;
        self.stopping_sound = false;
        self.sample_time = 0.0;
    }
}

pub struct CustomInterpolationEditor;

impl CustomInterpolationEditor {
    pub fn ui(
        ui: &egui::Ui,
        painter: &egui::Painter,
        response: &egui::Response,
        to_screen: &egui::emath::RectTransform,
        envelope: &EnvelopeHandle,
        sound: &mut Sound,
        bezier_editor: &mut BezierComponent,
        changed: &mut bool,
    ) {
        if let Interpolation::Custom(pos) = (envelope.get_envelope_interp)(sound) {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let pointer_pos = to_screen.inverse().transform_pos(pointer_pos);

                if response.double_clicked() {
                    let index = pos
                        .iter()
                        .position(|e| {
                            envelope.offset + e.y / envelope.scale
                                > envelope.offset + pointer_pos.y / envelope.scale
                        })
                        .unwrap_or(pos.len());

                    let new_pos = egui::pos2(
                        (pointer_pos.x - envelope.offset) / envelope.scale,
                        pointer_pos.y,
                    );

                    pos.insert(index, new_pos);
                    *changed = true;
                }

                if response.drag_started() {
                    bezier_editor.selected = pos
                        .iter()
                        .enumerate()
                        .find(|(i, p)| {
                            let screen_pos =
                                egui::pos2(p.x * envelope.scale + envelope.offset, p.y);
                            screen_pos.distance(pointer_pos) < 0.05
                                && *i != 0
                                && *i != pos.len() - 1
                        })
                        .map(|(i, _)| i);
                }

                if response.dragged() {
                    if let Some(i) = bezier_editor.selected {
                        let new_pos = egui::pos2(
                            (pointer_pos.x - envelope.offset) / envelope.scale,
                            pointer_pos.y,
                        );
                        pos[i] = new_pos;
                        *changed = true;
                    }
                }

                if response.drag_stopped() {
                    bezier_editor.selected = None;
                }
            }

            for p in pos.iter() {
                let new_pos = egui::pos2(envelope.offset + p.x * envelope.scale, p.y);

                painter.circle_filled(to_screen.transform_pos(new_pos), 5.0, egui::Color32::WHITE);
            }

            ui.input(|i| {
                for e in &i.events {
                    if let egui::Event::MouseWheel { delta, .. } = e {
                        (envelope.update_length)(sound, (delta.y.abs() - 0.9) * delta.y);
                        *changed = true;
                    }
                }
            });
        }
    }
}
#[derive(Default)]
pub struct BezierComponent {
    pub selected: Option<usize>, // which point is being dragged
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
            });
    }
}

fn round_to_step(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}
