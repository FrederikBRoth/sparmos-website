use std::{
    cell::Ref,
    collections::{BTreeMap, HashMap},
    ops::Deref,
    vec,
};

use er::{Er, ErError, ErOption, ErResult};
use rand::Rng;
use sparmos_engine::{
    application::{
        event_handler::{GenericEventContext, KeyboardEventContext},
        graphics::Graphics,
        gui_elements::tui::{TuiBorder, TuiPanel, TuiWindow, toggleable_tui_button},
        state::{Game, State, map_value},
    },
    audio::{
        audio_handler::{AudioCommand, AudioHandler, AudioTrigger, get_full_piano, pianokey_to_hz},
        midi::Midi,
        synth::{EnvelopeSegment, Sound, Waveform},
    },
    cgmath::{self, num_traits::ToPrimitive, *},
    core::{
        assets::asset_loader::AssetManifest,
        buffer::{Buffer, BufferType, UniformParameters},
        engine::System,
        entities::World,
        geometry::Vertex,
        instance::{Instance, SpriteInstanceLayout},
        models::model::Model,
        pbr::PhysicsBasedRenderingConstants,
        physics::{
            collision::{Collider, Ray},
            rigidbody::{BodyType, RigidBody},
        },
        post_processing::Effect,
        render::{
            render::{ComputeRenderable, RenderableHandle},
            render_view::{RenderTarget, RenderViewRole},
        },
        sprites::{sprite::Sprite, sprite_loader::SpriteSheet},
        texture::Texture,
    },
    egui::{self, FontFamily, FontId, Id, TextStyle, Ui, pos2, vec2},
    entities::meshes::Meshes,
    log,
    systems::{
        animation::{AnimationHandler, AnimationStep, AnimationType, Interpolation, StepState},
        camera::{
            Camera, CameraAnimator, CameraMode, CameraProjection, MovementKey, MovementPress,
        },
        light::{Light, LightSystem},
        physics::PhysicsSystem,
    },
    wgpu,
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

impl Website {
    fn initiate_audio_playground(&mut self, state: &mut State) {
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
        let bad_apple = state.graphics.asset("badapple.mid");
        let bad_apple_parsed = Midi::load_midi(&bad_apple);
        self.gui_context.piano_roll.midis.push(bad_apple_parsed);

        let wii_midi = state.graphics.asset("mii.mid");
        let wii_parsed = Midi::load_midi(&wii_midi);
        self.gui_context.piano_roll.midis.push(wii_parsed);
        self.gui_context.piano_roll.create_track_from_midi(0, 0);
    }
    fn initiate_playground(&mut self, gfx: &mut Graphics, camera_speed: f32) {
        let instances = instances_list_cube(vec3(0, 0, 0), vec3(40, 50, 40));

        let instances_len = instances.len();
        let animation_handler = AnimationHandler::new_from_instances(&instances, vec![]);
        let cube_mesh = Meshes::Cube
            .create()
            .make_mb(&mut gfx.engine.render_context);

        let box_ic = gfx.instances().from_instances(instances).build();

        let box_mat = gfx.material().shader("boxes").build();

        let box_entity = gfx.add_renderable(box_mat, cube_mesh, box_ic);

        gfx.add_entity((
            box_entity,
            markers::Boxes,
            animation_handler,
            Collider::Box {
                extents: vec3(1.0, 1.0, 1.0),
            },
        ));

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
            .mesh::<Vertex>()
            .input_data(&[compute_area])
            .shader("particle_render_with_mesh")
            .build();

        let particle_renderable = ComputeRenderable {
            rendering_handle: particle_rendering,
            mesh_handle: cube_mesh,
        };
        gfx.add_entity((particle_renderable,));

        let model_ic = gfx
            .instances()
            .from_instances(vec![Instance::new(
                [2.0, 2.0, 1.0].into(),
                vec3(100.0, 100.0, 100.0),
            )])
            .build();

        let model_mat = gfx
            .material()
            .texture_from_color([0.5, 0.5, 0.5], "datboi", 1, 0)
            .compute_buffer(compute, 2, 0)
            .shader("textured")
            .build();
        //
        let castle = gfx.asset("castle.vox");
        let chr_knight = gfx.asset("chr_knight.vox");
        let rust_logo = gfx.asset("rust.vox");
        let c_plus_plus = gfx.asset("cplusplus.vox");
        let c_sharp = gfx.asset("csharp.vox");
        let docker = gfx.asset("docker.vox");
        let hb_fugl = gfx.asset("hbfugl.vox");
        let femo_snake = gfx.asset("femoslangen.vox");
        self.voxel_handler.add_voxel(&castle, VoxelObjects::Castle);
        self.voxel_handler
            .add_voxel(&chr_knight, VoxelObjects::Viking);
        self.voxel_handler.add_voxel(&rust_logo, VoxelObjects::Rust);
        self.voxel_handler
            .add_voxel(&c_plus_plus, VoxelObjects::CPlusPLus);
        self.voxel_handler.add_voxel(&c_sharp, VoxelObjects::CSharp);
        self.voxel_handler
            .add_voxel(&docker, VoxelObjects::Containerization);
        self.voxel_handler
            .add_voxel(&hb_fugl, VoxelObjects::HandballBird);
        self.voxel_handler
            .add_voxel(&femo_snake, VoxelObjects::FemogfirsSlangen);

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
        let badapple_bin = gfx.asset("pixels.bin");

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
        gfx.post_process_effect(Effect::ChromaticAberration)
            .unwrap();
        self.camera_transition_handler.transition_map = camera_transition;
        self.bad_apple = badapple;
    }

    fn physics_playground(gfx: &mut Graphics, camera_speed: f32, ibl_maps: &Texture) {
        let sphere_mesh = Meshes::Sphere
            .create()
            .make_mb(&mut gfx.engine.render_context);

        let wood_diffuse = gfx.asset("pbr_test/viktors_peber/Wood_Planks_basecolor.png");
        let wood_normal = gfx.asset("pbr_test/viktors_peber/Wood_Planks_normal.png");
        let wood_metallic = gfx.asset("pbr_test/viktors_peber/viktor_peber_mettalic.png");
        let wood_roughness = gfx.asset("pbr_test/viktors_peber/Wood_Planks_roughness.png");
        let wood_ao = gfx.asset("pbr_test/viktors_peber/Wood_Planks_ambientocclusion.png");
        let texture = gfx
            .pbr_texture("sphere3")
            .diffuse_bytes(&wood_diffuse, wgpu::TextureFormat::Rgba8UnormSrgb)
            .normal_bytes(&wood_normal, wgpu::TextureFormat::Rgba8Unorm)
            .metallic_bytes(&wood_metallic, wgpu::TextureFormat::Rgba8Unorm)
            .roughness_bytes(&wood_roughness, wgpu::TextureFormat::Rgba8Unorm)
            .ao_bytes(&wood_ao, wgpu::TextureFormat::Rgba8Unorm)
            .build();
        let sphere_mat = gfx
            .material()
            .shader("pbr_textured")
            // .texture_from_color([0.0, 1.0, 0.0])
            .texture(&texture, 1, 0)
            .texture(ibl_maps, 2, 0)
            .build();

        let mut sphere_instances = Vec::with_capacity(5002);
        sphere_instances.push(Instance::new(vec3(10.0, 10.0, 0.0), vec3(1.0, 1.0, 1.0)));
        for i in 0..3000 {
            let random = rand::random::<f32>() + 9.0;
            sphere_instances.push(Instance::new(
                vec3(random, i as f32 + 30.0, random),
                vec3(1.0, 1.0, 1.0),
            ));
        }
        sphere_instances.push(Instance::new(vec3(9.9, 30.0, 0.0), vec3(1.0, 1.0, 1.0)));
        let sphere_ic = gfx.instances().from_instances(sphere_instances).build();
        gfx.add_physics_entity(
            sphere_mat,
            sphere_mesh,
            sphere_ic,
            Collider::Sphere { radius: 1.0 },
            RigidBody::new(1.0, BodyType::Dynamic),
        );
        let plane_mesh = Meshes::Plane
            .create()
            .make_mb(&mut gfx.engine.render_context);
        let plane_mat = gfx
            .material()
            .shader("textured")
            .texture_from_color([0.0, 1.0, 0.0], "color", 1, 0)
            .build();
        let boundary_size = 50.0;
        let wall_height = 50.0;

        let mut planes = Vec::with_capacity(5);
        let mut add_plane = |position: [f32; 3], rotation| {
            let mut instance =
                Instance::new(position.into(), vec3(boundary_size, 1.0, boundary_size));
            instance.transform.rotation = rotation;
            planes.push(instance);
        };
        // Floor
        add_plane([0.0, -50.0, 0.0], Quaternion::from_angle_x(Deg(0.0)));

        // +Z wall
        add_plane(
            [0.0, 0.0, boundary_size],
            Quaternion::from_angle_x(Deg(-90.0)),
        );

        // -Z wall -> normal points +Z
        add_plane(
            [0.0, 0.0, -boundary_size],
            Quaternion::from_angle_x(Deg(90.0)),
        );

        // +X wall -> normal points -X
        add_plane(
            [boundary_size, 0.0, 0.0],
            Quaternion::from_angle_z(Deg(90.0)),
        );

        // -X wall -> normal points +X
        add_plane(
            [-boundary_size, 0.0, 0.0],
            Quaternion::from_angle_z(Deg(-90.0)),
        );
        let plane_ic = gfx.instances().from_instances(planes).build();
        gfx.add_physics_entity(
            plane_mat,
            plane_mesh,
            plane_ic,
            Collider::Plane,
            RigidBody::new(100.0, BodyType::Static),
        );
    }
}

impl Game for Website {
    fn assets(&self) -> AssetManifest {
        AssetManifest::new().extend([
            "shaders/lights.wgsl",
            "shaders/boxes.wgsl",
            "shaders/compute.wgsl",
            "shaders/particle.wgsl",
            "shaders/particle_render.wgsl",
            "shaders/particle_render_with_mesh.wgsl",
            "shaders/textured.wgsl",
            "shaders/sprite.wgsl",
            "shaders/sprite_screen.wgsl",
            "pbr_test/cubemaps/solitude_night_4k.hdr",
            "pbr_test/cubemaps/kloofendal_48d_partly_cloudy_puresky_4k.hdr",
            "pbr_test/cubemaps/mealie_road_4k.hdr",
            "pbr_test/cubemaps/historic_cloister_passage_4k.hdr",
            "pbr_test/viktors_peber/HerringBone_INST_basecolor.PNG",
            "pbr_test/viktors_peber/HerringBone_INST_normal.PNG",
            "pbr_test/viktors_peber/viktor_peber_mettalic.png",
            "pbr_test/viktors_peber/HerringBone_INST_roughness.PNG",
            "pbr_test/viktors_peber/HerringBone_INST_ambientocclusion.PNG",
            "pbr_test/rusted_metal/rustediron2_basecolor.png",
            "pbr_test/rusted_metal/rustediron2_normal.png",
            "pbr_test/rusted_metal/rustediron2_metallic.png",
            "pbr_test/rusted_metal/rustediron2_roughness.png",
            "pbr_test/rusted_metal/blank_ao_2048x2048.png",
            "objs/sprites/test.json",
            "objs/sprites/test.png",
            "objs/gltfs/wolf/Wolf-Blender-2.82a.glb",
            "badapple.mid",
            "mii.mid",
            "castle.vox",
            "chr_knight.vox",
            "rust.vox",
            "cplusplus.vox",
            "csharp.vox",
            "docker.vox",
            "hbfugl.vox",
            "femoslangen.vox",
            "pixels.bin",
            "pbr_test/viktors_peber/Wood_Planks_basecolor.png",
            "pbr_test/viktors_peber/Wood_Planks_normal.png",
            "pbr_test/viktors_peber/Wood_Planks_roughness.png",
            "pbr_test/viktors_peber/Wood_Planks_ambientocclusion.png",
        ])
    }

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
                camera.set(MovementKey::RotateLeft, MovementPress::Override);
                camera.set_camera_mode(CameraMode::Animated);
                self.bad_apple.init_camera(camera);
                self.bad_apple.update_camera(camera);
            });

            world.query_first::<(&RenderableHandle, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);

                gfx.change_renderable_shader(*render, "lights");
            });
            println!("Test");

            self.bad_apple.toggle = true;
            log::warn!("EE started!");
        }
        if buffer_string == "ihatefun" && self.bad_apple.toggle {
            world.query_first::<&mut Camera>(|camera| {
                camera.set(MovementKey::RotateLeft, MovementPress::NotPressed);
                camera.set_camera_mode(CameraMode::Free);
                self.bad_apple.reset_camera(camera);
            });

            world.query_first::<(&RenderableHandle, &mut AnimationHandler)>(|(render, ah)| {
                self.voxel_handler
                    .transition_to_point_list(self.bad_apple.get_frame(), ah, 1.0);
                gfx.change_renderable_shader(*render, "boxes");
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
                    world.query_first::<(&RenderableHandle, &mut AnimationHandler)>(
                        |(renderable, ah)| {
                            let ic = gfx.get_instance_controller(*renderable);
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
                world.query_first::<(&RenderableHandle, &mut AnimationHandler)>(
                    |(renderable, ah)| {
                        let ic = gfx.get_instance_controller(*renderable);
                        ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                        self.voxel_handler.transition_to_point_list(
                            self.bad_apple.get_frame(),
                            ah,
                            1.0,
                        );
                    },
                );
                world.query_first::<&mut Camera>(|camera| {
                    log::warn!("{:?}", camera.eye.z);
                    self.bad_apple.update_camera(camera)
                });

                self.bad_apple.index += 1;
                self.bad_apple.elapsed -= target;
            }
        }
    }

    fn process_event(
        &mut self,
        event: &winit::event::WindowEvent,
        screen: &winit::dpi::PhysicalSize<u32>,
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
                        world.query_first::<(&RenderableHandle, &mut AnimationHandler)>(
                            |(render, ah)| {
                                let ic = gfx.get_instance_controller(*render);
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
                            .query::<(&RenderableHandle, &mut AnimationHandler)>();
                        println!("query len: {}", query.iter().len());
                        let (render, ah) = query.iter().next().expect("No AH");

                        let ic = gfx.get_instance_controller(*render);

                        ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                        self.voxel_handler.transition_to_object(
                            VoxelObjects::FemogfirsSlangen,
                            ah,
                            true,
                            1.0,
                        );
                        gfx.change_renderable_shader(*render, "boxes");
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
                            .query::<(&RenderableHandle, &mut AnimationHandler)>();
                        let (render, ah) = query.iter().next().expect("No AH");

                        let ic = gfx.get_instance_controller(*render);

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
                        ElementState::Released => {
                            let ndc_scale = gfx
                                .engine
                                .render_context
                                .post_processing
                                .display_to_render_ndc_scale();
                            let mut test = None;
                            world.query_first::<&Camera>(|camera| {
                                test = Some(camera.screen_to_world_ray(
                                    self.cursor_pos.x,
                                    self.cursor_pos.y,
                                    screen.width as f32,
                                    screen.height as f32,
                                    ndc_scale,
                                ));
                            });
                            if let Some((position, direction)) = test {
                                let ray = Ray {
                                    direction,
                                    origin: position,
                                };

                                let closest = ray.precise_intersects(&world, gfx);

                                if let Some(hit) = closest {
                                    let mut query =
                                        world
                                            .entities
                                            .query_one::<(&RenderableHandle, &markers::Boxes)>(
                                                hit.entity_handle,
                                            );

                                    if let Ok((render, _)) = query.get() {
                                        gfx.get_instance_controller(*render)
                                            .instances_mut()
                                            .get_mut(hit.instance_index)
                                            .unwrap()
                                            .should_render = false;
                                    }
                                }

                                println!("{:?}", closest)
                            }
                        }
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
    }

    fn setup(&mut self, state: &mut State) {
        let gfx = &mut state.graphics;

        let main_scene = gfx.new_scene("test_scene1", |_gfx, _world| {});

        // Initiate the main view. Its camera belongs to the view rather than the scene.
        let main_target = RenderTarget::window(state.size);
        let mut camera = Camera::new(main_target.clone(), 75.0, 50.0);
        camera.eye = Point3 {
            x: -17.16,
            y: 6.1,
            z: -12.4,
        };
        camera.yaw = 25.0;
        camera.pitch = -1.4;
        camera.projection = CameraProjection::Perspective;
        camera.update_camera(gfx.dt());
        camera.update_forward();
        let camera_speed = camera.speed;
        let main_view = gfx.new_render_view(
            "main",
            main_scene,
            main_target.clone(),
            RenderViewRole::Main,
        );

        gfx.render_views.get_render_view_mut(main_view).camera = camera;

        //Initiates lighting
        let light = Light {
            position: cgmath::vec3(5.0, 5.0, 1.0),
            color: cgmath::vec3(1.0, 1.0, 1.0),
            intensity: 150.0,
        };

        let light2 = Light {
            position: cgmath::vec3(-5.0, -5.0, 1.0),
            color: cgmath::vec3(1.0, 1.0, 1.0),
            intensity: 150.0,
        };
        let light_system = LightSystem::init(
            &[light.clone(), light2.clone()],
            &gfx.engine.render_context.device,
        );
        gfx.add_system(System::view(light_system));

        let physics_system = PhysicsSystem::new(vec3(0.0, -9.81, 0.0));
        gfx.add_system(System::scene(physics_system));

        //Initiate Shaders
        gfx.shader_asset("lights", "shaders/lights.wgsl").unwrap();
        gfx.shader_asset("boxes", "shaders/boxes.wgsl").unwrap();
        gfx.shader_asset("compute", "shaders/compute.wgsl").unwrap();
        gfx.shader_asset("particle", "shaders/particle.wgsl")
            .unwrap();
        gfx.shader_asset("particle_render", "shaders/particle_render.wgsl")
            .unwrap();
        gfx.shader_asset(
            "particle_render_with_mesh",
            "shaders/particle_render_with_mesh.wgsl",
        )
        .unwrap();
        gfx.shader_asset("textured", "shaders/textured.wgsl")
            .unwrap();
        //Initiate meshes

        let cube_mesh = Meshes::Cube
            .create()
            .make_mb(&mut gfx.engine.render_context);

        let sphere_mesh = Meshes::Sphere
            .create()
            .make_mb(&mut gfx.engine.render_context);

        let pbr_constants = PhysicsBasedRenderingConstants {
            metallic: 0.0,
            roughness: 0.0,
            ao: 0.0,
        };
        let buffer = Buffer::new_init(
            &[pbr_constants],
            &gfx.engine.render_context.device,
            BufferType::UniformBuffer(UniformParameters::default()),
        );

        gfx.register_buffer(buffer.clone(), "material_test");

        let solitude = gfx.asset("pbr_test/cubemaps/solitude_night_4k.hdr");
        let cubemap_texture = gfx.texture("cubemap").hdri_cubemap(&solitude);

        let ibl_maps = gfx.texture("ibl").ibl_maps(&cubemap_texture);
        Website::physics_playground(gfx, camera_speed, &ibl_maps);
        // {
        //     let world = gfx.get_world();
        //     let mut world = world.borrow_mut();
        //     gfx.add_skybox(&cubemap_texture, &mut world);
        // }
        let model_ic = gfx
            .instances()
            .origin(vec3(10.0, 10.0, 10.0))
            .uniform_scale(2.0)
            .build();

        let sprite_json = gfx.asset("objs/sprites/test.json");
        let sprite_png = gfx.asset("objs/sprites/test.png");
        let sprite_sheet = SpriteSheet::from_bytes(gfx, &sprite_json, &sprite_png);

        let sprite_sheet_texture =
            gfx.engine.render_context.gpu_objects.textures[sprite_sheet.texture_handle].clone();

        gfx.shader_asset("sprite", "shaders/sprite.wgsl").unwrap();
        gfx.shader_asset("sprite_screen", "shaders/sprite_screen.wgsl")
            .unwrap();
        let sprite_material = gfx
            .material_instance::<SpriteInstanceLayout>()
            .texture(&sprite_sheet_texture, 1, 0)
            .shader("sprite")
            .build();
        let screen_sprite_material = gfx
            .material_instance::<SpriteInstanceLayout>()
            .texture(&sprite_sheet_texture, 1, 0)
            .shader("sprite_screen")
            .build();

        let handle = gfx.engine.resources.sprite_sheets.insert(sprite_sheet);

        let _sprite = Sprite::new(
            gfx,
            handle,
            "Bingo trolden.aseprite",
            sprite_material,
            [30.0, 0.0, 0.0].into(),
        );

        let _sprite = Sprite::new(
            gfx,
            handle,
            "Håndboldfuglen.aseprite",
            sprite_material,
            [31.5, 0.0, 0.0].into(),
        );

        let _sprite = Sprite::new(
            gfx,
            handle,
            "85Slange.aseprite",
            sprite_material,
            [33.0, 0.0, 0.0].into(),
        );

        // let _screen_sprite = Sprite::new_screen_space(
        //     gfx,
        //     handle,
        //     "Bingo trolden.aseprite",
        //     screen_sprite_material,
        //     [50.0, 70.0, 0.0].into(),
        // );
        //
        // let _screen_sprite = Sprite::new_screen_space(
        //     gfx,
        //     handle,
        //     "Håndboldfuglen.aseprite",
        //     screen_sprite_material,
        //     [125.0, 70.0, 0.0].into(),
        // );
        //
        // let _screen_sprite = Sprite::new_screen_space(
        //     gfx,
        //     handle,
        //     "85Slange.aseprite",
        //     screen_sprite_material,
        //     [210.0, 70.0, 0.0].into(),
        // );

        // let wolf = gfx.asset("objs/gltfs/wolf/Wolf-Blender-2.82a.glb");
        // let model = Model::load_glb(gfx, &wolf, model_ic, sphere_mat);
        // gfx.add_entity((model,));

        {
            let solitude =
                gfx.asset("pbr_test/cubemaps/kloofendal_48d_partly_cloudy_puresky_4k.hdr");
            let cubemap_texture = gfx.texture("cubemap1").hdri_cubemap(&solitude);
            let world = gfx.world(main_scene);
            ball_setup(gfx, &mut world.borrow_mut(), &cubemap_texture);
        }
        gfx.new_scene("test_scene2", |gfx: &mut Graphics, world| {
            let solitude = gfx.asset("pbr_test/cubemaps/solitude_night_4k.hdr");
            let cubemap_texture = gfx.texture("cubemap2").hdri_cubemap(&solitude);
            ball_setup(gfx, world, &cubemap_texture);
        });
        gfx.new_scene("test_scene3", |gfx: &mut Graphics, world| {
            let solitude = gfx.asset("pbr_test/cubemaps/mealie_road_4k.hdr");
            let cubemap_texture = gfx.texture("cubemap3").hdri_cubemap(&solitude);
            ball_setup(gfx, world, &cubemap_texture);
        });
        let scene_handle = gfx.new_scene("test_scene4", |gfx: &mut Graphics, world| {
            let solitude = gfx.asset("pbr_test/cubemaps/historic_cloister_passage_4k.hdr");
            let cubemap_texture = gfx.texture("cubemap4").hdri_cubemap(&solitude);

            ball_setup(gfx, world, &cubemap_texture);
        });

        let main_target = RenderTarget::window(state.size);
        let mut camera = Camera::new(main_target.clone(), 75.0, 50.0);
        camera.eye = Point3 {
            x: -17.16,
            y: 6.1,
            z: -12.4,
        };
        camera.yaw = 30.0;
        camera.pitch = -1.4;
        camera.projection = CameraProjection::Perspective;
        camera.update_camera(gfx.dt());
        camera.update_forward();
        let camera_speed = camera.speed;
        let main_view = gfx.new_render_view(
            "alternate",
            scene_handle,
            main_target.clone(),
            RenderViewRole::Auxiliary,
        );

        gfx.render_views.get_render_view_mut(main_view).camera = camera;
        state.event_registry.key(KeyCode::Digit1, switch_cubemap_1);

        state.event_registry.key(KeyCode::Digit2, switch_cubemap_2);
        state.event_registry.key(KeyCode::Digit3, switch_cubemap_3);
        state.event_registry.key(KeyCode::Digit4, switch_cubemap_4);

        // self.initiate_playground(gfx, camera_speed);
        // self.initiate_audio_playground(state);
    }

    fn resize(&mut self, gfx: &mut Graphics, _world: Ref<'_, World>) {
        // let camera = &mut gfx
        //     .render_views
        //     .get_render_view_from_name_mut("main")
        //     .camera;
        // println!("{:?}", camera.aspect);
        // let new_fov = map_value(camera.aspect, 0.8, 1.88, 25.0, 55.0);
        // camera.fovy = new_fov;
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
                "Piao Roll",
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

        TuiWindow::new(
            Id::new("Material Editor"),
            "Material Sliders",
            pos2(100.0, 200.0),
            vec2(300.0, 200.0),
            TuiBorder::HardLines,
        )
        .show(ui, |ui| {
            self.gui_context.bc.ui(ui, gfx, "material_test");
        })
    }
}

impl Website {}

fn ball_setup(gfx: &mut Graphics, world: &mut World, cubemap_texture: &Texture) {
    let sphere_mesh = Meshes::Sphere
        .create()
        .make_mb(&mut gfx.engine.render_context);

    gfx.add_skybox(&cubemap_texture, world);
    let ibl_maps = gfx.texture("ibl").ibl_maps(&cubemap_texture);

    // let kloofendal = gfx.asset("pbr_test/cubemaps/kloofendal_48d_partly_cloudy_puresky_4k.hdr");
    // let cubemap_irradiance = gfx.texture("cubemap").hdri_irradiance_map(&kloofendal);

    let herring_diffuse = gfx.asset("pbr_test/viktors_peber/HerringBone_INST_basecolor.PNG");
    let herring_normal = gfx.asset("pbr_test/viktors_peber/HerringBone_INST_normal.PNG");
    let herring_metallic = gfx.asset("pbr_test/viktors_peber/viktor_peber_mettalic.png");
    let herring_roughness = gfx.asset("pbr_test/viktors_peber/HerringBone_INST_roughness.PNG");
    let herring_ao = gfx.asset("pbr_test/viktors_peber/HerringBone_INST_ambientocclusion.PNG");
    let texture = gfx
        .pbr_texture("sphere2")
        .diffuse_bytes(&herring_diffuse, wgpu::TextureFormat::Rgba8UnormSrgb)
        .normal_bytes(&herring_normal, wgpu::TextureFormat::Rgba8Unorm)
        .metallic_bytes(&herring_metallic, wgpu::TextureFormat::Rgba8Unorm)
        .roughness_bytes(&herring_roughness, wgpu::TextureFormat::Rgba8Unorm)
        .ao_bytes(&herring_ao, wgpu::TextureFormat::Rgba8Unorm)
        .build();
    let sphere_mat = gfx
        .material()
        .shader("pbr_textured")
        // .texture_from_color([0.0, 1.0, 0.0])
        .texture(&texture, 1, 0)
        .texture(&ibl_maps, 2, 0)
        .build();

    let sphere_ic = gfx.instances().build();
    let sphere_entity = gfx.add_renderable(sphere_mat, sphere_mesh, sphere_ic);
    world.add_entity((sphere_entity, Collider::Sphere { radius: 1.0 }));
    let rusted_diffuse = gfx.asset("pbr_test/rusted_metal/rustediron2_basecolor.png");
    let rusted_normal = gfx.asset("pbr_test/rusted_metal/rustediron2_normal.png");
    let rusted_metallic = gfx.asset("pbr_test/rusted_metal/rustediron2_metallic.png");
    let rusted_roughness = gfx.asset("pbr_test/rusted_metal/rustediron2_roughness.png");
    let rusted_ao = gfx.asset("pbr_test/rusted_metal/blank_ao_2048x2048.png");
    let texture2 = gfx
        .pbr_texture("sphere1")
        .diffuse_bytes(&rusted_diffuse, wgpu::TextureFormat::Rgba8UnormSrgb)
        .normal_bytes(&rusted_normal, wgpu::TextureFormat::Rgba8Unorm)
        .metallic_bytes(&rusted_metallic, wgpu::TextureFormat::Rgba8Unorm)
        .roughness_bytes(&rusted_roughness, wgpu::TextureFormat::Rgba8Unorm)
        .ao_bytes(&rusted_ao, wgpu::TextureFormat::Rgba8Unorm)
        .build();
    let sphere_mat2 = gfx
        .material()
        .shader("pbr_textured")
        // .texture_from_color([0.0, 1.0, 0.0])
        .texture(&texture2, 1, 0)
        .texture(&ibl_maps, 2, 0)
        .build();

    let sphere_ic2 = gfx.instances().origin(vec3(3.0, 0.0, 0.0)).build();
    let sphere_entity2 = gfx.add_renderable(sphere_mat2, sphere_mesh, sphere_ic2);
    world.add_entity((sphere_entity2, Collider::Sphere { radius: 1.0 }));
}

fn switch_cubemap_1(_game: &mut Website, context: &mut KeyboardEventContext) {
    let scene = context.gfx.scenes.scenes_lookup["test_scene1"];
    context
        .gfx
        .render_views
        .get_render_view_from_name_mut("main")
        .scene = scene;
    context.gfx.set_active_gameplay_scene(scene);
}

fn switch_cubemap_2(_game: &mut Website, context: &mut KeyboardEventContext) {
    let scene = context.gfx.scenes.scenes_lookup["test_scene2"];
    context
        .gfx
        .render_views
        .get_render_view_from_name_mut("main")
        .scene = scene;
    context.gfx.set_active_gameplay_scene(scene);
}

fn switch_cubemap_3(_game: &mut Website, context: &mut KeyboardEventContext) {
    let scene = context.gfx.scenes.scenes_lookup["test_scene3"];
    let old_view = context
        .gfx
        .render_views
        .get_render_view_from_name_mut("alternate");

    old_view.role = RenderViewRole::Auxiliary;
    let new_view = context
        .gfx
        .render_views
        .get_render_view_from_name_mut("main");

    new_view.role = RenderViewRole::Main;
    new_view.scene = scene;
}
fn switch_cubemap_4(_game: &mut Website, context: &mut KeyboardEventContext) {
    let old_view = context
        .gfx
        .render_views
        .get_render_view_from_name_mut("main");

    old_view.role = RenderViewRole::Auxiliary;
    let new_view = context
        .gfx
        .render_views
        .get_render_view_from_name_mut("alternate");

    new_view.role = RenderViewRole::Main;

    let scene_handle = new_view.scene.clone();

    context.gfx.set_active_gameplay_scene(scene_handle);
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
