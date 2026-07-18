use crate::game::{Environment, EnvironmentUniform, REGION_VOLUME, Region, RegionLocation, WorldGenerator};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RegionLocUniform {
    coords: [[i32; 3]; 9]
}

pub struct VoxelWorld {
    env: Environment,
    world_gen: WorldGenerator,
    is_paused: bool,
    regions: Vec<Region>,
}

impl VoxelWorld {
    pub fn new()-> Self {
        let world_gen = WorldGenerator;
        let mut regions = Vec::new();

        // for x in -1..1 {
        //     for z in -1..1 {
        //         let location = RegionLocation { x: x, y: 0, z: z };
        //         let region_data = world_gen.gen_region(location);

        //         regions.push(Region::new(region_data, location));
        //     }
        // }

        let location = RegionLocation { x: 0, y: 0, z: 0 };
        let region_data = world_gen.gen_region(location);

        regions.push(Region::new(region_data, location));

        Self {
            env: Environment::new(),
            world_gen,
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

    pub fn voxel_data(&self) -> Vec<u8> {
        let bytes_per_region = REGION_VOLUME * 4;
        let total_size = bytes_per_region * self.regions.len();

        let mut total_bytes = Vec::with_capacity(total_size);

        for region in &self.regions {
            total_bytes.extend_from_slice(&region.voxel_bytes());
        }

        total_bytes
    }

    // pub fn voxel_loc_data(&self) -> Vec<u8> {
        
    // }
}