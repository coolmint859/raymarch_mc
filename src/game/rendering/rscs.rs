use crate::{game::{VoxelPalette, VoxelWorld}, graphics::{Buffer, BufferId, Graphics, Sampler, SamplerId, TexDimensions, Texture, TextureId}};

pub struct ResourceIds {
    pub taa: TaaIds,
    pub world: VoxelWorldIds,
    pub g_buffer: GBufferIds,
    pub material: MaterialIds,
}

impl ResourceIds {
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld) -> Self {
        Self {
            taa: TaaIds::init(graphics),
            world: VoxelWorldIds::init(graphics, world),
            g_buffer: GBufferIds::init(graphics),
            material: MaterialIds::init(graphics)
        }
    }

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

        let rm_tex_id = TextureId("rm_texture");
        let taa_tex1_id = TextureId("taa_tex1_id");
        let taa_tex2_id = TextureId("taa_tex2_id");

        let raymarch_texture = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("Raymarch Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable()
            .writable();
        graphics.context.request_texture(&rm_tex_id, raymarch_texture);

        let taa_texture1 = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture 1")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&taa_tex1_id, taa_texture1);

        let taa_texture2 = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture 2")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&taa_tex2_id, taa_texture2);

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

        let raymarch_texture = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("Raymarch Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable()
            .writable();
        graphics.context.request_texture(&self.rm_tex, raymarch_texture);

        let taa_texture1 = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture1")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.tex1, taa_texture1);

        let taa_texture2 = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture2")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.tex2, taa_texture2);
    }
}

pub struct VoxelWorldIds {
    pub env: BufferId,
    pub voxels: BufferId,
    pub grid: BufferId,
    pub palette: BufferId,
}

impl VoxelWorldIds {
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld) -> Self {
        let env_buffer = Buffer::as_uniform()
            .with_struct_data(world.env_uniform())
            .with_label("Environment Buffer")
            .writable();
        let env_id = graphics.context.request_buffer(env_buffer);

        let region_bytes = world.region_bytes();
        let voxel_buffer = Buffer::as_storage()
            .with_label("Voxel Buffer")
            .with_byte_data(&region_bytes.voxels)
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        let vox_id = graphics.context.request_buffer(voxel_buffer);

        let grid_buffer = Buffer::as_storage()
            .with_label("Grid Buffer")
            .with_byte_data(&region_bytes.grids)
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        let grid_id = graphics.context.request_buffer( grid_buffer);

        let palette_data = VoxelPalette::create().colors;
        let palette_buffer = Buffer::as_uniform()
            .with_byte_data(&palette_data)
            .with_label("Palette Buffer")
            .writable();
        let palette_id = graphics.context.request_buffer( palette_buffer);
    
        Self {
            env: env_id,
            voxels: vox_id,
            grid: grid_id,
            palette: palette_id
        }
    }
}

pub struct GBufferIds {
    pub pos: TextureId,
    pub norm: TextureId,
    pub depth: TextureId,
    pub mat_id: TextureId,
}

impl GBufferIds {
    pub fn init(graphics: &mut Graphics) -> Self {
        // coarse pass runs at half-resolution
        let (cw, ch) = graphics.canvas.dimensions();
        let cw_half = cw / 2;
        let ch_half = ch / 2;

        let pos_id = TextureId("coarse_positions");
        let norm_id = TextureId("coarse_normals");
        let depth_id = TextureId("coarse_depth");
        let mat_id = TextureId("coarse_material_ids");

        let positions_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Position Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&pos_id, positions_texture);

        let normals_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&norm_id, normals_texture);

        let depth_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&depth_id, depth_texture);

        let material_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&mat_id, material_texture);

        Self {
            pos: pos_id,
            norm: norm_id,
            depth: depth_id,
            mat_id: mat_id
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let (cw, ch) = graphics.canvas.dimensions();
        let cw_half = cw / 2;
        let ch_half = ch / 2;

        graphics.context.remove_texture(&self.pos);
        graphics.context.remove_texture(&self.norm);
        graphics.context.remove_texture(&self.depth);
        graphics.context.remove_texture(&self.mat_id);

        let positions_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Position Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&self.pos, positions_texture);

        let normals_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&self.norm, normals_texture);

        let depth_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&self.depth, depth_texture);

        let material_texture = Texture::computed(TexDimensions::size_2d(cw_half, ch_half))
            .with_label("Normals Texture")
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.context.request_texture(&self.mat_id, material_texture);
    }
}

pub struct MaterialIds {
    pub block_atlas: TextureId,
    pub grass_alpha: TextureId,
    pub sampler: SamplerId,
}

impl MaterialIds {
    pub fn init(graphics: &mut Graphics) -> Self {
        let gsam_id = TextureId("grass_side_alpha_mask");
        let atlas_id = TextureId("block_atlas");
        let samp_id = SamplerId("texture_sampler");

        let grass_alpha_mask = Texture::on_disk("./assets/grass_block_side_overlay.png")
            .with_label("Grass Side Alpha Mask")
            .writable();
        graphics.context.request_texture(&gsam_id, grass_alpha_mask);

        let atlas_texture = Texture::on_disk("./assets/textures.png")
            .with_label("Block Atlas Texture")
            .writable();
        graphics.context.request_texture(&atlas_id, atlas_texture);

        let atlas_sampler = Sampler::nearest().with_label("Atlas Sampler");
        graphics.context.request_sampler(&samp_id, atlas_sampler);

        Self {
            grass_alpha: gsam_id,
            block_atlas: atlas_id,
            sampler: samp_id,
        }
    }
}