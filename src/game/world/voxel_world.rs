use std::println;

use crate::game::{Environment, EnvironmentUniform, REGION_VOLUME, Region, RegionLocation, WorldGenerator};

pub struct RegionBytes {
    pub voxels: Vec<u8>,
    pub grids: Vec<u8>,
}

pub struct VoxelWorld {
    env: Environment,
    // world_gen: WorldGenerator,
    is_paused: bool,
    regions: Vec<Region>,
}

impl VoxelWorld {
    pub fn new()-> Self {
        let world_gen = WorldGenerator;
        let mut regions = Vec::new();

        for x in -4..=4 {
            for z in -4..=4 {
                let location = RegionLocation { x: x, y: 0, z: z, _pad: 0 };
                let region_data = world_gen.gen_region(location);

                // println!("{:?}", location);

                regions.push(Region::new(region_data, location));
            }
        }

        Self {
            env: Environment::new(),
            // world_gen,
            is_paused: false,
            regions
        }
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        println!("Is Paused: {}", self.is_paused);
    }

    pub fn update(&mut self, dt: f32, is_step: bool) {
        if !self.is_paused || is_step {
            self.env.update(dt);
            self.regions.iter_mut().for_each(|region|region.update(dt));
        }
    }

    pub fn env_uniform(&self) -> EnvironmentUniform {
        self.env.to_uniform()
    }

    pub fn region_bytes(&self) -> RegionBytes {
        let bytes_per_region = REGION_VOLUME * 4;
        let vtotal_size = bytes_per_region * self.regions.len();

        let mut vox_total_bytes = Vec::with_capacity(vtotal_size);
        let mut grids_total_bytes = Vec::with_capacity(self.regions.len());

        for region in &self.regions {
            vox_total_bytes.extend_from_slice(&region.voxel_bytes());
            grids_total_bytes.extend_from_slice(&region.grids());
        }

        RegionBytes {
            voxels: vox_total_bytes,
            grids: grids_total_bytes,
        }
    }
}