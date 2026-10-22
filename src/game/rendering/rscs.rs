use crate::{game::{EnvironmentUniform, VoxelPalette, VoxelWorld}, graphics::{Buffer, BufferId, BufferSource, Computed, Graphics, OnDisk, Sampler, SamplerId, StructuredUpdate, TexDimensions, Texture, TextureId}, utils::{BufferInit, StorageBuffer, UniformBuffer}};

pub struct ResourceIds {
    pub taa: TaaIds,
    pub world: VoxelWorldIds,
    pub g_buffer: GBufferIds,
    pub material: MaterialIds,
}

impl ResourceIds {
    /// Initialize the resource ids
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld, cam_vp: glam::Mat4) -> Self {
        Self {
            taa: TaaIds::init(graphics, cam_vp),
            world: VoxelWorldIds::init(graphics, world),
            g_buffer: GBufferIds::init(graphics),
            material: MaterialIds::init(graphics)
        }
    }

    /// Update the camera motion (view projection) matrices for the taa pass. 
    /// 
    /// `cam_vp` replaces `curr_vp` in the uniform, and `curr_vp` replaces `prev_vp`. This effectively makes it a small ring buffer.
    pub fn update_cam_motion(&mut self, graphics: &mut Graphics, cam_vp: glam::Mat4) {
        self.taa.update_cam_motion(graphics, cam_vp);
    }

    /// Resize the screen-dependent textures
    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        self.taa.on_resize(graphics);
        self.g_buffer.on_resize(graphics);
    }
}

/// Uniform POD struct for the taa pass camera vp ring buffer
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CamTAAViewProjUniform {
    prev_vp: glam::Mat4,
    curr_vp_inv: glam::Mat4,
}

pub struct TaaIds {
    pub rm_tex: TextureId,
    pub tex1: TextureId,
    pub tex2: TextureId,
    pub cam_vp_mats: BufferId,
    pub samp: SamplerId,

    cam_vp_uniform: CamTAAViewProjUniform,
}

impl TaaIds {
    pub fn init(graphics: &mut Graphics, vp_mat: glam::Mat4) -> Self {
        let dims = graphics.canvas.dimensions();

        let rm_tex_id = graphics.context.request_texture( 
            Texture::new()
                .with_label("Raymarch Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );

        let taa_tex1_id = graphics.context.request_texture(
            Texture::new()
                .with_label("TAA Texture 1")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );

        let taa_tex2_id  = graphics.context.request_texture(
            Texture::new()
                .with_label("TAA Texture 2")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );

        let cam_vp_uniform = CamTAAViewProjUniform {
            prev_vp: vp_mat,
            curr_vp_inv: vp_mat.inverse()
        };

        let cam_vp_mats_id = graphics.context.request_buffer(
            Buffer::as_uniform().writable(), 
            BufferSource::from_pod(cam_vp_uniform)
        );

        let samp_id = graphics.context.request_sampler(Sampler::linear());

        Self {
            rm_tex: rm_tex_id,
            tex1: taa_tex1_id,
            tex2: taa_tex2_id,
            cam_vp_mats: cam_vp_mats_id,
            samp: samp_id,
            cam_vp_uniform
        }
    }

    /// Update the camera motion (view projection) matrices. 
    /// 
    /// `camp_vp` replaces `curr_vp` in the uniform, and `curr_vp` replaces `prev_vp`. This effectively makes it a small ring buffer.
    pub fn update_cam_motion(&mut self, graphics: &mut Graphics, cam_vp: glam::Mat4) {
        self.cam_vp_uniform.prev_vp = self.cam_vp_uniform.curr_vp_inv.inverse();
        self.cam_vp_uniform.curr_vp_inv = cam_vp.inverse();

        let _ = graphics.context.update_buffer(&self.cam_vp_mats, StructuredUpdate {
            data: &self.cam_vp_uniform,
            offset: 0
        });
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let dims = graphics.canvas.dimensions();

        graphics.context.remove_texture(&self.rm_tex);
        graphics.context.remove_texture(&self.tex1);
        graphics.context.remove_texture(&self.tex2);

        self.rm_tex = graphics.context.request_texture(
            Texture::new()
                .with_label("Raymarch Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .writable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );

        self.tex1 = graphics.context.request_texture(
            Texture::new()
                .with_label("TAA Texture1")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );

        self.tex2 = graphics.context.request_texture(
            Texture::new()
                .with_label("TAA Texture2")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable(),
            Computed { dim: TexDimensions::size_2d(dims.width, dims.height) }
        );
    }
}

pub struct VoxelWorldIds {
    pub env: UniformBuffer<EnvironmentUniform>,
    pub voxels: StorageBuffer<BufferInit>,
    pub grid: BufferId,
    pub palette: UniformBuffer<VoxelPalette>,
}

impl VoxelWorldIds {
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld) -> Self {
        let env = UniformBuffer::new(world.env_uniform())
            .with_label("Environment Uniform")
            .init(&mut graphics.context);

        let region_bytes = world.region_bytes();
        let voxel_byte_length = region_bytes.voxels.len();
        let voxels = StorageBuffer::new()
            .with_label("Voxel Buffer")
            .with_attribute("voxel_data", region_bytes.voxels)
            .init(&mut graphics.context, voxel_byte_length);

        let grid_id = graphics.context.request_buffer( 
            Buffer::as_storage().with_label("Grid Buffer").writable(),
            BufferSource::from_raw(region_bytes.grids)
        );

        let palette = UniformBuffer::new(VoxelPalette::create())
            .with_label("Palette Buffer")
            .init(&mut graphics.context);
    
        Self {
            env,
            voxels,
            grid: grid_id,
            palette
        }
    }
}

pub struct GBufferIds {
    pub pos: TextureId,
    pub norm: TextureId,
    pub depth: TextureId,
    pub mat: TextureId,
}

impl GBufferIds {
    pub fn init(graphics: &mut Graphics) -> Self {
        // coarse pass runs at half-resolution
        let dims = graphics.canvas.dimensions();
        let cw_half = dims.width / 2;
        let ch_half = dims.height / 2;

        let pos_tex_id = graphics.context.request_texture(
            Texture::new()
                .with_label("Position Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        let norm_tex_id = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::Rgba8Unorm)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        let depth_tex_id = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        let mat_tex_id = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        Self {
            pos: pos_tex_id,
            norm: norm_tex_id,
            depth: depth_tex_id,
            mat: mat_tex_id
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let dims = graphics.canvas.dimensions();
        let cw_half = dims.width / 2;
        let ch_half = dims.height / 2;

        graphics.context.remove_texture(&self.pos);
        graphics.context.remove_texture(&self.norm);
        graphics.context.remove_texture(&self.depth);
        graphics.context.remove_texture(&self.mat);

        self.pos = graphics.context.request_texture(
            Texture::new()
                .with_label("Position Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        self.norm = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::Rgba8Unorm)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        self.depth = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );

        self.mat = graphics.context.request_texture(
            Texture::new()
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable(),
            Computed { dim: TexDimensions::size_2d(cw_half, ch_half) }
        );
    }
}

pub struct MaterialIds {
    pub block_atlas: TextureId,
    pub grass_alpha: TextureId,
    pub sampler: SamplerId,
}

impl MaterialIds {
    pub fn init(graphics: &mut Graphics) -> Self {
        let gsam_id = graphics.context.request_texture(
            Texture::new().with_label("Grass Side Alpha Mask").writable(),
            OnDisk { path: "./assets/grass_block_side_overlay.png" }
        );

        let atlas_id = graphics.context.request_texture(
            Texture::new().with_label("Block Atlas Texture").writable(),
            OnDisk { path: "./assets/textures.png" }
        );

        let samp_id = graphics.context.request_sampler(
            Sampler::nearest()
                .with_label("Atlas Sampler")
        );

        Self {
            grass_alpha: gsam_id,
            block_atlas: atlas_id,
            sampler: samp_id,
        }
    }
}