use core::num;

use crate::game::{REGION_SIZE, REGION_VOLUME, RegionLocation, Voxel};

pub struct RegionData {
    /// Raw voxel data
    pub voxels: Box<[Voxel; REGION_VOLUME]>,
    /// 16^3 grid level
    pub grid16: u8,
}

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn gen_region(&self, loc: RegionLocation) -> RegionData {
        let voxels = self.gen_voxels(loc);
        let grid16 = self.gen_grid16(&voxels);

        return RegionData {
            voxels,
            grid16,
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

        let num_trees = 3;
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

    fn gen_grid16(&self, voxels: &Box<[Voxel; REGION_VOLUME]>) -> u8 {
        let mut region_byte: u8 = 0;
        
        // Each 32x32 region contains 8 sub-bricks of 16x16x16
        let mut sub_idx = 0;
        for sub_z in 0..2 {
            for sub_y in 0..2 {
                for sub_x in 0..2 {
                    let mut has_solid = false;
                    
                    // Check voxels inside this specific 16x16x16 sub-brick
                    for z in 0..16 {
                        for y in 0..16 {
                            for x in 0..16 {
                                let vx = (sub_x * 16) + x;
                                let vy = (sub_y * 16) + y;
                                let vz = (sub_z * 16) + z;
                                
                                let voxel_idx = vx + (vy * 32) + (vz * 32 * 32);
                                // Assuming region_voxels slices per region
                                if voxels[voxel_idx].0 > 0 {
                                    has_solid = true;
                                    break;
                                }
                            }
                            if has_solid { break; }
                        }
                        if has_solid { break; }
                    }
                    
                    // If the sub-brick has solid data, set the corresponding bit
                    if has_solid {
                        region_byte |= 1 << sub_idx;
                    }
                    sub_idx += 1;
                }
            }
        }

        region_byte
    }
}