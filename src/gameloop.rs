use std::{
    cell::Ref,
    collections::{BTreeMap, HashMap},
    ops::Deref,
    vec,
};

use rand::Rng;
use sparmos_engine::{
    application::{
        graphics::Graphics,
        gui_elements::tui::{TuiBorder, TuiPanel, TuiWindow, toggleable_tui_button},
        state::{Game, State, map_value},
    },
    audio::{
        audio_handler::{AudioCommand, AudioHandler, AudioTrigger, get_full_piano, pianokey_to_hz},
        midi::Midi,
        synth::{EnvelopeSegment, Sound, Waveform},
    },
    cgmath::{self, *},
    core::{
        entities::World,
        geometry::{Model, Primitive, Textured},
        instance::{GpuInstance, Instance},
        post_processing::Effect,
        render::{ComputeRenderable, Renderable},
    },
    egui::{self, FontFamily, FontId, Id, TextStyle, Ui, pos2, vec2},
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
    gui::sound_editor::{GuiState, Ratio, RatioHandle},
    markers::{self, Bounds, ComputeArea, Particle},
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
    fn update(&mut self, gfx: &mut Graphics, world: Ref<'_, World>) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();

        let buffer_string =
            gfx.engine
                .arguments
                .with_arg::<CircularBuffer<String>, _>("keypress", |buffer| match buffer {
                    Some(buffer) => buffer.to_string(),
                    None => "".to_string(),
                });

        if buffer_string == "badapple" && !self.bad_apple.toggle {
            world.query_first::<&mut Camera>(|camera| {
                let camera_system = gfx.get_system_mut::<CameraSystem>();
                camera_system.set(MovementKey::RotateLeft, MovementPress::Override);
                camera.set_camera_mode(CameraMode::AnimatedMode);
                self.bad_apple.init_camera(camera);
                self.bad_apple.update_camera(camera_system, camera);
            });

            world.query_first::<(&Renderable, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);

                gfx.change_shader(&render.material_handle, "lights");
            });
            println!("Test");

            self.bad_apple.toggle = true;
            log::warn!("EE started!");
        }
        if buffer_string == "ihatefun" && self.bad_apple.toggle {
            world.query_first::<&mut Camera>(|camera| {
                let camera_system = gfx.get_system_mut::<CameraSystem>();
                camera_system.set(MovementKey::RotateLeft, MovementPress::NotPressed);
                camera.set_camera_mode(CameraMode::FreeMode);
                self.bad_apple.reset_camera(camera_system);
            });

            world.query_first::<(&Renderable, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);
                gfx.change_shader(&render.material_handle, "boxes");
            });
            self.bad_apple.toggle = false;

            log::warn!("EE Stopped :(");
        }
        let scroll_y = gfx
            .engine
            .arguments
            .with_arg::<f64, _>("scrolly", |buffer| *buffer.unwrap_or(&0.0));

        if let Some(transition) = self.transition_handler.get_transition_once(scroll_y as i64) {
            log::warn!("Transition!!!");
            match transition.clone() {
                VoxelObjects::Home => {}
                _ => {
                    world.query_first::<(&Renderable, &mut AnimationHandler)>(
                        |(renderable, ah)| {
                            let ic = gfx
                                .engine
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
            self.bad_apple.elapsed += gfx.dt().as_secs_f32();

            if self.bad_apple.elapsed >= target {
                world.query_first::<(&Renderable, &mut AnimationHandler)>(|(renderable, ah)| {
                    let ic = gfx
                        .engine
                        .get_instance_controller(&renderable.instance_controller_handle);
                    ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                    self.voxel_handler.transition_to_point_list(
                        self.bad_apple.get_frame(),
                        ah,
                        1.0,
                    );
                });
                world.query_first::<&mut Camera>(|camera| {
                    log::warn!("{:?}", camera.eye.z);
                    let camera_system = gfx.get_system_mut::<CameraSystem>();
                    self.bad_apple.update_camera(camera_system, camera)
                });

                self.bad_apple.index += 1;
                self.bad_apple.elapsed -= target;
            }
        }
    }

    fn process_event(
        &mut self,
        event: &winit::event::WindowEvent,
        _screen: &winit::dpi::PhysicalSize<u32>,
        gfx: &mut Graphics,
        world: Ref<'_, World>,
    ) {
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
                                let ic = gfx
                                    .engine
                                    .get_instance_controller(&render.instance_controller_handle);
                                ah.reset_instance_position_to_current_position(ic.instances_mut());
                                self.voxel_handler.transition_to_object(
                                    VoxelObjects::HandballBird,
                                    ah,
                                    true,
                                    1.0,
                                );
                                ah.update_instance(0.0, ic.instances_mut().as_mut());
                            },
                        );

                        gfx.engine
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
                        println!("query len: {}", query.iter().len());
                        let (render, ah) = query.iter().next().expect("No AH");

                        let ic = gfx
                            .engine
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
                        gfx.change_shader(&render.material_handle, "boxes");
                        println!("snake!l!");
                        gfx.engine
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

                        let ic = gfx
                            .engine
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
                        let buffer = gfx
                            .engine
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
                        let buffer = gfx
                            .engine
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
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::MouseWheel { delta, .. } => {
                use sparmos_engine::winit::event::MouseScrollDelta;

                if let MouseScrollDelta::LineDelta(_, y) = delta {
                    gfx.engine
                        .arguments
                        .args
                        .insert("scrolly".to_string(), Box::new(*y));
                }
            }

            _ => (),
        }
        world.query_first::<&mut Camera>(|camera| {
            let camera_system = gfx.get_system_mut::<CameraSystem>();
            camera_system.process_events(event, camera);
        });
    }

    fn setup(&mut self, state: &mut State) {
        let gfx = &mut state.graphics;
        //Initiates Camera system
        let camera = Camera::new(PhysicalSize::new(
            state.size.width as f32,
            state.size.height as f32,
        ));
        let camera_system =
            CameraSystem::new(75.0, 50.0, &gfx.engine.render_context.device, &camera);

        let camera_animater = CameraAnimator::new(0.75, camera.eye, camera.target);

        let camera_speed = camera_system.speed;
        gfx.add_entity((camera, camera_animater));
        gfx.add_system(camera_system);

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
            &gfx.engine.render_context.device,
        );
        gfx.add_system(light_system);

        //Initiate Shaders
        gfx.shader("lights", include_str!("shaders/lights.wgsl"));
        gfx.shader("boxes", include_str!("shaders/boxes.wgsl"));

        gfx.shader("compute", include_str!("shaders/compute.wgsl"));

        gfx.shader("particle", include_str!("shaders/particle.wgsl"));
        gfx.shader(
            "particle_render",
            include_str!("shaders/particle_render.wgsl"),
        );

        gfx.shader(
            "particle_render_with_mesh",
            include_str!("shaders/particle_render_with_mesh.wgsl"),
        );

        gfx.shader("textured", include_str!("shaders/textured.wgsl"));
        //Initiate meshes
        let cube_mesh = cube::new().make_mb(&mut gfx.engine.render_context);

        let light_ic = gfx
            .instances::<GpuInstance>()
            .from_instances(vec![
                Instance::new([200.0, 200.0, 1.0].into(), 10.0),
                Instance::new([-200.0, -200.0, 1.0].into(), 10.0),
            ])
            .build();

        let light_mat = gfx
            .material::<Primitive, GpuInstance>()
            .shader("lights")
            .build();
        let light_entity = Renderable {
            material_handle: light_mat,
            instance_controller_handle: light_ic,
            mesh_handle: cube_mesh,
        };

        gfx.add_entity((light_entity, markers::Light));
        let instances = instances_list_cube(vec3(0, 0, 0), vec3(40, 50, 40));

        let instances_len = instances.len();
        let animation_handler = AnimationHandler::new_from_instances(&instances, vec![]);
        let cube_mesh = cube::new().make_mb(&mut gfx.engine.render_context);

        let box_ic = gfx
            .instances::<GpuInstance>()
            .from_instances(instances)
            .build();

        let box_mat = gfx
            .material::<Primitive, GpuInstance>()
            .shader("boxes")
            .build();

        let box_entity = Renderable {
            material_handle: box_mat,
            instance_controller_handle: box_ic,
            mesh_handle: cube_mesh,
        };

        gfx.add_entity((box_entity, markers::Boxes, animation_handler));

        let test: [u32; 8] = [2, 5, 1, 2, 3, 4, 6, 8];

        let compute = gfx
            .compute::<u32>()
            .shader("compute")
            .size(64)
            .input_buffer(&test)
            // .readback()
            .build();

        gfx.add_entity((compute,));
        let particles = create_particles(128000);
        let bounds = Bounds {
            bounds: [100.0, 100.0, 100.0],
            _padding: 0.0,
        };
        let compute2 = gfx
            .compute::<Particle>()
            .shader("particle")
            .size(128000)
            .initial_data(&particles)
            .input_buffer(&[bounds])
            .build();
        gfx.add_entity((compute2,));

        let compute_area = ComputeArea {
            global_pos: [100.0, 100.0, -3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            _padding: 0.0,
        };
        let particle_rendering = gfx
            .compute_rendering(compute2)
            .mesh::<Primitive>()
            .input_buffer(&[compute_area])
            .shader("particle_render_with_mesh")
            .build();

        let particle_renderable = ComputeRenderable {
            rendering_handle: particle_rendering,
            mesh_handle: cube_mesh,
        };
        gfx.add_entity((particle_renderable,));

        let model_ic = gfx
            .instances::<GpuInstance>()
            .from_instances(vec![Instance::new([2.0, 2.0, 1.0].into(), 100.0)])
            .build();

        let model_mat = gfx
            .material::<Textured, GpuInstance>()
            .texture_from_color([0.5, 0.5, 0.5], None)
            .compute_buffer(compute)
            .shader("textured")
            .build();

        let model = gfx
            .model()
            .model(include_bytes!("../DATBOI.obj"))
            .material(include_bytes!("../DATBOI.mtl"))
            .texture_pipeline(model_mat)
            .instances(model_ic)
            .build();

        gfx.add_entity((model,));

        println!("{}", gfx.engine.render_context.gpu_objects.materials.len());

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
        gfx.engine.render_context.post_processing.new_effect(
            (
                gfx.engine.render_context.config.width,
                gfx.engine.render_context.config.height,
            )
                .into(),
            gfx.engine.render_context.config.format,
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
        let bad_apple = include_bytes!("../badapple.mid");
        let bad_apple_parsed = Midi::load_midi(bad_apple);
        self.gui_context.piano_roll.midis.push(bad_apple_parsed);

        let wii_midi = include_bytes!("../mii.mid");
        let wii_parsed = Midi::load_midi(wii_midi);
        self.gui_context.piano_roll.midis.push(wii_parsed);
        self.gui_context.piano_roll.create_track_from_midi(0, 0);
    }

    fn resize(&mut self, gfx: &mut Graphics, world: Ref<'_, World>) {
        let mut query = world.entities.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");

        camera.aspect = gfx.engine.render_context.config.width as f32
            / gfx.engine.render_context.config.height as f32;
        println!("{:?}", camera.aspect);
        let new_fov = map_value(camera.aspect, 0.8, 1.88, 25.0, 55.0);
        camera.fovy = new_fov;
    }

    fn gui_setup(&mut self, dt: std::time::Duration, gfx: &mut Graphics, ui: &mut Ui) {
        let mut visuals = egui::Visuals::dark();

        visuals.window_corner_radius = 0.0.into();
        visuals.menu_corner_radius = 0.0.into();
        visuals.widgets.noninteractive.corner_radius = 0.0.into();
        visuals.widgets.inactive.corner_radius = 0.0.into();
        visuals.widgets.hovered.corner_radius = 0.0.into();
        visuals.widgets.active.corner_radius = 0.0.into();

        visuals.window_shadow = egui::Shadow::NONE;
        visuals.popup_shadow = egui::Shadow::NONE;

        visuals.window_fill = egui::Color32::BLACK;
        visuals.panel_fill = egui::Color32::BLACK;
        visuals.extreme_bg_color = egui::Color32::BLACK;
        visuals.faint_bg_color = egui::Color32::from_gray(15);

        visuals.widgets.noninteractive.bg_fill = egui::Color32::BLACK;
        visuals.widgets.inactive.bg_fill = egui::Color32::BLACK;
        visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(20);
        visuals.widgets.active.bg_fill = egui::Color32::from_gray(40);

        visuals.override_text_color = Some(egui::Color32::LIGHT_GRAY);
        let mut style = (*ui.style().deref()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(16.0, FontFamily::Monospace)),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Monospace)),
            (
                TextStyle::Monospace,
                FontId::new(16.0, FontFamily::Monospace),
            ),
            (TextStyle::Button, FontId::new(16.0, FontFamily::Monospace)),
            (TextStyle::Small, FontId::new(16.0, FontFamily::Monospace)),
        ]
        .into();
        ui.ctx().set_style_of(egui::Theme::Dark, style);
        ui.ctx().set_visuals(visuals);
        TuiPanel::top(TuiBorder::HardLines)
            .size(ui, 1)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if toggleable_tui_button(
                        ui,
                        &mut self.gui_context.piano_roll_toggled,
                        "Piano Roll",
                    )
                    .clicked()
                    {}
                    if toggleable_tui_button(
                        ui,
                        &mut self.gui_context.sound_editor_toggled,
                        "Sound Editor",
                    )
                    .clicked()
                    {}
                });
            });

        if self.gui_context.piano_roll_toggled {
            TuiWindow::new(
                Id::new("piano roll"),
                "Piano Roll",
                pos2(100.0, 200.0),
                vec2(800.0, 600.0),
                TuiBorder::HardLines,
            )
            .show(ui, |ui| {
                self.gui_context.piano_roll.ui(dt, &mut gfx.engine, ui);
            });

            //         egui::Window::new("Sound Player")
            // .resizable(true)
            // .min_width(200.0)
            // .min_height(50.0)
            // .show(ui, |ui| {
            //     self.gui_context.piano_roll.ui(dt, engine, ui);
            // });
        }

        if self.gui_context.sound_editor_toggled {
            TuiWindow::new(
                Id::new("sound editor"),
                "Sound Editor",
                pos2(100.0, 200.0),
                vec2(800.0, 600.0),
                TuiBorder::HardLines,
            )
            .show(ui, |ui| {
                self.gui_context.sound_editor.ui(dt, &mut gfx.engine, ui);
            });
        }
    }
}

pub fn create_particles(count: usize) -> Vec<Particle> {
    let mut rng = rand::rng();

    (0..count)
        .map(|_| {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let radius = rng.random_range(0.0..2.0);

            let x = angle.cos() * radius;
            let z = angle.sin() * radius;

            Particle {
                position: [x, rng.random_range(-1.0..1.0), z, 1.0],
                velocity: [x * 1.5, rng.random_range(2.0..8.0), z * 1.5, 0.0],
            }
        })
        .collect()
}
