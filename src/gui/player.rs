use std::collections::HashSet;

use dot_vox::Color;
use sparmos_engine::{
    egui::{self, Color32, Frame, Id, Painter, Rect, Response, Sense, Ui, emath::OrderedFloat},
    entity::{
        audio::{
            audio_handler::{AudioCommand, AudioTrigger},
            synth::{AudioState, Sound},
        },
        core::engine::Engine,
    },
};

pub const TICK_FACTOR: f32 = 1000.0;

pub fn f32_to_u32(f: f32) -> u32 {
    (f * TICK_FACTOR) as u32
}

pub fn u32_to_f32(u: u32) -> f32 {
    u as f32 / TICK_FACTOR
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Note {
    name: String,
    duration: u32,
    start: u32,
    layer: usize,
}

impl Note {
    pub fn new(name: &str, duration: f32, start: f32, layer: usize) -> Self {
        Self {
            name: name.to_string(),
            duration: f32_to_u32(duration),
            start: f32_to_u32(start),
            layer,
        }
    }

    pub fn should_play(&self, x: f32) -> bool {
        let x32 = f32_to_u32(x);
        x32 >= self.start && self.start + self.duration >= x32
    }
}

pub struct PianoRoll {
    layers: Vec<Vec<Note>>,
    sound_count: usize,
    selected: Option<(usize, usize, Note, egui::Vec2)>,
    audio_state: AudioState,
    sample_time: f32,
    duration: f32,
    playing_notes: HashSet<Note>,
}

impl Default for PianoRoll {
    fn default() -> Self {
        Self {
            layers: [
                vec![
                    Note::new("test", 25.0, 0.0, 0),
                    Note::new("test2!", 60.0, 20.0, 0),
                ],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            ]
            .into(),
            sound_count: 0,
            selected: None,
            audio_state: AudioState::Stopped,
            sample_time: 0.0,
            duration: 20.0,
            playing_notes: HashSet::new(),
        }
    }
}

impl PianoRoll {
    pub fn ui(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut Ui) {
        egui::Window::new("Sound Player")
            .resizable(true)
            .min_width(1000.0)
            .min_height(500.0)
            .show(ui, |ui| {
                let sample_rate = engine.get_audio_handler().sample_rate;

                let piano_roll_bar_height = 30.0;
                let (rect, response, painter) = self.draw_piano_roll(ui, piano_roll_bar_height);

                let player_head = match self.audio_state {
                    AudioState::Playing => {
                        let dt_sec = dt.as_secs_f32();
                        self.sample_time += dt_sec * sample_rate;

                        let t = self.sample_time / sample_rate; // seconds
                        let local = (t / self.duration) * rect.width();

                        let line_top = egui::pos2(local + rect.left(), rect.top());
                        let line_bottom = egui::pos2(local + rect.left(), rect.bottom());

                        Some((line_top, line_bottom))
                    }
                    _ => None,
                };

                let mut to_be_deleted = Vec::new();
                for (row, bar) in self.layers.clone().into_iter().enumerate() {
                    for (col, sound) in bar.iter().enumerate() {
                        if let Some((_, player_head_x)) = player_head {
                            let x = player_head_x.x - rect.left();
                            if sound.should_play(x) {
                                if !self.playing_notes.contains(sound) {
                                    println!("{} has started playing!!!", sound.name);
                                    engine.get_audio_handler().update_from_gamelogic(
                                        AudioCommand::ForcePlay(AudioTrigger::gamelogic(&format!(
                                            "{}",
                                            row
                                        ))),
                                    );

                                    self.playing_notes.insert(sound.clone());
                                }
                            }
                        }
                        let mut r = create_sound_block(row, piano_roll_bar_height, sound);
                        r = r.translate(rect.min.to_vec2());

                        let mut color = Color32::RED;
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            if response.drag_started() && r.contains(pointer_pos) {
                                self.selected = Some((
                                    row,
                                    col,
                                    sound.clone(),
                                    [pointer_pos.x - r.left(), pointer_pos.y - r.top()].into(),
                                ));
                                to_be_deleted.push((row, col));
                            }
                        };
                        if let Some(pointer_pos) = response.hover_pos() {
                            if response.hovered() && r.contains(pointer_pos) {
                                color = Color32::LIGHT_RED;
                            }
                        }
                        ui.painter().rect_filled(r, 0.0, color);
                    }
                }

                while let Some((x, y)) = to_be_deleted.pop() {
                    self.layers[x].remove(y);
                }

                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    if response.dragged()
                        && let Some((_, _, note, offset)) = self.selected.as_ref()
                    {
                        let mut r = create_sound_block_from_cursor(
                            pointer_pos,
                            note,
                            piano_roll_bar_height,
                            offset,
                        );

                        ui.painter().rect_filled(r, 0.0, Color32::LIGHT_RED);
                    }
                    if response.drag_stopped()
                        && let Some((x, y, note, offset)) = self.selected.as_mut()
                    {
                        let y = layer_from_cursor(pointer_pos, rect, piano_roll_bar_height);
                        note.start = f32_to_u32(pointer_pos.x - rect.left() - offset.x);
                        note.layer = y;
                        self.layers[y].push(note.clone());
                        self.selected = None;
                    }
                    if response.double_clicked() {
                        let y = layer_from_cursor(pointer_pos, rect, piano_roll_bar_height);
                        let x = pointer_pos.x - rect.left();
                        let note = Note::new("fucker", 60.0, x, y);
                        self.layers[y].push(note);
                    }
                }

                if let Some((line_top, line_bottom)) = player_head {
                    painter.line_segment(
                        [line_top, line_bottom],
                        egui::Stroke::new(2.0, egui::Color32::GRAY),
                    );
                    self.playing_notes.retain(|note| {
                        if !note.should_play(line_top.x - rect.left()) {
                            println!("{} is stopped playing :(", note.name);
                            engine
                                .get_audio_handler()
                                .update_from_gamelogic(AudioCommand::Stop(
                                    AudioTrigger::gamelogic(&format!("{}", note.layer)),
                                ));

                            false
                        } else {
                            true
                        }
                    });
                }

                if ui.button("Start Track").clicked() {
                    self.audio_state = AudioState::Playing;
                    self.sample_time = 0.0;
                    self.playing_notes.clear();
                }
                if ui.button("Stop Track").clicked() {
                    self.audio_state = AudioState::Stopped;
                }

                ui.allocate_space(ui.available_size());
            });
    }
    fn draw_piano_roll(&mut self, ui: &mut Ui, roll_height: f32) -> (Rect, Response, Painter) {
        let rect_height = roll_height * self.layers.len() as f32;
        let (rect, sense) =
            ui.allocate_exact_size([1000.0, rect_height].into(), Sense::click_and_drag());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let spacing = roll_height;

        for i in 0..self.layers.len() {
            let y = rect.top() + i as f32 * spacing;
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, Color32::from_gray(40)),
            );
        }

        (rect, sense, painter)
    }
}
fn get_overlapping_notes(roll: &Vec<Vec<Note>>, x: f32, playing_notes: &mut HashSet<Note>) {}

fn layer_from_cursor(cursor: egui::Pos2, rect: egui::Rect, piano_roll_bar_height: f32) -> usize {
    let local_y = (cursor.y - rect.top()).clamp(0.0, rect.height());
    (local_y / piano_roll_bar_height).floor() as usize
}
//
fn create_sound_block_from_cursor(
    cursor: egui::Pos2,
    sound_element: &Note,
    piano_roll_bar_height: f32,
    offset: &egui::Vec2,
) -> Rect {
    let top_left = egui::pos2(cursor.x - offset.x, cursor.y - offset.y);
    let bottom_right = egui::pos2(
        top_left.x + u32_to_f32(sound_element.duration),
        top_left.y + piano_roll_bar_height,
    );
    Rect::from_two_pos(top_left, bottom_right)
}
fn create_sound_block(layer_index: usize, bar_height: f32, sound_element: &Note) -> Rect {
    let layer_top_point = layer_index as f32 * bar_height;
    let layer_bottom_point = layer_index as f32 * bar_height + bar_height;

    Rect::from_two_pos(
        [u32_to_f32(sound_element.start), layer_top_point].into(),
        [
            u32_to_f32(sound_element.start + sound_element.duration),
            layer_bottom_point,
        ]
        .into(),
    )
}
