use crate::game::{REGION_SIZE, REGION_VOLUME, RegionLocation, Voxel};

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn gen_region(&self, _loc: RegionLocation) -> Box<[Voxel; REGION_VOLUME]> {
        let mut voxels = [Voxel(0); REGION_VOLUME];

        for z in 0..REGION_SIZE {
            for x in 0..REGION_SIZE {
                for y in 0..REGION_SIZE {
                    let idx = x + (y * REGION_SIZE) + (z * REGION_SIZE * REGION_SIZE);

                    voxels[idx] = if y < 16 {
                        Voxel(1)
                    } else if y == 16 && x < 12 && z < 12 {
                        Voxel(3)
                    } else if y == 16 {
                        Voxel(2)
                    } else {
                        Voxel(0)
                    }
                }
            }
        }

        Box::new(voxels)
    }
}