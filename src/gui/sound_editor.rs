use std::{fmt, vec};

use sparmos_engine::{
    audio::{
        audio_handler::{AudioCommand, AudioTrigger, hz_to_index, index_to_hz, index_to_key},
        midi::Midi,
        synth::{AudioState, EnvelopeSegment, Sound, Waveform},
    },
    cgmath::{Vector2, vec2},
    core::engine::Engine,
    egui::{self, Color32, RichText, Ui},
    systems::animation::Interpolation,
};

use crate::gui::player::PianoRoll;

#[derive(Default)]
pub struct GuiState {
    pub piano_roll_toggled: bool,
    pub sound_editor_toggled: bool,
    pub sound_editor: SoundEditor,
    pub piano_roll: PianoRoll,
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
    get_length: fn(&Sound) -> f32,
    get_envelope_interp: fn(&mut Sound) -> &mut Interpolation,
    get_envelope: fn(&mut Sound) -> &mut EnvelopeSegment,
}
impl EnvelopeHandle {
    pub fn attack(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Attack,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.attack.interpolation.lerp(t, false),
            get_length: |sound: &Sound| sound.envelope.attack.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.attack.interpolation,
            get_envelope: |sound: &mut Sound| &mut sound.envelope.attack,
        }
    }

    pub fn decay(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Decay,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.decay.interpolation.lerp(t, true),
            get_length: |sound: &Sound| sound.envelope.decay.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.decay.interpolation,
            get_envelope: |sound: &mut Sound| &mut sound.envelope.decay,
        }
    }

    pub fn release(scale: f32, offset: f32) -> Self {
        Self {
            kind: EnvelopeType::Refrain,
            scale,
            offset,
            lerp: |sound: &Sound, t: f32| sound.envelope.refrain.interpolation.lerp(t, true),
            get_length: |sound: &Sound| sound.envelope.refrain.length,
            get_envelope_interp: |sound: &mut Sound| &mut sound.envelope.refrain.interpolation,
            get_envelope: |sound: &mut Sound| &mut sound.envelope.refrain,
        }
    }

    pub fn lerp_envelope(&self, sound: &Sound, t: f32) -> egui::Pos2 {
        let (y, mut x) = (self.lerp)(sound, t);
        // hacky way to get the custom bezier x curve, while also getting the other more rigid
        // interpolations included that are bound to t. this is purely visual
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

pub struct EnvelopeContainer {
    attack: EnvelopeHandle,
    decay: EnvelopeHandle,
    refrain: EnvelopeHandle,
}

impl EnvelopeContainer {
    pub fn iter(&self) -> impl Iterator<Item = &EnvelopeHandle> {
        [&self.attack, &self.decay, &self.refrain].into_iter()
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
pub struct SoundEditor {
    pub sample_time: f32,
    pub sounds: Vec<Sound>,
    pub selected_sound: Option<usize>,
    pub audio_state: AudioState,
    pub envelope_edge_points: Vec<egui::Pos2>,
    pub handles: Vec<RatioHandle>,
    pub selected_point: Option<Ratio>,
    pub selected_envelope: Option<EnvelopeType>,
    pub attack_interp: Interpolation,
    pub decay_interp: Interpolation,
    pub release_interp: Interpolation,
    pub bezier_editor: BezierComponent,
}

impl SoundEditor {
    pub fn ui(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut egui::Ui) {
        egui::Window::new("Sound editor")
            .resizable(true)
            .min_width(1000.0)
            .min_height(500.0)
            // .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui, |ui| {
                let attack = EnvelopeHandle::attack(self.handles[0].ratio, 0.0);

                let decay = EnvelopeHandle::decay(
                    self.handles[1].ratio - self.handles[0].ratio,
                    attack.scale,
                );
                let refrain =
                    EnvelopeHandle::release(1.0 - self.handles[1].ratio, self.handles[1].ratio);

                let envelopes = EnvelopeContainer {
                    attack,
                    decay,
                    refrain,
                };

                let mut changed = false;

                if let Some(selected_sound) = self.selected_sound
                    && let Some(sound) = self.sounds.get_mut(selected_sound)
                {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let (rect, response) = ui.allocate_exact_size(
                                [ui.available_size().x - 200.0, 400.0].into(),
                                egui::Sense::click_and_drag(),
                            );

                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 0.0, Color32::BLACK);

                            let to_screen = egui::emath::RectTransform::from_to(
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 1.0),
                                    egui::pos2(1.0, 0.0),
                                ),
                                rect,
                            );

                            for handle in self.handles.iter_mut() {
                                changed |= drag_handle(
                                    &response,
                                    &to_screen,
                                    &mut self.selected_point,
                                    handle,
                                );

                                draw_handle(
                                    &to_screen,
                                    &painter,
                                    handle.ratio,
                                    self.selected_point == Some(handle.kind),
                                );
                            }

                            //Audio bar tracking
                            let sample_rate = engine.get_audio_handler().sample_rate;

                            match self.audio_state {
                                AudioState::Playing => {
                                    let dt_sec = dt.as_secs_f32();

                                    self.sample_time += dt_sec * sample_rate;

                                    let attack_len = (envelopes.attack.get_length)(sound);
                                    let decay_len = (envelopes.decay.get_length)(sound);

                                    let total_len = attack_len + decay_len;

                                    let t = self.sample_time / sample_rate;

                                    if t >= total_len {
                                        engine.get_audio_handler().update_from_gamelogic(
                                            AudioCommand::Stop(AudioTrigger::gamelogic(
                                                "waveform_visualizer_sound",
                                            )),
                                        );
                                        self.audio_state = AudioState::Stopped;
                                    }

                                    let visible_x = if t < attack_len {
                                        let local = t / attack_len; // 0 → 1

                                        envelopes.attack.offset + local * envelopes.attack.scale
                                    } else {
                                        let local_time = t - attack_len;
                                        let local = local_time / decay_len; // 0 → 1

                                        envelopes.decay.offset + local * envelopes.decay.scale
                                    };

                                    let line_top =
                                        to_screen.transform_pos(egui::pos2(visible_x, 1.0));
                                    let line_bottom =
                                        to_screen.transform_pos(egui::pos2(visible_x, -1.0));

                                    painter.line_segment(
                                        [line_top, line_bottom],
                                        egui::Stroke::new(2.0, egui::Color32::GRAY),
                                    );
                                }
                                AudioState::Stopping => {
                                    let dt_sec = dt.as_secs_f32();
                                    self.sample_time += dt_sec * sample_rate;

                                    let length = (envelopes.refrain.get_length)(sound);
                                    let t = self.sample_time / sample_rate;
                                    let local = t / length;

                                    let visible_x =
                                        envelopes.refrain.offset + local * envelopes.refrain.scale;

                                    let line_top =
                                        to_screen.transform_pos(egui::pos2(visible_x, 1.0));
                                    let line_bottom =
                                        to_screen.transform_pos(egui::pos2(visible_x, -1.0));

                                    println!("{:?}", visible_x);

                                    painter.line_segment(
                                        [line_top, line_bottom],
                                        egui::Stroke::new(2.0, egui::Color32::GRAY),
                                    );

                                    if t >= length {
                                        self.audio_state = AudioState::Stopped;
                                    }
                                }
                                AudioState::Stopped => {}
                            }
                            //Envelope handling
                            for envelope in envelopes.iter() {
                                let mut curve_points = Vec::new();
                                for i in 0..=64 {
                                    let var_name = 64.0;
                                    let t = i as f32 / var_name;
                                    let p = envelope.lerp_envelope(sound, t);
                                    curve_points.push(to_screen.transform_pos(p));
                                }

                                // red offset
                                painter.add(egui::Shape::line(
                                    curve_points
                                        .iter()
                                        .map(|p| *p + egui::vec2(-1.0, 0.0))
                                        .collect(),
                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 0, 0)),
                                ));

                                // blue offse
                                painter.add(egui::Shape::line(
                                    curve_points
                                        .iter()
                                        .map(|p| *p + egui::vec2(1.5, 0.0))
                                        .collect(),
                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 100, 255)),
                                ));

                                // main white line
                                painter.add(egui::Shape::line(
                                    curve_points.clone(),
                                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                                ));

                                curve_points.clear();
                                if let Some(pointer_pos) = response.hover_pos()
                                    && !response.dragged()
                                {
                                    let pointer = to_screen.inverse().transform_pos(pointer_pos);
                                    let bounds = envelope.get_bounds();
                                    if pointer.x > bounds.x && pointer.x <= bounds.y {
                                        self.selected_envelope = Some(envelope.kind)
                                    }
                                }
                                if let Some(selected) = self.selected_envelope.as_ref()
                                    && envelope.kind == *selected
                                {
                                    CustomInterpolationEditor::ui(
                                        &painter,
                                        &response,
                                        &to_screen,
                                        envelope,
                                        sound,
                                        &mut self.bezier_editor,
                                        &mut changed,
                                    );
                                }
                            }

                            ui.allocate_ui([ui.available_size().x - 200.0, 20.0].into(), |ui| {
                                ui.columns(3, |columns| {
                                    columns[0].horizontal(|ui| {
                                        let attack = (envelopes.attack.get_envelope)(sound);

                                        ui.label("Attack Length:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut attack.length)
                                                    .range(0..=90)
                                                    .speed(0.01),
                                            )
                                            .changed()
                                        {
                                            changed |= true;
                                        };
                                    });
                                    columns[1].with_layout(
                                        egui::Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            ui.horizontal(|ui| {
                                                let decay = (envelopes.decay.get_envelope)(sound);
                                                ui.label("Decay Length:");
                                                if ui
                                                    .add(
                                                        egui::DragValue::new(&mut decay.length)
                                                            .range(0..=90)
                                                            .speed(0.01),
                                                    )
                                                    .changed()
                                                {
                                                    changed |= true;
                                                };
                                            })
                                        },
                                    );

                                    columns[2].with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.horizontal(|ui| {
                                                let refrain =
                                                    (envelopes.refrain.get_envelope)(sound);
                                                if ui
                                                    .add(
                                                        egui::DragValue::new(&mut refrain.length)
                                                            .range(0..=90)
                                                            .speed(0.01),
                                                    )
                                                    .changed()
                                                {
                                                    changed |= true;
                                                };

                                                ui.label("Refrain Length:");
                                            })
                                        },
                                    );
                                });
                            });
                        });
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(200.0, ui.available_height() * 3.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.collapsing("General", |ui| {
                                            let mut index = hz_to_index(sound.freq);
                                            if ui
                                                .add(egui::Slider::new(&mut index, 0..=90))
                                                .changed()
                                            {
                                                sound.freq = index_to_hz(index);

                                                changed |= true;
                                            };

                                            let key_label = index_to_key(index);
                                            ui.label(key_label);
                                        });
                                        ui.collapsing("Interpolation", |ui| {
                                            for envelope in envelopes.iter() {
                                                let interp = (envelope.get_envelope_interp)(sound);

                                                ui.collapsing(
                                                    format!("{:?}", envelope.kind),
                                                    |ui| {
                                                        ui.radio_value(
                                                            interp,
                                                            Interpolation::EaseOut,
                                                            "Ease Out",
                                                        );
                                                        ui.radio_value(
                                                            interp,
                                                            Interpolation::EaseInEaseOut,
                                                            "Ease In Ease Out",
                                                        );
                                                        ui.radio_value(
                                                            interp,
                                                            Interpolation::Linear,
                                                            "Linear",
                                                        );

                                                        let starting_points: Vec<egui::Pos2> =
                                                            match envelope.kind {
                                                                EnvelopeType::Attack => {
                                                                    vec![
                                                                        [0.0, 0.0].into(),
                                                                        [1.0, 1.0].into(),
                                                                    ]
                                                                }
                                                                EnvelopeType::Refrain
                                                                | EnvelopeType::Decay => {
                                                                    vec![
                                                                        [1.0, 0.0].into(),
                                                                        [0.0, 1.0].into(),
                                                                    ]
                                                                }
                                                            };

                                                        ui.radio_value(
                                                            interp,
                                                            Interpolation::Custom(starting_points),
                                                            "Custom Bezier",
                                                        );
                                                    },
                                                );
                                            }
                                        });

                                        changed |= harmonic_sliders(&mut sound.harmonics, ui);
                                    },
                                );
                            });
                    });
                    if changed {
                        sound.phases = sound.harmonics.clone();

                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Edit(
                                AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                sound.clone(),
                            ));
                    }
                };
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
                        self.sounds.push(sound.clone());
                        self.selected_sound = Some(0);
                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::Add(
                                AudioTrigger::gamelogic("waveform_visualizer_sound"),
                                sound,
                            ));
                    }
                });

                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, sound) in self.sounds.iter_mut().enumerate() {
                            sound_preview(i, &mut self.selected_sound, sound, engine, ui);
                        }
                    });
            });
    }

    fn stop_sound(&mut self, engine: &mut Engine) {
        engine
            .get_audio_handler()
            .update_from_gamelogic(AudioCommand::Stop(AudioTrigger::gamelogic(
                "waveform_visualizer_sound",
            )));
        self.sample_time = 0.0;
        self.audio_state = AudioState::Stopping;

        println!("Stopped!");
    }
    fn start_sound(&mut self, engine: &mut Engine) {
        engine
            .get_audio_handler()
            .update_from_gamelogic(AudioCommand::Play(AudioTrigger::gamelogic(
                "waveform_visualizer_sound",
            )));
        println!("Played!");
        self.audio_state = AudioState::Playing;
        self.sample_time = 0.0;
    }
}

pub fn harmonic_sliders(harmonics: &mut Vec<f32>, ui: &mut Ui) -> bool {
    let mut changed = false;
    ui.collapsing("Harmonics", |ui| {
        for harmonic in harmonics.iter_mut() {
            if ui.add(egui::Slider::new(harmonic, 0.0..=1.0)).dragged() {
                changed |= true;
            };
        }
        if ui.button("Add Harmonic").clicked() {
            harmonics.push(1.0);
            changed |= true;
        }
        if ui
            .button(RichText::new("Remove Harmonic").color(Color32::RED))
            .clicked()
        {
            harmonics.pop();
            changed |= true;
        }
    });
    changed
}
fn build_curve_points(
    interpolation: &Interpolation,
    offset: f32,
    width: f32,
    flip: bool,
    to_screen: &egui::emath::RectTransform,
) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    let steps = 16.0;

    for i in 0..=16 {
        let t = i as f32 / steps;
        let (y, mut x) = interpolation.lerp(t, flip);

        //hacky solution to keep proper custom bezier graph structure
        //is purely visual
        x = if x == 0.0 {
            offset + (t * width)
        } else {
            offset + (x * width)
        };

        let point = egui::pos2(x, y);
        points.push(to_screen.transform_pos(point));
    }

    points
}
pub fn sound_preview(
    index: usize,
    selected_sound: &mut Option<usize>,
    sound: &mut Sound,
    engine: &mut Engine,
    ui: &mut Ui,
) {
    let fill = if let Some(selected_index) = selected_sound
        && *selected_index == index
    {
        ui.visuals().selection.bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };

    egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.horizontal(|ui| ui.label(sound.to_string()));
            ui.horizontal(|ui| {
                ui.label("Envelope layout");

                let (rect, _) = ui.allocate_exact_size([200.0, 20.0].into(), egui::Sense::hover());

                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);

                let to_screen = egui::emath::RectTransform::from_to(
                    egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)),
                    rect,
                );

                let mut curve_points = Vec::new();

                curve_points.extend(build_curve_points(
                    &sound.envelope.attack.interpolation,
                    0.0,
                    0.4,
                    false,
                    &to_screen,
                ));

                curve_points.extend(build_curve_points(
                    &sound.envelope.decay.interpolation,
                    0.4,
                    0.4,
                    true,
                    &to_screen,
                ));

                painter.add(egui::Shape::line(
                    curve_points,
                    egui::Stroke::new(2.0, egui::Color32::RED),
                ));

                let refrain_points = build_curve_points(
                    &sound.envelope.refrain.interpolation,
                    0.8,
                    0.2,
                    true,
                    &to_screen,
                );

                painter.add(egui::Shape::line(
                    refrain_points,
                    egui::Stroke::new(2.0, egui::Color32::RED),
                ));
            });
            let response = ui.interact(ui.max_rect(), ui.id().with(index), egui::Sense::click());
            if response.clicked() {
                *selected_sound = Some(index);
                sound.phases = sound.harmonics.clone();
                engine
                    .get_audio_handler()
                    .update_from_gamelogic(AudioCommand::Edit(
                        AudioTrigger::gamelogic("waveform_visualizer_sound"),
                        sound.clone(),
                    ));
            }
            ui.allocate_space([ui.available_size().x, 0.0].into());
        });
}

pub struct CustomInterpolationEditor;

impl CustomInterpolationEditor {
    pub fn ui(
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
        }
    }
}
#[derive(Default)]
pub struct BezierComponent {
    pub selected: Option<usize>, // which point is being dragged
}
