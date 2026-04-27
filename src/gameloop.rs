use std::{
    collections::{BTreeMap, HashMap},
    vec,
};

use sparmos_engine::{
    application::state::{Game, State, map_value},
    audio::{
        audio_handler::{AudioCommand, AudioHandler, AudioTrigger, get_full_piano, pianokey_to_hz},
        midi::Midi,
        synth::{EnvelopeSegment, Sound, Waveform},
    },
    cgmath::{self, *},
    core::{
        engine::Engine,
        entities::World,
        instance::{GpuInstance, Instance, InstanceController},
        material::MaterialBuilder,
        post_processing::Effect,
        render::Renderable,
    },
    egui::{self, Ui},
    entities::cube,
    log,
    systems::{
        animation::{AnimationHandler, AnimationStep, AnimationType, Interpolation, StepState},
        camera::{Camera, CameraAnimator, CameraMode, CameraSystem, MovementKey, MovementPress},
        light::{Light, LightSystem},
    },
    winit::{
        self,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::KeyCode,
    },
};

use crate::{
    circular_buffer::CircularBuffer,
    easter_egg::EasterEgg,
    gui::sound_editor::{GuiState, Ratio, RatioHandle, SoundEditor},
    markers::{self},
    transition::{CameraPositions, TransitionHandler},
    voxel_builder::{VoxelHandler, VoxelObjects, instances_list_cube},
};

pub struct Website {
    pub score: u32,
    pub counter: usize,
    pub cursor_pos: PhysicalPosition<f32>,
    pub cursor_delta: (f64, f64),
    pub voxel_handler: VoxelHandler<VoxelObjects>,
    pub transition_handler: TransitionHandler<VoxelObjects>,
    pub camera_transition_handler: TransitionHandler<CameraPositions>,
    pub bad_apple: EasterEgg,
    pub gui_context: GuiState,
    pub sounds: Vec<Sound>,
}

impl Default for Website {
    fn default() -> Self {
        Self {
            score: 0,
            counter: 0,
            cursor_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            cursor_delta: (0.0, 0.0),
            voxel_handler: VoxelHandler::<VoxelObjects>::default(),
            transition_handler: TransitionHandler::<VoxelObjects>::new(BTreeMap::new()),
            camera_transition_handler: TransitionHandler::<CameraPositions>::new(BTreeMap::new()),
            bad_apple: EasterEgg::default(),
            gui_context: GuiState::default(),
            sounds: vec![],
        }
    }
}

impl Game for Website {
    fn update(&mut self, dt: std::time::Duration, engine: &mut Engine, world: &mut World) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();

        let buffer_string = engine.arguments.with_arg::<CircularBuffer<String>, _>(
            "keypress",
            |buffer| match buffer {
                Some(buffer) => buffer.to_string(),
                None => "".to_string(),
            },
        );

        if buffer_string == "badapple" && !self.bad_apple.toggle {
            world.query_first_with_resources::<&mut Camera>(|resource, camera| {
                let camera_system = resource.get_system_mut::<CameraSystem>();
                camera_system.set(MovementKey::RotateLeft, MovementPress::Override);
                camera.set_camera_mode(CameraMode::AnimatedMode);
                self.bad_apple.init_camera(camera);
                self.bad_apple.update_camera(camera_system, camera);
            });

            world.query_first::<(&Renderable, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);

                engine.change_shader(&render.material_handle, "lights");
            });
            println!("Test");

            self.bad_apple.toggle = true;
            log::warn!("EE started!");
        }
        if buffer_string == "ihatefun" && self.bad_apple.toggle {
            world.query_first_with_resources::<&mut Camera>(|resource, camera| {
                let camera_system = resource.get_system_mut::<CameraSystem>();
                camera_system.set(MovementKey::RotateLeft, MovementPress::NotPressed);
                camera.set_camera_mode(CameraMode::FreeMode);
                self.bad_apple.reset_camera(camera_system);
            });

            world.query_first::<(&Renderable, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);
                engine.change_shader(&render.material_handle, "boxes");
            });
            self.bad_apple.toggle = false;

            log::warn!("EE Stopped :(");
        }
        let scroll_y = engine
            .arguments
            .with_arg::<f64, _>("scrolly", |buffer| *buffer.unwrap_or(&0.0));

        if let Some(transition) = self.transition_handler.get_transition_once(scroll_y as i64) {
            log::warn!("Transition!!!");
            match transition.clone() {
                VoxelObjects::Home => {}
                _ => {
                    world.query_first::<(&Renderable, &mut AnimationHandler)>(
                        |(renderable, ah)| {
                            let ic = engine
                                .get_instance_controller(&renderable.instance_controller_handle);
                            ah.reset_instance_position_to_current_position(
                                ic.instances_mut().as_mut(),
                            );
                            self.voxel_handler
                                .transition_to_object(transition, ah, true, 1.0);
                        },
                    );
                }
            }
        }

        if let Some(transition) = self
            .camera_transition_handler
            .get_transition_once(scroll_y as i64)
        {
            world.query_first::<(&mut Camera, &mut CameraAnimator)>(|(camera, camera_animator)| {
                match transition.clone() {
                    CameraPositions::Middle(position)
                    | CameraPositions::LeftSide(position)
                    | CameraPositions::RightSide(position)
                    | CameraPositions::FrontAndCenter(position) => {
                        camera_animator.reset_animation(camera);
                        camera_animator.add_animation(
                            Some(AnimationType::Step(AnimationStep::new(
                                camera.eye.to_vec(),
                                vec3(
                                    position.0.x as f32,
                                    position.0.y as f32,
                                    position.0.z as f32,
                                ),
                                0.0,
                                camera_animator.speed,
                                Interpolation::EaseInEaseOut,
                                StepState::Forward,
                            ))),
                            Some(AnimationType::Step(AnimationStep::new(
                                camera.target.to_vec(),
                                vec3(
                                    position.1.x as f32,
                                    position.1.y as f32,
                                    position.1.z as f32,
                                ),
                                0.0,
                                camera_animator.speed,
                                Interpolation::EaseInEaseOut,
                                StepState::Forward,
                            ))),
                        );
                    }
                }
            });
        }
        if self.bad_apple.toggle {
            let target = 1.0 / self.bad_apple.fps;
            self.bad_apple.elapsed += dt.as_secs_f32();

            if self.bad_apple.elapsed >= target {
                world.query_first::<(&Renderable, &mut AnimationHandler)>(|(renderable, ah)| {
                    let ic = engine.get_instance_controller(&renderable.instance_controller_handle);
                    ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                    self.voxel_handler.transition_to_point_list(
                        self.bad_apple.get_frame(),
                        ah,
                        1.0,
                    );
                });
                world.query_first_with_resources::<&mut Camera>(|resource, camera| {
                    log::warn!("{:?}", camera.eye.z);
                    let camera_system = resource.get_system_mut::<CameraSystem>();
                    self.bad_apple.update_camera(camera_system, camera)
                });

                self.bad_apple.index += 1;
                self.bad_apple.elapsed -= target;
            }
        }

        // world.query_first_with_resources::<(&mut Camera, &mut CameraAnimator)>(
        //     |resources, (camera, camera_animator)| {
        //         let camera_system = resources.get_system_mut::<CameraSystem>();
        //         camera_system.update_camera(dt, &engine.render_context, camera);
        //         camera_animator.update(dt.as_secs_f32(), camera);
        //     },
        // );
    }

    fn process_event(
        &mut self,
        event: &winit::event::WindowEvent,
        _screen: &winit::dpi::PhysicalSize<u32>,
        engine: &mut Engine,
        world: &mut World,
    ) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();
        // let (entity, camera) = state
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: winit::keyboard::PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => match keycode {
                KeyCode::Space => {}
                KeyCode::PageUp => {
                    if state == &winit::event::ElementState::Pressed {
                        world.query_first::<(&Renderable, &mut AnimationHandler)>(
                            |(render, ah)| {
                                let ic = engine
                                    .get_instance_controller(&render.instance_controller_handle);
                                ah.reset_instance_position_to_current_position(
                                    ic.instances_mut().as_mut(),
                                );
                                self.voxel_handler.transition_to_object(
                                    VoxelObjects::HandballBird,
                                    ah,
                                    true,
                                    1.0,
                                );
                                ah.update_instance(0.0, ic.instances_mut().as_mut());
                            },
                        );
                        engine
                            .audio_handler
                            .as_mut()
                            .unwrap()
                            .update_from_gamelogic(AudioCommand::ForcePlay(
                                AudioTrigger::GameLogic("test".to_string()),
                            ));
                    }
                }

                KeyCode::PageDown => {
                    if state == &winit::event::ElementState::Pressed {
                        let mut query = world
                            .entities
                            .query::<(&Renderable, &mut AnimationHandler)>();
                        let (render, ah) = query.iter().next().expect("No AH");

                        let ic = engine
                            .render_context
                            .gpu_objects
                            .instance_controllers
                            .get_mut(render.instance_controller_handle)
                            .unwrap();

                        ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                        self.voxel_handler.transition_to_object(
                            VoxelObjects::FemogfirsSlangen,
                            ah,
                            true,
                            1.0,
                        );
                        engine.change_shader(&render.material_handle, "boxes");
                        println!("snake!l!");
                        engine
                            .audio_handler
                            .as_mut()
                            .unwrap()
                            .update_from_gamelogic(AudioCommand::Edit(
                                AudioTrigger::GameLogic("test".to_string()),
                                self.sounds[9].clone(),
                            ));
                    }
                }
                KeyCode::Delete => {
                    if state == &winit::event::ElementState::Pressed {
                        let mut query = world
                            .entities
                            .query::<(&Renderable, &mut AnimationHandler)>();
                        let (render, ah) = query.iter().next().expect("No AH");

                        let ic = engine
                            .render_context
                            .gpu_objects
                            .instance_controllers
                            .get_mut(render.instance_controller_handle)
                            .unwrap();

                        ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                        self.voxel_handler.transition_to_point_list(
                            self.bad_apple.get_frame(),
                            ah,
                            1.0,
                        );
                        self.bad_apple.index += 1;
                    }
                }

                KeyCode::Home => match state {
                    #[cfg(not(target_arch = "wasm32"))]
                    winit::event::ElementState::Pressed => {}
                    _ => {
                        let buffer = engine
                            .arguments
                            .args
                            .entry("keypress".to_string())
                            .or_insert(Box::new(CircularBuffer::<String>::new(8)))
                            .downcast_mut::<CircularBuffer<String>>();
                        if let Some(buffer) = buffer {
                            buffer.insert("i".to_string());
                            buffer.insert("h".to_string());
                            buffer.insert("a".to_string());
                            buffer.insert("t".to_string());
                            buffer.insert("e".to_string());
                            buffer.insert("f".to_string());
                            buffer.insert("u".to_string());
                            buffer.insert("n".to_string());

                            log::warn!("{:?}", buffer.to_string())
                        }
                    }
                },
                KeyCode::End => match state {
                    #[cfg(not(target_arch = "wasm32"))]
                    winit::event::ElementState::Pressed => {
                        let buffer = engine
                            .arguments
                            .args
                            .entry("keypress".to_string())
                            .or_insert(Box::new(CircularBuffer::<String>::new(8)))
                            .downcast_mut::<CircularBuffer<String>>();
                        if let Some(buffer) = buffer {
                            buffer.insert("b".to_string());
                            buffer.insert("a".to_string());
                            buffer.insert("d".to_string());
                            buffer.insert("a".to_string());
                            buffer.insert("p".to_string());
                            buffer.insert("p".to_string());
                            buffer.insert("l".to_string());
                            buffer.insert("e".to_string());

                            log::warn!("{:?}", buffer.to_string())
                        }
                    }
                    _ => {}
                },

                _ => (),
            },
            WindowEvent::MouseInput { state, button, .. } => {
                match button {
                    winit::event::MouseButton::Left => match state {
                        ElementState::Pressed => {}
                        ElementState::Released => {}
                    },

                    winit::event::MouseButton::Right => match state {
                        ElementState::Pressed => {}
                        ElementState::Released => {}
                    },

                    // winit::event::MouseButton::Right => todo!(),
                    // winit::event::MouseButton::Middle => todo!(),
                    // winit::event::MouseButton::Back => todo!(),
                    // winit::event::MouseButton::Forward => todo!(),
                    // winit::event::MouseButton::Other(_) => todo!(),
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = PhysicalPosition::new(position.x as f32, position.y as f32);

                // let test = self.camera_controller.camera.screen_to_world_ray(
                //     self.cursor_position.x,
                //     self.cursor_position.y,
                //     screen.width as f32,
                //     screen.height as f32,
                // );
                // line_trace(&mut self.instance_controller2, camera, &self.queue, &self.device, test);

                // if let Some(controller) = self.chunk_map.get_mut(&target_chunk) {
                //     if let Some(i) = line_trace(controller, test) {
                //         controller.remove_instance(i, &self.queue);
                //     }
                // }
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::MouseWheel { delta, .. } => {
                use sparmos_engine::winit::event::MouseScrollDelta;

                if let MouseScrollDelta::LineDelta(_, y) = delta {
                    engine
                        .arguments
                        .args
                        .insert("scrolly".to_string(), Box::new(*y));
                }
            }

            _ => (),
        }
        world.query_first_with_resources::<&mut Camera>(|resources, camera| {
            let camera_system = resources.get_system_mut::<CameraSystem>();
            camera_system.process_events(event, camera);
        });
    }

    fn setup(&mut self, state: &mut State) {
        let engine = &mut state.engine;
        let world = &mut state.world;
        //Initiates Camera system
        let camera = Camera::new(PhysicalSize::new(
            state.size.width as f32,
            state.size.height as f32,
        ));
        let camera_system = CameraSystem::new(75.0, 50.0, &engine.render_context.device, &camera);

        let camera_animater = CameraAnimator::new(0.75, camera.eye, camera.target);

        let camera_speed = camera_system.speed;
        world.add_entity((camera, camera_animater));
        world.add_system(camera_system);
        //Initiates lighting
        let light = Light {
            position: cgmath::vec3(200.0, 200.0, 1.0),
            color: cgmath::vec3(1.0, 1.0, 1.0),
        };

        let light2 = Light {
            position: cgmath::vec3(-200.0, -200.0, 1.0),
            color: cgmath::vec3(1.0, 1.0, 1.0),
        };
        let light_system = LightSystem::init(
            &[light.clone(), light2.clone()],
            &engine.render_context.device,
        );
        world.add_system(light_system);

        //Initiate Shaders
        engine
            .render_context
            .add_shader("lights", include_str!("shaders/lights.wgsl"));
        engine
            .render_context
            .add_shader("boxes", include_str!("shaders/boxes.wgsl"));

        //Initiate meshes
        let cube_mesh = cube::new().make_mb(&mut engine.render_context);

        let light_ic = InstanceController::<GpuInstance>::new(
            vec![
                Instance::new([200.0, 200.0, 1.0].into(), 10.0),
                Instance::new([-200.0, -200.0, 1.0].into(), 10.0),
            ],
            &mut engine.render_context,
        );
        let light_mat = MaterialBuilder::new()
            .add_layout("camera", world.resources.get_system::<CameraSystem>())
            .add_layout("light", world.resources.get_system::<LightSystem>())
            .add_shader("lights")
            .build(&cube_mesh, &light_ic, &mut engine.render_context);

        let light_entity = Renderable {
            material_handle: light_mat,
            instance_controller_handle: light_ic,
            mesh_handle: cube_mesh,
        };

        world.add_entity((light_entity, markers::Light));
        let instances = instances_list_cube(vec3(0, 0, 0), vec3(40, 50, 40));

        let instances_len = instances.len();
        let animation_handler = AnimationHandler::new_from_instances(&instances, vec![]);
        let box_ic = InstanceController::<GpuInstance>::new(instances, &mut engine.render_context);

        let box_mat = MaterialBuilder::new()
            .add_layout("camera", world.resources.get_system::<CameraSystem>())
            .add_layout("light", world.resources.get_system::<LightSystem>())
            .add_shader("boxes")
            .build(&cube_mesh, &box_ic, &mut engine.render_context);
        let box_entity = Renderable {
            material_handle: box_mat,
            instance_controller_handle: box_ic,
            mesh_handle: cube_mesh,
        };

        world.add_entity((box_entity, markers::Boxes, animation_handler));
        // }

        let castle = include_bytes!("../castle.vox");
        let chr_knight = include_bytes!("../chr_knight.vox");
        let rust_logo = include_bytes!("../rust.vox");
        let c_plus_plus = include_bytes!("../cplusplus.vox");
        let c_sharp = include_bytes!("../csharp.vox");
        let docker = include_bytes!("../docker.vox");
        let hb_fugl = include_bytes!("../hbfugl.vox");
        let femo_snake = include_bytes!("../femoslangen.vox");
        self.voxel_handler.add_voxel(castle, VoxelObjects::Castle);
        self.voxel_handler
            .add_voxel(chr_knight, VoxelObjects::Viking);
        self.voxel_handler.add_voxel(rust_logo, VoxelObjects::Rust);
        self.voxel_handler
            .add_voxel(c_plus_plus, VoxelObjects::CPlusPLus);
        self.voxel_handler.add_voxel(c_sharp, VoxelObjects::CSharp);
        self.voxel_handler
            .add_voxel(docker, VoxelObjects::Containerization);
        self.voxel_handler
            .add_voxel(hb_fugl, VoxelObjects::HandballBird);
        self.voxel_handler
            .add_voxel(femo_snake, VoxelObjects::FemogfirsSlangen);

        for p in 0..instances_len {
            self.voxel_handler.current_cubes.push(p);
        }
        let transition_map: BTreeMap<i64, VoxelObjects> = BTreeMap::from([
            (300, VoxelObjects::Home),
            (1300, VoxelObjects::CSharp),
            (2100, VoxelObjects::Rust),
            (2950, VoxelObjects::CPlusPLus),
            (3850, VoxelObjects::Containerization),
            (4750, VoxelObjects::CPlusPLus),
            (5599, VoxelObjects::CSharp),
            (6485, VoxelObjects::Rust),
            (7200, VoxelObjects::CPlusPLus),
        ]);
        self.transition_handler.transition_map = transition_map;

        let camera_middle = CameraPositions::Middle(((-120, 90, -120).into(), (20, 25, 20).into()));
        let camera_right_side =
            CameraPositions::RightSide(((-50, 50, -190).into(), (90, 25, -50).into()));
        let camera_left_side =
            CameraPositions::LeftSide(((90, 90, -190).into(), (-50, 25, -50).into()));
        let camera_transition: BTreeMap<_, _> = [
            (300, camera_middle.clone()),
            (1300, camera_right_side.clone()),
            (2100, camera_left_side.clone()),
            (2950, camera_right_side.clone()),
            (3850, camera_left_side.clone()),
            (4750, camera_middle.clone()),
            (5599, camera_right_side.clone()),
            (6485, camera_left_side.clone()),
            (7200, camera_middle.clone()),
        ]
        .into_iter()
        .collect();

        //Bad Apple setup
        let badapple_bin = include_bytes!("../pixels.bin");

        // let pixels = vec![]
        let badapple = EasterEgg::new(
            PhysicalSize {
                width: 326,
                height: 244,
            },
            30.0,
            badapple_bin.to_vec(),
            camera_speed,
        );
        engine.render_context.post_processing.new_effect(
            (
                engine.render_context.config.width,
                engine.render_context.config.height,
            )
                .into(),
            engine.render_context.config.format,
            Effect::ChromaticAberration,
        );
        self.camera_transition_handler.transition_map = camera_transition;
        self.bad_apple = badapple;
        let keys = [
            "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4", "A#4", "B4", "C5",
        ];
        const HARMONICS_PIANO_ORGANIC: [f32; 7] = [1.00, 0.30, 0.10, 0.05, 0.10, 0.7, 0.02];
        let sounds = keys
            .iter()
            .map(|key| {
                let freq = pianokey_to_hz(key);
                println!("{}", freq.unwrap());
                Sound::new(
                    HARMONICS_PIANO_ORGANIC.into(),
                    freq.expect("Key not parsed"),
                    0.0,
                    Waveform::SineWave,
                    EnvelopeSegment {
                        length: 0.01,
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
                )
            })
            .collect::<Vec<Sound>>();

        let mut audio_triggers = HashMap::from([
            (AudioTrigger::Keyboard(KeyCode::KeyF), sounds[0].clone()),
            (AudioTrigger::Keyboard(KeyCode::KeyG), sounds[2].clone()),
            (AudioTrigger::Keyboard(KeyCode::KeyH), sounds[4].clone()),
            (AudioTrigger::Keyboard(KeyCode::KeyJ), sounds[5].clone()),
            (AudioTrigger::Keyboard(KeyCode::KeyK), sounds[7].clone()),
            (AudioTrigger::Keyboard(KeyCode::KeyL), sounds[9].clone()),
            (
                AudioTrigger::Keyboard(KeyCode::Semicolon),
                sounds[11].clone(),
            ),
            (
                AudioTrigger::GameLogic("test".to_string()),
                sounds[6].clone(),
            ),
        ]);
        //88 is the standard piano key count
        for (i, sound) in get_full_piano().iter().enumerate() {
            audio_triggers.insert(AudioTrigger::GameLogic(i.to_string()), sound.clone());
        }
        AudioHandler::init_sounds(state, audio_triggers);
        self.gui_context.sound_editor.handles = [
            RatioHandle {
                ratio: 0.3,
                kind: Ratio::AttackDecayBoundary,
            },
            RatioHandle {
                ratio: 0.8,
                kind: Ratio::DecayRefrainBoundary,
            },
        ]
        .into();
        self.sounds = sounds;
        let midi = include_bytes!("../rickroll.mid");
        let midi_parsed = Midi::load_midi(midi);
        self.gui_context.piano_roll.midis.push(midi_parsed);
        // self.gui_context.piano_roll.create_track_from_midi(0, 15);
        let wii_midi = include_bytes!("../mii.mid");
        let wii_parsed = Midi::load_midi(wii_midi);
        self.gui_context.piano_roll.midis.push(wii_parsed);
        self.gui_context.piano_roll.create_track_from_midi(1, 0);
    }

    fn resize(&mut self, engine: &mut Engine, world: &mut World) {
        let mut query = world.entities.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");

        camera.aspect =
            engine.render_context.config.width as f32 / engine.render_context.config.height as f32;
        println!("{:?}", camera.aspect);
        let new_fov = map_value(camera.aspect, 0.8, 1.88, 25.0, 55.0);
        camera.fovy = new_fov;
    }

    fn gui_setup(&mut self, dt: std::time::Duration, engine: &mut Engine, ui: &mut Ui) {
        let mut visuals = egui::Visuals::dark();

        visuals.override_text_color = Some(egui::Color32::from_gray(220));

        // Panels
        visuals.panel_fill = egui::Color32::from_rgb(10, 10, 10);
        visuals.window_fill = egui::Color32::from_rgb(15, 15, 15);

        // Subtle separation instead of borders
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(12, 12, 12);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(18, 18, 18);

        // Remove harsh outlines
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);

        // Selection (use your red here)
        visuals.selection.bg_fill = egui::Color32::from_rgb(120, 0, 0);

        ui.set_visuals(visuals);
        egui::Panel::top("top_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .toggle_value(&mut self.gui_context.piano_roll_toggled, "Piano Roll")
                        .clicked()
                    {};
                    if ui
                        .toggle_value(&mut self.gui_context.sound_editor_toggled, "Sound Editor")
                        .clicked()
                    {};
                });
            });

        if self.gui_context.piano_roll_toggled {
            egui::Window::new("Sound Player")
                .resizable(false)
                .min_width(1100.0)
                .min_height(500.0)
                .show(ui, |ui| {
                    self.gui_context.piano_roll.ui(dt, engine, ui);
                });
        }

        if self.gui_context.sound_editor_toggled {
            self.gui_context.sound_editor.ui(dt, engine, ui);
        }
    }
}
