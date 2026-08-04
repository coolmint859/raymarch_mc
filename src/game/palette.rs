use glam::{Vec3, Vec4};

pub const DAY_ZENITH: Vec3 =    glam::vec3(0.2,  0.4,  0.8 );
pub const DUSK_ZENITH: Vec3 =   glam::vec3(0.05, 0.08, 0.15);
pub const NIGHT_ZENITH: Vec3 =  glam::vec3(0.01,  0.01,  0.01);

pub const DAY_HORIZON: Vec3 =   glam::vec3(0.6, 0.75, 0.95);
pub const DUSK_HORIZON: Vec3 =  glam::vec3(0.4, 0.2, 0.2);
pub const NIGHT_HORIZON: Vec3 = glam::vec3(0.0, 0.0, 0.04);

pub const DAY_SUN: Vec3 =       glam::vec3(1.0, 0.95, 0.85);
pub const DUSK_SUN: Vec3 =      glam::vec3(1.0, 0.5, 0.2);
pub const NIGHT_SUN: Vec3 =     glam::vec3(0.0, 0.0, 0.0);

pub struct VoxelPalette {
    pub colors: Vec<u8>
}

impl VoxelPalette {
    pub fn create() -> Self {
        let pad = 4.0;
        let len = 32.0;
        let pdl = pad + len;
        let size = 1024.0;

        // let x_min = (pdl * row) / size;
        // let y_min = (pdl * col) / size;
        // let x_max = (len + pdl * row) / size;
        // let y_max = (len + pdl * col) / size;

        let colors = vec![
            Vec4::new(0.0, 0.0, 0.0, 0.0),                                                  // 0: Air
            Vec4::new(0.0, (pdl * 0.0) / size, len / size, (len + pdl * 0.0) / size),       // 1: Stone
            Vec4::new(0.0, (pdl * 1.0) / size, len / size, (len + pdl * 1.0) / size),       // 2: Grass
            Vec4::new(0.0, (pdl * 2.0) / size, len / size, (len + pdl * 2.0) / size),       // 3: Tree Trunk
            Vec4::new(0.0, (pdl * 3.0) / size, len / size, (len + pdl * 3.0) / size),       // 4: Water
        ];

        let mut bytes: Vec<u8> = Vec::new();
        for color in &colors {
            bytes.extend_from_slice(bytemuck::bytes_of(color));
        }

        Self {
            colors: bytes
        }
    }
}