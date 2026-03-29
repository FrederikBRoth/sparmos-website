use std::{collections::BTreeMap, sync::Arc, time::Duration};

use sparmos_engine::{
    application::state::{Core, DeviceBackend, Game, State, map_value},
    cgmath::{self, *},
    egui::{self, Color32, Rect, Response, Sense, Ui, Vec2},
    entity::{
        core::{
            buffer::{Buffer, BufferType},
            geometry::Primitive,
            instance::{GpuInstance, Instance, InstanceController, InstanceRaw},
            material::MaterialBuilder,
            render::Renderable,
        },
        entities::cube::{self, new},
        systems::{
            camera::{Camera, CameraSystem},
            light::{Light, LightSystem},
        },
        texture::Texture,
    },
    helpers::{animation::AnimationHandler, line_trace::line_trace_square},
    log, web_time,
    wgpu::{self},
    winit::{
        self,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::KeyCode,
    },
};

use crate::{
    markers::{self, Boxes},
    transition::{self, TransitionHandler},
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
    fn update(&mut self, dt: std::time::Duration, core: &mut Core) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();
        let mut query = core.engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = core.engine.resources.get_system_mut::<CameraSystem>();
        camera_system.update_camera(dt, &core.render_context, camera);

        if let Some(scroll_y) = core.args.get("scrolly")
            && let Some(scroll_y) = scroll_y.downcast_ref::<f64>()
            && let Some(transition) = self
                .transition_handler
                .get_transition_once(*scroll_y as i64)
        {
            log::warn!("Test123");
            match transition.clone() {
                VoxelObjects::Home => {}
                _ => {
                    let mut query = core
                        .engine
                        .world
                        .query::<(&Renderable, &mut AnimationHandler)>();
                    let (render, ah) = query.iter().next().expect("No AH");

                    let ic = core
                        .render_context
                        .gpu_objects
                        .instance_controllers
                        .get_mut(render.instance_controller_handle)
                        .unwrap();

                    ah.reset_instance_position_to_current_position(ic.instances_mut().as_mut());
                    self.voxel_handler
                        .transition_to_object(transition, ah, true, 1.0);
                }

                _ => {}
            }
        }
    }

    fn process_event(
        &mut self,
        event: &winit::event::WindowEvent,
        screen: &winit::dpi::PhysicalSize<u32>,
        core: &mut Core,
    ) {
        // let mut camera_system = self.world.query::<&mut CameraSystem>();
        // let camera_system = camera_system.iter().next().unwrap();
        // let (entity, camera) = state
        let mut query = core.engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = core.engine.resources.get_system_mut::<CameraSystem>();
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
                            let mut query = core
                                .engine
                                .world
                                .query::<(&Renderable, &mut AnimationHandler)>();
                            let (render, ah) = query.iter().next().expect("No AH");

                            let ic = core
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
                            let mut query = core
                                .engine
                                .world
                                .query::<(&Renderable, &mut AnimationHandler)>();
                            let (render, ah) = query.iter().next().expect("No AH");

                            let ic = core
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
        let camera = Camera::new(PhysicalSize::new(
            state.size.width as f32,
            state.size.height as f32,
        ));
        let camera_system =
            CameraSystem::new(75.0, 50.0, &state.core.render_context.device, &camera);
        //registers system and creates bind_group

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
            &state.core.render_context.device,
        );
        let engine = &mut state.core.engine;
        engine.add_entity((camera,));
        engine.add_system(camera_system, &state.core.render_context.device);
        engine.add_system(light_system, &state.core.render_context.device);
        let primitive_shader =
            state
                .core
                .render_context
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("PrimitiveShader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/lights.wgsl").into()),
                });

        let box_shader =
            state
                .core
                .render_context
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("PrimitiveShader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/boxes.wgsl").into()),
                });
        state
            .core
            .render_context
            .shaders
            .insert("lights".to_string(), primitive_shader);

        state
            .core
            .render_context
            .shaders
            .insert("boxes".to_string(), box_shader);

        let cube_mesh = cube::new().make_mb(&state.core.render_context.device);
        let light_ic = InstanceController::<GpuInstance>::new(
            vec![
                Instance::new([200.0, 200.0, 1.0].into(), 10.0),
                Instance::new([-200.0, -200.0, 1.0].into(), 10.0),
            ],
            &state.core.render_context.device,
        );
        let light_mat = MaterialBuilder::new()
            .add_layout("camera", engine.resources.get_system::<CameraSystem>())
            .add_layout("light", engine.resources.get_system::<LightSystem>())
            .add_shader("lights")
            .build(
                &cube_mesh.buffer_layout,
                &state.core.render_context,
                &light_ic.buffer_layout,
            );

        let instances = instances_list_cube(vec3(0, 0, 0), vec3(50, 50, 50));
        let box_ic1 =
            InstanceController::<GpuInstance>::new(instances, &state.core.render_context.device);

        let box_mat = MaterialBuilder::new()
            .add_layout("camera", engine.resources.get_system::<CameraSystem>())
            .add_layout("light", engine.resources.get_system::<LightSystem>())
            .add_shader("boxes")
            .build(
                &cube_mesh.buffer_layout,
                &state.core.render_context,
                &box_ic1.buffer_layout,
            );

        let gpu_objects = &mut state.core.render_context.gpu_objects;

        let light_ic = gpu_objects.instance_controllers.insert(Box::new(light_ic));
        let light_mesh = gpu_objects.meshes.insert(cube_mesh);
        let light_mat = gpu_objects.materials.insert(light_mat);

        let boxes_mat = gpu_objects.materials.insert(box_mat);
        let light_entity = Renderable {
            material_handle: light_mat,
            instance_controller_handle: light_ic,
            mesh_handle: light_mesh,
        };
        engine.add_entity((light_entity, markers::Light));

        // for x in 0..10 {
        let instances = instances_list_cube(vec3(0, 0, 0), vec3(40, 50, 40));

        let mut animation_handler = AnimationHandler::new(&instances, vec![]);
        let box_ic =
            InstanceController::<GpuInstance>::new(instances, &state.core.render_context.device);

        let boxes_ic = gpu_objects.instance_controllers.insert(Box::new(box_ic));
        let box_entity = Renderable {
            material_handle: boxes_mat,
            instance_controller_handle: boxes_ic,
            mesh_handle: light_mesh,
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
        let transition_scroll_positions: Vec<i64> =
            vec![300, 1300, 2100, 2950, 3850, 4750, 5599, 6485, 7200];

        let mut transition_map = BTreeMap::new();
        transition_map.insert(
            *transition_scroll_positions.get(0).unwrap(),
            VoxelObjects::Home,
        );
        transition_map.insert(
            *transition_scroll_positions.get(1).unwrap(),
            VoxelObjects::CSharp,
        );
        transition_map.insert(
            *transition_scroll_positions.get(2).unwrap(),
            VoxelObjects::Rust,
        );
        transition_map.insert(
            *transition_scroll_positions.get(3).unwrap(),
            VoxelObjects::CPlusPLus,
        );
        transition_map.insert(
            *transition_scroll_positions.get(4).unwrap(),
            VoxelObjects::Containerization,
        );
        transition_map.insert(
            *transition_scroll_positions.get(5).unwrap(),
            VoxelObjects::CPlusPLus,
        );
        transition_map.insert(
            *transition_scroll_positions.get(6).unwrap(),
            VoxelObjects::CSharp,
        );
        transition_map.insert(
            *transition_scroll_positions.get(7).unwrap(),
            VoxelObjects::Rust,
        );
        transition_map.insert(
            *transition_scroll_positions.get(8).unwrap(),
            VoxelObjects::CPlusPLus,
        );

        self.transition_handler.transition_map = transition_map;
    }

    fn resize(&mut self, core: &mut Core) {
        let mut query = core.engine.world.query::<&mut Camera>();
        let camera = query.iter().next().expect("No camera found");
        let camera_system = core.engine.resources.get_system_mut::<CameraSystem>();

        camera.aspect =
            core.render_context.config.width as f32 / core.render_context.config.height as f32;
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
