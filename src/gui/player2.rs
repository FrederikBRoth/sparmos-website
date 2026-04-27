use std::{collections::HashSet, vec};

use dot_vox::Color;
use sparmos_engine::{
    audio::{
        audio_handler::{AudioCommand, AudioTrigger},
        midi::{Midi, MidiNote},
        synth::{AudioState, Sound},
    },
    core::engine::{self, Engine},
    egui::{
        self, Align2, Color32, FontId, Frame, Id, Painter, Rect, Response, Sense, TextStyle, Ui,
        emath::OrderedFloat, pos2,
    },
};
pub const OCTAVES: usize = 7;
pub const OCTAVE_OFFSET: usize = 3;

pub const SCALE: f32 = 10.0;
pub const PIANO_ROLL_SIZE: usize = 88;

pub fn f32_to_u32(f: f32) -> u32 {
    (f * SCALE) as u32
}

pub fn u32_to_f32(u: u32) -> f32 {
    u as f32 / SCALE
}
pub struct Key {
    index: usize,
    rect: Rect,
    border_radius: f32,
    flat: bool,
}

fn new_note(name: &str, duration: f32, start: f32, key: usize) -> MidiNote {
    MidiNote {
        name: name.to_string(),
        length: f32_to_u32(duration),
        start: f32_to_u32(start),
        key,
    }
}
fn should_play(note: &MidiNote, x: f32) -> bool {
    let x32 = f32_to_u32(x);
    x32 >= note.start && note.start + note.length >= x32
}

pub struct PianoRoll {
    pub midis: Vec<Midi>,
    keys: Vec<Vec<MidiNote>>,
    selected: Option<(usize, usize, MidiNote, egui::Vec2)>,
    audio_state: AudioState,
    sample_time: f32,
    tempo: f32,
    ticks_per_quarter: f32,
    duration_ticks: f32,
    duration_seconds: f32,
    playing_notes: HashSet<MidiNote>,
    hovered_note: Option<usize>,
    scroll_x: f32,
}

impl Default for PianoRoll {
    fn default() -> Self {
        Self {
            midis: vec![],
            keys: vec![Vec::new(); OCTAVES * 12],
            selected: None,
            audio_state: AudioState::Stopped,
            sample_time: 0.0,
            tempo: 500000.0,
            ticks_per_quarter: 480.0,
            duration_ticks: 5000000.0,
            duration_seconds: 60.0,
            playing_notes: HashSet::new(),
            hovered_note: None,
            scroll_x: 0.0,
        }
    }
}

impl PianoRoll {
    pub fn create_track_from_midi(&mut self, index: usize, channel: u8) {
        for key in self.keys.iter_mut() {
            key.clear();
        }
        if let Some(midi) = self.midis.get(index) {
            for mut note in midi.channels[&channel].iter().cloned() {
                note.key -= 20 + 1 + OCTAVE_OFFSET;

                let index = self.keys.len() - 1 - note.key;

                self.keys[index].push(note);
            }
            self.tempo = midi.tempos[1] as f32;
            self.ticks_per_quarter = midi.ticks_per_quarter as f32;
            let seconds_per_tick = (self.tempo / 1_000_000.0) / self.ticks_per_quarter;
            self.duration_ticks = midi.length as f32;
            self.duration_seconds = midi.length as f32 * seconds_per_tick;
        }
    }

    pub fn ui(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(500.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let piano_roll_bar_height = 15.0;

                    self.draw_piano_sidebar(ui, piano_roll_bar_height, engine);

                    egui::ScrollArea::horizontal()
                        .auto_shrink([false, false])
                        .max_width(self.duration_ticks / SCALE)
                        .scroll_offset(egui::vec2(self.scroll_x, 0.0))
                        .show(ui, |ui| {
                            let (rect, response, painter) = self.draw_piano_roll(
                                ui,
                                piano_roll_bar_height,
                                self.duration_ticks / SCALE,
                            );
                            let player_head = match self.audio_state {
                                AudioState::Playing => {
                                    let dt_sec = dt.as_secs_f32();
                                    let ticks_per_second =
                                        self.ticks_per_quarter * (1_000_000.0 / self.tempo);
                                    self.sample_time += dt_sec; // now in seconds

                                    let progress = self.sample_time * ticks_per_second;
                                    let x = rect.left() + progress / SCALE;

                                    // --- smooth scroll ---
                                    let view_width = ui.clip_rect().width();
                                    let target_scroll =
                                        ((progress / SCALE) - view_width * 0.5).max(0.0);

                                    let smoothing = 10.0;
                                    let lerp_factor = 1.0 - (-smoothing * dt_sec).exp();

                                    self.scroll_x += (target_scroll - self.scroll_x) * lerp_factor;

                                    // ----------------------
                                    let line_top = egui::pos2(x, rect.top());
                                    let line_bottom = egui::pos2(x, rect.bottom());

                                    let playhead_rect =
                                        egui::Rect::from_min_max(line_top, line_bottom);
                                    Some((line_top, line_bottom))
                                }
                                _ => None,
                            };

                            let mut to_be_deleted = Vec::new();
                            for (row, bar) in self.keys.clone().into_iter().enumerate() {
                                for (col, sound) in bar.iter().enumerate() {
                                    if let Some((_, player_head_x)) = player_head {
                                        let x = player_head_x.x - rect.left();
                                        if should_play(sound, x) {
                                            if !self.playing_notes.contains(sound) {
                                                println!(
                                                    "{:?} layer: {} has started playing!!!",
                                                    sound,
                                                    self.keys.len() - 1 - row + OCTAVE_OFFSET
                                                );
                                                engine.get_audio_handler().update_from_gamelogic(
                                                    AudioCommand::ForcePlay(
                                                        AudioTrigger::gamelogic(&format!(
                                                            "{}",
                                                            self.keys.len() - 1 - row
                                                                + OCTAVE_OFFSET
                                                        )),
                                                    ),
                                                );

                                                self.playing_notes.insert(sound.clone());
                                            }
                                        }
                                    }
                                    let mut r =
                                        create_sound_block(row, piano_roll_bar_height, sound);
                                    r = r.translate(rect.min.to_vec2());

                                    let mut color = Color32::RED;
                                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                                        if response.drag_started()
                                            && r.contains(pointer_pos)
                                            && ui.rect_contains_pointer(r)
                                        {
                                            self.selected = Some((
                                                row,
                                                col,
                                                sound.clone(),
                                                [pointer_pos.x - r.left(), pointer_pos.y - r.top()]
                                                    .into(),
                                            ));
                                            to_be_deleted.push((row, col));
                                        }
                                    };
                                    if let Some(pointer_pos) = response.hover_pos() {
                                        if response.hovered()
                                            && r.contains(pointer_pos)
                                            && ui.rect_contains_pointer(r)
                                        {
                                            color = Color32::LIGHT_RED;
                                        }
                                    }
                                    ui.painter().rect_filled(r, 2.0, color);
                                }
                            }

                            while let Some((x, y)) = to_be_deleted.pop() {
                                self.keys[x].remove(y);
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

                                    ui.painter().rect_filled(r, 2.0, Color32::LIGHT_RED);
                                }
                                if response.drag_stopped()
                                    && let Some((x, y, note, offset)) = self.selected.as_mut()
                                {
                                    let y =
                                        layer_from_cursor(pointer_pos, rect, piano_roll_bar_height);
                                    note.start = f32_to_u32(pointer_pos.x - rect.left() - offset.x);
                                    note.key = self.keys.len() - 1 - y;
                                    println!("{}", y);
                                    self.keys[y].push(note.clone());
                                    self.selected = None;
                                }
                                if response.double_clicked() {
                                    let y =
                                        layer_from_cursor(pointer_pos, rect, piano_roll_bar_height);
                                    let x = pointer_pos.x - rect.left();
                                    let note = new_note("fucker", 60.0, x, self.keys.len() - 1 - y);
                                    self.keys[y].push(note);
                                }
                            }

                            if let Some((line_top, line_bottom)) = player_head {
                                painter.line_segment(
                                    [line_top, line_bottom],
                                    egui::Stroke::new(2.0, egui::Color32::GRAY),
                                );
                                self.playing_notes.retain(|note| {
                                    if !should_play(note, line_top.x - rect.left()) {
                                        println!("{:?} is stopped playing :(", note);
                                        engine.get_audio_handler().update_from_gamelogic(
                                            AudioCommand::Stop(AudioTrigger::gamelogic(&format!(
                                                "{}",
                                                note.key + OCTAVE_OFFSET
                                            ))),
                                        );

                                        false
                                    } else {
                                        true
                                    }
                                });
                            }
                        });
                });
            });

        if ui.button("Start Track").clicked() {
            self.audio_state = AudioState::Playing;
            self.sample_time = 0.0;
            self.playing_notes.clear();
        }
        if ui.button("Stop Track").clicked() {
            self.audio_state = AudioState::Stopped;
        }

        ui.allocate_space(ui.available_size());
    }
    fn draw_piano_sidebar(
        &mut self,
        ui: &mut Ui,
        roll_height: f32,
        engine: &mut Engine,
    ) -> (Rect, Response, Painter) {
        let rect_height = roll_height * self.keys.len() as f32;

        let (rect, response) =
            ui.allocate_exact_size([100.0, rect_height].into(), Sense::click_and_drag());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);

        let pointer_pressed = ui.input(|i| i.pointer.primary_down());
        let pointer_released = ui.input(|i| i.pointer.primary_released());

        let mut keys: Vec<Key> = vec![];
        let (blacks, whites) = get_black_and_white_keys(PIANO_ROLL_SIZE);
        let white_height = rect_height / whites.len() as f32;
        let mut black_hovered = false;
        for key in blacks.iter() {
            let note_index = key % 12;

            // Find which white key this black key belongs after
            let white_index = match note_index {
                1 => 0,  // C#
                4 => 2,  // D#
                6 => 3,  // F#
                9 => 5,  // G#
                11 => 6, // A#
                _ => continue,
            };

            let octave = key / 12;

            let global_white_index = octave * 7 + white_index - 1;

            let y =
                rect.bottom() - (global_white_index as f32 * white_height) - (white_height * 0.25);

            let key_rect = Rect::from_min_max(
                egui::pos2(rect.left(), y),
                egui::pos2(rect.left() + rect.width() * 0.5, y + white_height * 0.5),
            );
            black_hovered |= ui.rect_contains_pointer(key_rect);

            keys.push(Key {
                index: *key,
                rect: key_rect,
                border_radius: 2.0,
                flat: true,
            });
        }

        for (i, key) in whites.iter().enumerate() {
            let y = rect.bottom() - (i as f32 * white_height) - white_height;
            let key_rect = Rect::from_min_max(
                egui::pos2(rect.left(), y),
                egui::pos2(rect.right(), y + white_height),
            );

            keys.push(Key {
                index: *key,
                rect: key_rect,
                border_radius: 2.0,
                flat: false,
            });
        }

        keys.iter().rev().for_each(|key| {
            let mut color = if key.flat {
                Color32::BLACK
            } else {
                Color32::WHITE
            };

            if ui.rect_contains_pointer(key.rect) && (!black_hovered || key.flat) {
                color = if key.flat {
                    Color32::DARK_GRAY
                } else {
                    Color32::LIGHT_GRAY
                };
                if pointer_pressed {
                    let should_play = match self.hovered_note {
                        Some(note) => {
                            if note != key.index {
                                engine.get_audio_handler().update_from_gamelogic(
                                    AudioCommand::Stop(AudioTrigger::gamelogic(&format!(
                                        "{}",
                                        note
                                    ))),
                                );

                                true
                            } else {
                                false
                            }
                        }
                        None => {
                            self.hovered_note = Some(key.index);
                            true
                        }
                    };

                    if should_play {
                        println!("NOTE OFF: {}", key.index + 1);

                        engine
                            .get_audio_handler()
                            .update_from_gamelogic(AudioCommand::ForcePlay(
                                AudioTrigger::gamelogic(&format!("{}", key.index)),
                            ));
                        self.hovered_note = Some(key.index);
                    }
                }

                if pointer_released {
                    println!("NOTE OFF: {}", key.index + 1);
                    engine
                        .get_audio_handler()
                        .update_from_gamelogic(AudioCommand::Stop(AudioTrigger::gamelogic(
                            &format!("{}", key.index),
                        )));
                    self.hovered_note = None
                }
            }

            painter.rect_filled(key.rect, key.border_radius, color);
            if (key.index % 12) == 3 {
                let text = format!("C{}", (key.index / 12) + 1); // or whatever label you want

                let font_id = FontId::new(12.0, egui::FontFamily::Monospace);

                let text_color = Color32::BLACK;

                painter.text(
                    key.rect.center(),
                    Align2::CENTER_CENTER,
                    text,
                    font_id,
                    text_color,
                );
            }
        });

        (rect, response, painter)
    }
    fn draw_piano_roll(
        &mut self,
        ui: &mut Ui,
        roll_height: f32,
        width: f32,
    ) -> (Rect, Response, Painter) {
        let rect_height = roll_height * self.keys.len() as f32;
        let (rect, sense) =
            ui.allocate_exact_size([width, rect_height].into(), Sense::click_and_drag());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let spacing = roll_height;

        for i in 0..self.keys.len() {
            let y = rect.bottom() - i as f32 * spacing;
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, Color32::from_gray(40)),
            );
        }

        (rect, sense, painter)
    }
}

fn layer_from_cursor(cursor: egui::Pos2, rect: egui::Rect, piano_roll_bar_height: f32) -> usize {
    let local_y = (cursor.y - rect.top()).clamp(0.0, rect.height());
    (local_y / piano_roll_bar_height).floor() as usize
}
//
fn create_sound_block_from_cursor(
    cursor: egui::Pos2,
    sound_element: &MidiNote,
    piano_roll_bar_height: f32,
    offset: &egui::Vec2,
) -> Rect {
    let top_left = egui::pos2(cursor.x - offset.x, cursor.y - offset.y);
    let bottom_right = egui::pos2(
        top_left.x + u32_to_f32(sound_element.length),
        top_left.y + piano_roll_bar_height,
    );
    Rect::from_two_pos(top_left, bottom_right)
}
fn create_sound_block(layer_index: usize, bar_height: f32, sound_element: &MidiNote) -> Rect {
    let layer_top_point = layer_index as f32 * bar_height;
    let layer_bottom_point = layer_index as f32 * bar_height + bar_height;

    Rect::from_two_pos(
        [u32_to_f32(sound_element.start), layer_top_point].into(),
        [
            u32_to_f32(sound_element.start + sound_element.length),
            layer_bottom_point,
        ]
        .into(),
    )
}

fn get_black_and_white_keys(piano_size: usize) -> (Vec<usize>, Vec<usize>) {
    let mut blacks = Vec::with_capacity(piano_size / 2);
    let mut whites = Vec::with_capacity(piano_size);
    for key in OCTAVE_OFFSET..(OCTAVE_OFFSET + OCTAVES * 12) {
        let note_index = key % 12;

        let is_black = matches!(note_index, 1 | 4 | 6 | 9 | 11);
        if is_black {
            blacks.push(key);
        } else {
            whites.push(key);
        }
    }
    (blacks, whites)
}
