// use crate::helpers::animation::AnimationType;
// use crate::helpers::animation::{
//     AnimationHandler, AnimationStep, AnimationTransition, EaseInEaseOut,
// };
use cgmath::{MetricSpace, Vector3};
use dot_vox::load_bytes;
use rand::{rng, seq::SliceRandom};
use sparmos_engine::cgmath::{InnerSpace, Rotation3, Zero};
use sparmos_engine::entity::core::instance::Instance;
use sparmos_engine::helpers::animation::{
    AnimationHandler, AnimationStep, AnimationTransition, AnimationType, StepState,
};
use sparmos_engine::{cgmath, log};
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VoxelObjects {
    Home,
    CSharp,
    CPlusPLus,
    Rust,
    Containerization,
    Castle,
    Viking,
    Buttplug,
    HandballBird,
    FemogfirsSlangen,
    BadApple,
}
#[derive(Clone)]
pub struct Object {
    pub cubes: Vec<Vector3<f32>>,
    pub color: Vec<Vector3<f32>>,
}

pub struct VoxelHandler<T: Eq + std::hash::Hash> {
    pub voxels: Vec<Object>,
    pub voxels_map: HashMap<T, Object>,
    pub custom_voxel_map: HashMap<T, Vec<Object>>,
    pub current_voxel: Option<T>,
    pub current_cubes: Vec<usize>,
    pub current_object: usize,

    temp_indices: Vec<usize>,
    temp_flags: Vec<bool>,
}
impl<T: Eq + std::hash::Hash + Clone> VoxelHandler<T> {
    pub fn new() -> Self {
        Self {
            voxels: vec![],
            voxels_map: HashMap::new(),
            custom_voxel_map: HashMap::new(),
            current_cubes: vec![],
            current_object: 0,
            current_voxel: None,

            temp_indices: Vec::new(),
            temp_flags: Vec::new(),
        }
    }

    pub fn add_voxel(&mut self, path: &[u8], voxel_type: T) {
        match load_bytes(path) {
            Ok(scene) => {
                let palette = scene.palette.clone();
                for model in scene.models {
                    let new_voxel = Object {
                        cubes: model
                            .voxels
                            .clone()
                            .iter()
                            .map(|v| Vector3::new(v.x as f32, v.z as f32, v.y as f32))
                            .collect(),
                        color: model
                            .voxels
                            .clone()
                            .iter()
                            .map(|v| {
                                let color = palette.get(v.i as usize).unwrap();
                                Vector3::new(
                                    get_srgb(color.r),
                                    get_srgb(color.g),
                                    get_srgb(color.b),
                                )
                            })
                            .collect(),
                    };
                    self.voxels_map.insert(voxel_type.clone(), new_voxel);
                }
            }
            Err(err) => {
                log::warn!("Failed to load voxel file");
                eprintln!("Failed to load .vox file: {}", err);
            }
        }
    }

    pub fn add_custom_voxel(&mut self, vector_list: &Vec<Vector3<f32>>, voxel_type: T) {
        match self.custom_voxel_map.get(&voxel_type) {
            Some(_list) => {
                let new_voxel = Object {
                    cubes: vector_list.clone(),
                    color: vector_list
                        .clone()
                        .iter()
                        .enumerate()
                        .map(|(_x, _)| Vector3::new(1.0 as f32, 1.0 as f32, 1.0 as f32))
                        .collect(),
                };
                self.custom_voxel_map
                    .get_mut(&voxel_type)
                    .unwrap()
                    .push(new_voxel);
                // You may want to push to the existing list here if needed
            }
            None => {
                self.custom_voxel_map.insert(voxel_type, vec![]);
            }
        }
    }

    pub fn get_object(&self, current_object: T) -> Option<Object> {
        self.voxels_map.get(&current_object).cloned()
    }

    pub fn get_object_mut(&mut self, current_object: T) -> Option<&mut Object> {
        self.voxels_map.get_mut(&current_object)
    }

    // pub fn explode_object(&mut self, animation_handler: &mut AnimationHandler, amplify: f32) {
    //     let current_voxel = self.current_voxel.as_mut().unwrap().clone();
    //     self.transition_to_object_base(
    //         current_voxel,
    //         animation_handler,
    //         amplify,
    //         false,
    //         false,
    //         false,
    //     );
    // }
    // pub fn transition_to_custom_object(
    //     &mut self,
    //     object: T,
    //     index: usize,
    //     animation_handler: &mut AnimationHandler,
    //     use_object_color: bool,
    //     is_instant: bool,
    // ) {
    //     if let Some(obj_list) = self.custom_voxel_map.get(&object) {
    //         let obj = obj_list.get(index).cloned();
    //         if let Some(obj) = obj {
    //             self.transition_to_custom_object_base(
    //                 &obj,
    //                 animation_handler,
    //                 1.0,
    //                 true,
    //                 use_object_color,
    //                 is_instant,
    //             );
    //         }
    //     }
    // }
    pub fn transition_to_object(
        &mut self,
        object: T,
        animation_handler: &mut AnimationHandler,
        use_object_color: bool,
        amplify: f32,
    ) {
        self.transition_to_object_base(
            object.clone(),
            animation_handler,
            amplify,
            use_object_color,
        );
    }

    // fn transition_to_custom_object_base(
    //     &mut self,
    //     object: &Object,
    //     animation_handler: &mut AnimationHandler,
    //     amplify: f32,
    //     is_onetime: bool,
    //     use_object_color: bool,
    //     is_instant: bool,
    // ) {
    //     let mut current_cubes = self.current_cubes.clone();
    //     let mut new_current_cubes: Vec<usize> = vec![];
    //
    //     if object.cubes.len() > animation_handler.movement_list.len() {
    //         println!("Object too large to show");
    //         return;
    //     }
    //     let cube_indices: Vec<usize> = (0..object.cubes.len()).collect();
    //     let instance_indices: Vec<usize> = (0..animation_handler.movement_list.len()).collect();
    //     if current_cubes.is_empty() {
    //         current_cubes = instance_indices.clone();
    //     }
    //     let instance_indices_in_order: Vec<usize> =
    //         (0..animation_handler.movement_list.len()).collect();
    //
    //     let cube_indices_len = cube_indices.len();
    //     let current_cubes_len = current_cubes.len();
    //     let mut rng = rng();
    //
    //     if cube_indices_len > current_cubes_len {
    //         let current_cubes_set: HashSet<_> = current_cubes.iter().copied().collect();
    //
    //         let mut cube_indicees_excluded: Vec<usize> = instance_indices
    //             .iter()
    //             .filter(|i| !current_cubes_set.contains(i))
    //             .copied()
    //             .collect();
    //
    //         cube_indicees_excluded.shuffle(&mut rng);
    //         for n in 0..(cube_indices_len - current_cubes_len) {
    //             let elem = cube_indicees_excluded.get(n).unwrap();
    //             current_cubes.push(*elem);
    //         }
    //         // println!("Bigger!")
    //     } else {
    //         // println!("Smaller or the same")
    //     }
    //
    //     // current_cubes.shuffle(&mut rng);
    //     current_cubes.shuffle(&mut rng);
    //     let cubes_indices: Vec<usize> = (0..object.cubes.len()).collect();
    //     for &i in &cubes_indices {
    //         let cube = object.cubes.get(i).unwrap();
    //         let instance_index = current_cubes.pop().unwrap();
    //         let animation = animation_handler.movement_list.get(instance_index).unwrap();
    //         new_current_cubes.push(instance_index);
    //         let movement_vector = cube - animation.grid_pos;
    //         let animation = AnimationType::Step(AnimationStep::new(
    //             movement_vector * amplify,
    //             0.75,
    //             false,
    //             false,
    //             is_onetime,
    //             is_instant,
    //             AnimationTransition::EaseInEaseOut(EaseInEaseOut),
    //             15,
    //         ));
    //         if use_object_color {
    //             let &color = object.color.get(i).unwrap();
    //
    //             animation_handler.set_manual_animation_color(instance_index, color);
    //         } else {
    //             animation_handler.set_animated_color(instance_index);
    //         }
    //         animation_handler.set_animation(instance_index, animation);
    //         // animation_handler.reset_animation_time(index);
    //         animation_handler.set_animation_state(instance_index, true);
    //     }
    //     let instance_indices_remaining: HashSet<usize> =
    //         new_current_cubes.iter().cloned().collect();
    //
    //     // Filter instance_indices to exclude anything in cubes_set
    //     let remaining_indices: Vec<usize> = instance_indices_in_order
    //         .clone()
    //         .into_iter()
    //         .filter(|i| !instance_indices_remaining.contains(i))
    //         .collect();
    //
    //     let mut circle = fibonacci_sphere(remaining_indices.clone().len(), 750.0);
    //
    //     for i in remaining_indices {
    //         let animation = animation_handler.movement_list.get(i).unwrap();
    //
    //         let point = circle.pop().unwrap();
    //         let mut movement_vector = Vector3::new(0.0, 0.0, 0.0);
    //         if animation.grid_pos.distance(Vector3 {
    //             x: 0.0,
    //             y: 0.0,
    //             z: 0.0,
    //         }) <= 500.0
    //         {
    //             movement_vector = Vector3::new(point.x, point.y, point.z);
    //             movement_vector = movement_vector - animation.grid_pos;
    //         }
    //         let animation = AnimationType::Step(AnimationStep::new(
    //             movement_vector,
    //             0.5,
    //             false,
    //             false,
    //             is_onetime,
    //             is_instant,
    //             AnimationTransition::EaseInEaseOut(EaseInEaseOut),
    //             15,
    //         ));
    //
    //         animation_handler.set_animated_color(i);
    //         animation_handler.set_animation(i, animation);
    //         // animation_handler.reset_animation_time(index);
    //         animation_handler.set_animation_state(i, true);
    //     }
    //     self.current_cubes = new_current_cubes;
    // }

    // pub fn transition_to_ba_object(
    //     &mut self,
    //     object: &Vec<Vector3<f32>>,
    //     animation_handler: &mut AnimationHandler,
    //     amplify: f32,
    //     is_onetime: bool,
    //     use_object_color: bool,
    //     is_instant: bool,
    // ) {
    //     let mut current_cubes = self.current_cubes.clone();
    //     let mut new_current_cubes: Vec<usize> = vec![];
    //
    //     if object.len() > animation_handler.movement_list.len() {
    //         println!("Object too large to show");
    //         return;
    //     }
    //     let cube_indices: Vec<usize> = (0..object.len()).collect();
    //     let instance_indices: Vec<usize> = (0..animation_handler.movement_list.len()).collect();
    //     if current_cubes.is_empty() {
    //         current_cubes = instance_indices.clone();
    //     }
    //     let instance_indices_in_order: Vec<usize> =
    //         (0..animation_handler.movement_list.len()).collect();
    //
    //     let cube_indices_len = cube_indices.len();
    //     let current_cubes_len = current_cubes.len();
    //     let mut rng = rng();
    //
    //     if cube_indices_len > current_cubes_len {
    //         let current_cubes_set: HashSet<_> = current_cubes.iter().copied().collect();
    //
    //         let mut cube_indicees_excluded: Vec<usize> = instance_indices
    //             .iter()
    //             .filter(|i| !current_cubes_set.contains(i))
    //             .copied()
    //             .collect();
    //
    //         cube_indicees_excluded.shuffle(&mut rng);
    //         for n in 0..(cube_indices_len - current_cubes_len) {
    //             let elem = cube_indicees_excluded.get(n).unwrap();
    //             current_cubes.push(*elem);
    //         }
    //         // println!("Bigger!")
    //     } else {
    //         // println!("Smaller or the same")
    //     }
    //
    //     // current_cubes.shuffle(&mut rng);
    //     current_cubes.shuffle(&mut rng);
    //     let cubes_indices: Vec<usize> = (0..object.len()).collect();
    //     for &i in &cubes_indices {
    //         let cube = object.get(i).unwrap();
    //         let instance_index = current_cubes.pop().unwrap();
    //         let animation = animation_handler.movement_list.get(instance_index).unwrap();
    //         new_current_cubes.push(instance_index);
    //         let movement_vector = cube - animation.grid_pos;
    //         let animation = AnimationType::Step(AnimationStep::new(
    //             movement_vector * amplify,
    //             0.75,
    //             false,
    //             false,
    //             is_onetime,
    //             is_instant,
    //             AnimationTransition::EaseInEaseOut(EaseInEaseOut),
    //             15,
    //         ));
    //         if use_object_color {
    //             let color = Vector3::new(0.852, 0.067, 0.308);
    //
    //             animation_handler.set_manual_animation_color(instance_index, color);
    //         } else {
    //             animation_handler.set_animated_color(instance_index);
    //         }
    //         animation_handler.set_animation(instance_index, animation);
    //         // animation_handler.reset_animation_time(index);
    //         animation_handler.set_animation_state(instance_index, true);
    //     }
    //     let instance_indices_remaining: HashSet<usize> =
    //         new_current_cubes.iter().cloned().collect();
    //
    //     // Filter instance_indices to exclude anything in cubes_set
    //     let remaining_indices: Vec<usize> = instance_indices_in_order
    //         .clone()
    //         .into_iter()
    //         .filter(|i| !instance_indices_remaining.contains(i))
    //         .collect();
    //
    //     let mut circle = fibonacci_sphere(remaining_indices.clone().len(), 750.0);
    //
    //     for i in remaining_indices {
    //         let animation = animation_handler.movement_list.get(i).unwrap();
    //
    //         let point = circle.pop().unwrap();
    //         let mut movement_vector = Vector3::new(0.0, 0.0, 0.0);
    //         if animation.grid_pos.distance(Vector3 {
    //             x: 0.0,
    //             y: 0.0,
    //             z: 0.0,
    //         }) <= 500.0
    //         {
    //             movement_vector = Vector3::new(point.x, point.y, point.z);
    //             movement_vector = movement_vector - animation.grid_pos;
    //         }
    //         let animation = AnimationType::Step(AnimationStep::new(
    //             movement_vector,
    //             0.5,
    //             false,
    //             false,
    //             is_onetime,
    //             is_instant,
    //             AnimationTransition::EaseInEaseOut(EaseInEaseOut),
    //             15,
    //         ));
    //
    //         animation_handler.set_animated_color(i);
    //         animation_handler.set_animation(i, animation);
    //         // animation_handler.reset_animation_time(index);
    //         animation_handler.set_animation_state(i, true);
    //     }
    //     self.current_cubes = new_current_cubes;
    // }

    fn transition_to_object_base(
        &mut self,
        object: T,
        animation_handler: &mut AnimationHandler,
        amplify: f32,
        use_object_color: bool,
    ) {
        self.current_voxel = Some(object.clone());

        let object = match self.get_object(object) {
            Some(o) => o,
            None => return,
        };

        let instance_count = animation_handler.movement_list.len();
        let cube_count = object.cubes.len();

        if cube_count > instance_count {
            return;
        }

        // 🔥 reuse buffers
        self.temp_indices.clear();
        self.temp_indices.extend(0..instance_count);

        self.temp_flags.clear();
        self.temp_flags.resize(instance_count, false);

        // 🔀 partial shuffle (Fisher-Yates, only what we need)
        let mut rng = rng();
        for i in 0..cube_count {
            let j = rand::Rng::random_range(&mut rng, i..instance_count);
            self.temp_indices.swap(i, j);
        }

        self.current_cubes.clear();
        self.current_cubes.reserve(cube_count);

        // ✅ assign cubes → instances
        for i in 0..cube_count {
            let instance_index = self.temp_indices[i];
            self.temp_flags[instance_index] = true;
            self.current_cubes.push(instance_index);

            let cube = object.cubes[i];
            let anim = &mut animation_handler.movement_list[instance_index];

            let step = AnimationStep {
                from: anim.base_position,
                to: cube * amplify,
                t: 0.0,
                speed: 0.75,
                animation_transition: AnimationTransition::EaseInEaseOut,
                state: StepState::Forward,
            };

            anim.steps.clear(); // optional: overwrite instead of stacking
            anim.steps.push(step);

            if use_object_color {
                anim.color = object.color[i];
            }
        }

        // 🌌 remaining instances → sphere
        let mut sphere_index = 0;
        let sphere = fibonacci_sphere(instance_count - cube_count, 750.0);

        for i in 0..instance_count {
            if self.temp_flags[i] {
                continue;
            }

            let anim = &mut animation_handler.movement_list[i];

            let point = sphere[sphere_index];
            sphere_index += 1;

            let step = AnimationStep {
                from: anim.base_position,
                to: point,
                t: 0.0,
                speed: 0.5,
                animation_transition: AnimationTransition::EaseInEaseOut,
                state: StepState::Forward,
            };

            anim.steps.clear();
            anim.steps.push(step);
        }
    }
}
pub fn instances_list_cube(chunk: Vector3<i32>, chunk_size: Vector3<i32>) -> Vec<Instance> {
    (0..(chunk_size.x * chunk_size.y * chunk_size.z))
        .map(move |n| {
            let x = n % chunk_size.x;
            let z = (n / chunk_size.x) % chunk_size.z;
            let y = n / (chunk_size.x * chunk_size.z);

            let position = cgmath::Vector3 {
                x: x as f32 + (chunk.x * chunk_size.x) as f32,
                y: y as f32 + (chunk.z * chunk_size.y) as f32,
                z: z as f32 + (chunk.y * chunk_size.z) as f32,
            };

            let rotation = if position.is_zero() {
                // this is needed so an object at (0, 0, 0) won't get scaled to zero
                // as Quaternions can effect scale if they're not created correctly
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
            };
            let default_color = cgmath::Vector3::new(1.0, 0.0, 0.0);
            let default_size = cgmath::Vector3::new(1.0, 1.0, 1.0);
            let default_bounding = default_size + position;

            Instance {
                index: n as u32,
                position,
                rotation,
                scale: 1.0,
                should_render: true,
                color: default_color,
                size: default_size,
                bounding: default_bounding,
            }
        })
        .collect::<Vec<_>>()
}
fn fibonacci_sphere(points: usize, scalar: f32) -> Vec<Vector3<f32>> {
    let mut vecs: Vec<Vector3<f32>> = vec![];
    let phi = PI * (f32::sqrt(5.0) - 1.0);

    for n in 0..points {
        let y = 1.0 - (n as f32 / (points as f32 - 1.0)) * 2.0;
        let radius = f32::sqrt(1.0 - y * y);
        let theta = phi * n as f32;

        let x = f32::cos(theta) * radius;
        let z = f32::sin(theta) * radius;

        vecs.push(Vector3 { x: x, y: y, z: z } * scalar);
    }

    vecs
}

fn get_srgb(color: u8) -> f32 {
    ((color as f32 / 255 as f32 + 0.055) / 1.055).powf(2.4)
}
