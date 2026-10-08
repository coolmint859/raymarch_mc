use crate::game::{REGION_SIZE, REGION_VOLUME, RegionLocation, Voxel};

pub struct RegionData {
    /// Raw voxel data
    pub voxels: Box<[Voxel; REGION_VOLUME]>,
    /// 16^3 grid level
    pub grids: RegionGrids,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RegionGrids {
    pub grid4: [u32; 16],       // 4x4x4 voxel bricks, 512 per region
    pub grid8: [u32; 2],        // 8x8x8 voxel bricks, 64 per region
    pub grid16: u32             // 16x16x16 bricks, 8 per region 
}

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn gen_region(&self, loc: RegionLocation) -> RegionData {
        let voxels = self.gen_voxels(loc);
        let grids = self.gen_grids(&voxels);

        return RegionData {
            voxels,
            grids,
        }
    }

    fn gen_voxels(&self, _loc: RegionLocation) -> Box<[Voxel; REGION_VOLUME]> {
        let mut voxels = [Voxel(0); REGION_VOLUME];

        for z in 0..REGION_SIZE {
            for x in 0..REGION_SIZE {
                for y in 0..REGION_SIZE {
                    let idx = x + (y * REGION_SIZE) + (z * REGION_SIZE * REGION_SIZE);

                    voxels[idx] = if y == 14 && x <= 12 && z <= 12 && x >= 2 && z >= 2 {
                        Voxel(4)
                    } else if y == 15 && (x > 12 || z > 12 || x < 2 || z < 2) {
                        Voxel(2)
                    } else if y < 15 {
                        Voxel(1)
                    } else {
                        Voxel(0)
                    };
                }
            }
        }

        let num_trees = 6;
        for _ in 0..num_trees {
            let tx = (rand::random::<f32>() * REGION_SIZE as f32).floor() as usize;
            let tz = (rand::random::<f32>() * REGION_SIZE as f32).floor() as usize;
            let ty = ((rand::random::<f32>() * 8.0).floor() + 4.0) as usize;
            let my = ty + 16;

            for y in 16..my {
                let idx = tx + (y * REGION_SIZE) + (tz * REGION_SIZE * REGION_SIZE);

                voxels[idx] = Voxel(3);
            }
        }

        Box::new(voxels)
    }

    fn gen_grids(&self, voxels: &Box<[Voxel; REGION_VOLUME]>) -> RegionGrids {
        let mut grids = RegionGrids {
            grid4: [0; 16],
            grid8: [0; 2],
            grid16: 0,
        };
        
        for z in 0..32 {
            for x in 0..32 {
                for y in 0..32 {
                    let vox_idx = x + (y * REGION_SIZE) + (z * REGION_SIZE * REGION_SIZE);
                    if voxels[vox_idx] == Voxel(0) { continue; }

                    // Grid16
                    let b16_x = x >> 4;
                    let b16_y = y >> 4;
                    let b16_z = z >> 4;
                    let idx16 = b16_x + (b16_y * 2) + (b16_z * 4);
                    grids.grid16 |= 1 << idx16;

                    // Grid8
                    let b8_x = x >> 3;
                    let b8_y = y >> 3;
                    let b8_z = z >> 3;
                    let idx8 = b8_x + (b8_y * 4) + (b8_z * 16);
                    grids.grid8[idx8 / 32] |= 1 << (idx8 % 32);

                    // Grid4
                    let b4_x = x >> 2;
                    let b4_y = y >> 2;
                    let b4_z = z >> 2;
                    let idx4 = b4_x + (b4_y * 8) + (b4_z * 64);
                    grids.grid4[idx4 / 32] |= 1 << (idx4 % 32);
                }
            }
        }

        grids
    }
}

