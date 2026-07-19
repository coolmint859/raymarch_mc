use crate::game::{REGION_SIZE, REGION_VOLUME, RegionLocation, Voxel};

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn gen_region(&self, _loc: RegionLocation) -> Box<[Voxel; REGION_VOLUME]> {
        let mut voxels = [Voxel(0); REGION_VOLUME];

        for z in 0..REGION_SIZE {
            for x in 0..REGION_SIZE {
                for y in 0..REGION_SIZE {
                    let idx = x + (y * REGION_SIZE) + (z * REGION_SIZE * REGION_SIZE);

                    voxels[idx] = if y == 15 && x <= 12 && z <= 12 && x >= 2 && z >= 2 {
                        Voxel(3)
                    } else if y == 16 && (x > 12 || z > 12 || x < 2 || z < 2) {
                        Voxel(2)
                    } else if y < 16 {
                        Voxel(1)
                    } else {
                        Voxel(0)
                    };
                }
            }
        }

        for _ in 0..15 {
            let tx = (rand::random::<f32>() * REGION_SIZE as f32).floor() as usize;
            let tz = (rand::random::<f32>() * REGION_SIZE as f32).floor() as usize;
            let ty = ((rand::random::<f32>() * 8.0).floor() + 4.0) as usize;
            let my = ty + 17;

            for y in 17..my {
                let idx = tx + (y * REGION_SIZE) + (tz * REGION_SIZE * REGION_SIZE);

                voxels[idx] = Voxel(4);
            }
        }

        Box::new(voxels)
    }
}