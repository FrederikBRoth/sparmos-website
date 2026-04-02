use sparmos_engine::{cgmath::Vector3, winit::dpi::PhysicalSize};

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
    pub length: i32,
    pub raw: Vec<u8>,
}

impl EasterEgg {
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

    pub fn reset(&mut self) {
        self.toggle = false;
        self.index = 0;
    }
}
