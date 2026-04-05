use sparmos_engine::{
    cgmath::Vector3,
    entity::systems::camera::{Camera, CameraSystem},
    winit::dpi::PhysicalSize,
};

pub enum EasterEggType {
    BadApple,
    Umaru,
}

#[derive(Clone, Default)]
pub struct EasterEgg {
    pub toggle: bool,
    pub data: Vec<Vec<u8>>,
    pub index: i32,
    pub dimensions: PhysicalSize<i32>,
    pub fps: f32,
    pub elapsed: f32,
    pub raw: Vec<u8>,
    pub original_camera_speed: f32,
}

impl EasterEgg {
    pub fn new(
        dimensions: PhysicalSize<i32>,
        fps: f32,
        raw: Vec<u8>,
        original_camera_speed: f32,
    ) -> Self {
        Self {
            dimensions,
            fps,
            raw,
            original_camera_speed,
            ..Default::default()
        }
    }
    pub fn get_frame(&self) -> Vec<Vector3<f32>> {
        let chunk_size = (self.dimensions.height * self.dimensions.width) / 8;

        let start = self.index * chunk_size;
        // if start >= chunk_size * 2157 {
        //     continue;
        // }
        let end = start + chunk_size;

        let chunk = &self.raw[start as usize..end as usize];

        let data: Vec<u8> = chunk
            .iter()
            .flat_map(|byte| (0..8).rev().map(move |bit| (byte >> bit) & 1))
            .rev()
            .collect();
        let voxel_canvas: Vec<_> = data
            .iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                if value != 1 {
                    return None;
                }
                let x = i as i32 % self.dimensions.width;
                let y = i as i32 / self.dimensions.width;
                if x == 0
                    || x == self.dimensions.width - 1
                    || y == 0
                    || y == self.dimensions.height - 1
                {
                    None
                } else {
                    Some(Vector3::new(x as f32, y as f32, 0.0))
                }
            })
            .collect();
        voxel_canvas
    }

    pub fn init_camera(&self, camera: &mut Camera) {
        let (bad_apple_eye, bad_apple_target) = ((162.0, 122.0, -560.0), (162.0, 122.0, 0.0));
        camera.eye = bad_apple_eye.into();
        camera.target = bad_apple_target.into();
    }

    pub fn reset_camera(&mut self, camera_system: &mut CameraSystem) {
        camera_system.speed = self.original_camera_speed;
        self.index = 0;
    }
    pub fn update_camera(&mut self, camera_system: &mut CameraSystem, camera: &mut Camera) {
        let x = camera.eye.z.abs(); // make x absolute
        let max = 3.0;
        let min = 0.5;
        let k = 0.01; // controls steepness
        let midpoint = 275.0; // controls where curve bends (half of 550)

        let value = min + (max - min) / (1.0 + (x / midpoint).powf(k * midpoint));

        camera_system.speed = value * 0.25;
    }
}
