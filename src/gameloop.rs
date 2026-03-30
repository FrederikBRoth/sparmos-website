use std::collections::BTreeMap;

use sparmos_engine::entity::{
    core::{
        engine::Engine,
        instance::{GpuInstance, Instance, InstanceController},
        material::MaterialBuilder,
        render::Renderable,
    },
    entities::cube::{self},
    systems::{
        camera::{Camera, CameraSystem},
        light::{Light, LightSystem},
    },
};
use sparmos_engine::{
    application::state::{Game, State, map_value},
    cgmath::{self, *},
    helpers::animation::AnimationHandler,
    log,
    wgpu::{self},
    winit::{
        self,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::KeyCode,
    },
};

use crate::{
    markers::{self},
    transition::TransitionHandler,
    voxel_builder::{VoxelHandler, VoxelObjects, instances_list_cube},
};

pub struct Website {
    pub score: u32,
    pub counter: usize,
    pub cursor_pos: PhysicalPosition<f32>,
    pub cursor_delta: (f64, f64),
    pub voxel_handler: VoxelHandler<VoxelObjects>,
    pub transition_handler: TransitionHandler<VoxelObjects>,
}

impl Default for Website {
    fn default() -> Self {
        Self {
            score: 0,
            counter: 0,
            cursor_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            cursor_delta: (0.0, 0.0),
            voxel_handler: VoxelHandler::<VoxelObjects>::new(),
            transition_handler: TransitionHandler::<VoxelObjects>::new(BTreeMap::new()),
        }
    }
}

impl Game for Website {
    fn update(&mut self, dt: std::time::Duration, engine: &mut Engine) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();
        let mut query = engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = engine.resources.get_system_mut::<CameraSystem>();
        camera_system.update_camera(dt, &engine.render_context, camera);

        if let Some(scroll_y) = engine.args.get("scrolly")
            && let Some(scroll_y) = scroll_y.downcast_ref::<f64>()
            && let Some(transition) = self
                .transition_handler
                .get_transition_once(*scroll_y as i64)
        {
            log::warn!("Test123");
            match transition.clone() {
                VoxelObjects::Home => {}
                _ => {
                    let mut query = engine.world.query::<(&Renderable, &mut AnimationHandler)>();
                    let (render, ah) = query.iter().next().expect("No AH");

                    let ic = engine
                        .render_context
                        .gpu_objects
                        .instance_controllers
                        .get_mut(render.instance_controller_handle)
                        .unwrap();

                    ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                    self.voxel_handler
                        .transition_to_object(transition, ah, true, 1.0);
                }
            }
        }
    }

    fn process_event(
        &mut self,
        event: &winit::event::WindowEvent,
        _screen: &winit::dpi::PhysicalSize<u32>,
        engine: &mut Engine,
    ) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();
        // let (entity, camera) = state
        let mut query = engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = engine.resources.get_system_mut::<CameraSystem>();
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: winit::keyboard::PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let var_name = *state == ElementState::Pressed;
                let is_pressed = var_name;
                match keycode {
                    KeyCode::Space => {}
                    KeyCode::PageUp => match state {
                        winit::event::ElementState::Pressed => {
                            let mut query =
                                engine.world.query::<(&Renderable, &mut AnimationHandler)>();
                            let (render, ah) = query.iter().next().expect("No AH");

                            let ic = engine
                                .render_context
                                .gpu_objects
                                .instance_controllers
                                .get_mut(render.instance_controller_handle)
                                .unwrap();
                            ah.reset_instance_position_to_current_position(
                                ic.instances_mut().as_mut(),
                            );
                            self.voxel_handler.transition_to_object(
                                VoxelObjects::HandballBird,
                                ah,
                                true,
                                1.0,
                            );
                        }
                        _ => {}
                    },

                    KeyCode::PageDown => match state {
                        winit::event::ElementState::Pressed => {
                            let mut query =
                                engine.world.query::<(&Renderable, &mut AnimationHandler)>();
                            let (render, ah) = query.iter().next().expect("No AH");

                            let ic = engine
                                .render_context
                                .gpu_objects
                                .instance_controllers
                                .get_mut(render.instance_controller_handle)
                                .unwrap();

                            ah.reset_instance_position_to_current_position(
                                ic.instances_mut().as_mut(),
                            );
                            self.voxel_handler.transition_to_object(
                                VoxelObjects::FemogfirsSlangen,
                                ah,
                                true,
                                1.0,
                            );
                        }
                        _ => {}
                    },
                    _ => (),
                }
            }
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

            _ => (),
        }
        camera_system.process_events(event, camera);
    }

    fn setup(&mut self, state: &mut State) {
        let engine = &mut state.engine;

        //Initiates Camera system
        let camera = Camera::new(PhysicalSize::new(
            state.size.width as f32,
            state.size.height as f32,
        ));
        let camera_system = CameraSystem::new(75.0, 50.0, &engine.render_context.device, &camera);
        //registers system and creates bind_group

        engine.add_entity((camera,));
        engine.add_system(camera_system);
        //Initiates lighting
        let light = Light {
            position: cgmath::vec3(200.0, 200.0, 1.0),
            color: cgmath::vec3(1.0, 0.0, 0.0),
        };

        let light2 = Light {
            position: cgmath::vec3(-200.0, -200.0, 1.0),
            color: cgmath::vec3(0.0, 1.0, 0.0),
        };
        let light_system = LightSystem::init(
            &[light.clone(), light2.clone()],
            &engine.render_context.device,
        );
        engine.add_system(light_system);

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
            .add_layout("camera", engine.resources.get_system::<CameraSystem>())
            .add_layout("light", engine.resources.get_system::<LightSystem>())
            .add_shader("lights")
            .build(&cube_mesh, &light_ic, &mut engine.render_context);

        let light_entity = Renderable {
            material_handle: light_mat,
            instance_controller_handle: light_ic,
            mesh_handle: cube_mesh,
        };

        engine.add_entity((light_entity, markers::Light));
        let instances = instances_list_cube(vec3(0, 0, 0), vec3(40, 50, 40));

        let animation_handler = AnimationHandler::new(&instances, vec![]);
        let box_ic = InstanceController::<GpuInstance>::new(instances, &mut engine.render_context);

        let box_mat = MaterialBuilder::new()
            .add_layout("camera", engine.resources.get_system::<CameraSystem>())
            .add_layout("light", engine.resources.get_system::<LightSystem>())
            .add_shader("boxes")
            .build(&cube_mesh, &box_ic, &mut engine.render_context);
        let box_entity = Renderable {
            material_handle: box_mat,
            instance_controller_handle: box_ic,
            mesh_handle: cube_mesh,
        };

        engine.add_entity((box_entity, markers::Boxes, animation_handler));
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
    }

    fn resize(&mut self, engine: &mut Engine) {
        let mut query = engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = engine.resources.get_system_mut::<CameraSystem>();

        camera.aspect =
            engine.render_context.config.width as f32 / engine.render_context.config.height as f32;
        println!("{:?}", camera.aspect);
        let new_fov = map_value(camera.aspect, 0.8, 1.88, 25.0, 55.0);
        camera.fovy = new_fov;
        // if camera.aspect < camera.camera_animator.aspect_ratio_limit {
        //     let eye = Point3::new(110.0, 90.0, -130.0);
        //     let target = Point3::new(20.0, 25.0, 20.0);
        //     camera.eye = eye;
        //     camera.target = target;
        //     camera.fovy = 90.0;
        // }
    }
}
