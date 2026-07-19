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
        let colors = vec![
            Vec4::new(0.0, 0.0, 0.0, 0.0),      // 0: Air
            Vec4::new(0.5, 0.5, 0.5, 1.0),      // 1: Stone
            Vec4::new(0.0, 0.5, 0.0, 1.0),      // 2: Grass
            Vec4::new(0.0, 0.0, 0.5, 1.0),      // 3: Water
            Vec4::new(0.39, 0.254, 0.09, 1.0),   // 4: Tree Trunk
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