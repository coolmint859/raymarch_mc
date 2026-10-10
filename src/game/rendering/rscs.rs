use std::println;

use crate::{game::{EnvironmentUniform, VoxelPalette, VoxelWorld}, graphics::{Buffer, BufferId, Graphics, Sampler, SamplerId, TexDimensions, Texture, TextureId}, utils::{BufferInit, StorageBuffer, UniformBuffer}};

pub struct ResourceIds {
    pub taa: TaaIds,
    pub world: VoxelWorldIds,
    pub g_buffer: GBufferIds,
    pub material: MaterialIds,
}

impl ResourceIds {
    /// Initialize the ids
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld) -> Self {
        Self {
            taa: TaaIds::init(graphics),
            world: VoxelWorldIds::init(graphics, world),
            g_buffer: GBufferIds::init(graphics),
            material: MaterialIds::init(graphics)
        }
    }

    /// Resize the textures when the window resizes
    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        self.taa.on_resize(graphics);
        self.g_buffer.on_resize(graphics);
    }
}

pub struct TaaIds {
    pub rm_tex: TextureId,
    pub tex1: TextureId,
    pub tex2: TextureId,
}

impl TaaIds {
    pub fn init(graphics: &mut Graphics) -> Self {
        let (cw, ch) = graphics.canvas.dimensions();

        let rm_tex_id = graphics.context.request_texture( 
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("Raymarch Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .writable()
        );

        let taa_tex1_id = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("TAA Texture 1")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
        );

        let taa_tex2_id  = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("TAA Texture 2")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
        );

        Self {
            rm_tex: rm_tex_id,
            tex1: taa_tex1_id,
            tex2: taa_tex2_id
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let (cw, ch) = graphics.canvas.dimensions();

        graphics.context.remove_texture(&self.rm_tex);
        graphics.context.remove_texture(&self.tex1);
        graphics.context.remove_texture(&self.tex2);

        self.rm_tex = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("Raymarch Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .writable()
        );

        self.tex1 = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("TAA Texture1")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
        );

        self.tex2 = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw, ch))
                .with_label("TAA Texture2")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
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
            Buffer::as_storage()
                .with_label("Grid Buffer")
                .with_byte_data(&region_bytes.grids)
                .writable()
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
        let (cw, ch) = graphics.canvas.dimensions();
        let cw_half = cw / 2;
        let ch_half = ch / 2;

        let pos_tex_id = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Position Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .readable()
        );

        let norm_tex_id = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::Rgba8Unorm)
                .storage_bindable()
                .readable()
        );

        let depth_tex_id = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable()
        );

        let mat_tex_id = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable()
        );

        Self {
            pos: pos_tex_id,
            norm: norm_tex_id,
            depth: depth_tex_id,
            mat: mat_tex_id
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let (cw, ch) = graphics.canvas.dimensions();
        let cw_half = cw / 2;
        let ch_half = ch / 2;

        graphics.context.remove_texture(&self.pos);
        graphics.context.remove_texture(&self.norm);
        graphics.context.remove_texture(&self.depth);
        graphics.context.remove_texture(&self.mat);

        self.pos = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Position Texture")
                .with_format(wgpu::TextureFormat::Rgba16Float)
                .storage_bindable()
                .readable()
        );

        self.norm = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::Rgba8Unorm)
                .storage_bindable()
                .readable()
        );

        self.depth = graphics.context.request_texture(Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable()
        );

        self.mat = graphics.context.request_texture(
            Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
                .with_label("Normals Texture")
                .with_format(wgpu::TextureFormat::R32Float)
                .storage_bindable()
                .readable()
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
            Texture::on_disk("./assets/grass_block_side_overlay.png")
                .with_label("Grass Side Alpha Mask")
                .writable()
        );

        let atlas_id = graphics.context.request_texture(
            Texture::on_disk("./assets/textures.png")
                .with_label("Block Atlas Texture")
                .writable()
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