use bytemuck::NoUninit;

/// The number of blocks/voxels that make up a region's length
pub const REGION_SIZE: usize = 32;
/// The total number of voxels in a region
pub const REGION_VOLUME: usize = REGION_SIZE * REGION_SIZE * REGION_SIZE;
/// The max number of regions on any given side of the player that can be rendered by the game
// 1.e. A distance of 5 regions: 11x11x11 = 1331 regions around the player, with a width of 32 voxels, this is 42592 voxels total
pub const RENDER_DISTANCE: usize = 5;

/// A single block in the game world. Each voxel only holds an index into a global palette.
/// 
/// The index is 1 byte in size, allowing for the pallete to hold 256 different block types
#[derive(Clone, Copy, Debug, PartialEq, Eq, NoUninit)]
#[repr(transparent)]
pub struct Voxel(pub u32);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RegionLocation {
    pub x: i32, pub y: i32, pub z: i32
}

pub struct Region {
    pub voxels: Box<[Voxel; REGION_VOLUME]>,
    pub location: RegionLocation,
}

impl Region {
    pub fn new(voxels: Box<[Voxel; REGION_VOLUME]>, location: RegionLocation) -> Self {
        Self { 
            voxels,
            location
        }
    }

    pub fn update(&mut self, _dt: f32) {}

    /// Serialize the region's voxel data into a vec of bytes
    pub fn voxel_bytes(&self) -> Vec<u8> {
        bytemuck::cast_slice(self.voxels.as_ref()).to_vec()
    }

    pub fn get_location(&self) -> Vec<u8> {
        bytemuck::bytes_of(&self.location).to_vec()
    }
}