use crate::{game::{GlobalIds, VoxelPalette, VoxelWorld}, graphics::*};

/// Ray March resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RayMarchIds {
    /// voxel buffer
    pub vox_id: BufferId,
    /// grid bitmask
    pub grid_id: BufferId,

    /// positions buffer
    pub pos_id: TextureId,
    /// normal buffer
    pub norm_id: TextureId,
    /// depth buffer
    pub depth_id: TextureId,
    /// material buffer
    pub mat_id: TextureId,

    /// voxel data bind group
    pub vox_bg_id: NamedBindGroup,
}

/// Ray March resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CoarseIds {
    /// global bind group
    pub global_bg_id: NamedBindGroup,
    /// deferred texture bind group
    pub deferred_tex_bg_id: NamedBindGroup,

    /// coarse ray march pipeline
    pub pip_id: PipelineId,
}

pub struct FineIds {
    /// grass alpha mask
    pub gsam_id: TextureId,
    /// block texture atlas
    pub atlas_id: TextureId,
    /// block texture atlas sampler
    pub samp_id: SamplerId,
    /// palette buffer
    pub pal_id: BufferId,

    /// global bind group
    pub global_bg_id: NamedBindGroup,
    /// material bind group
    pub material_bg_id: NamedBindGroup,
    /// screen_space texture bind group
    pub screen_textures_bg: NamedBindGroup,

    /// ray march pipeline
    pub pip_id: PipelineId,
}

/// Resources that both ray march passes use
pub struct RayMarchResources {
    pub ids: RayMarchIds,
}

impl RayMarchResources {
    pub fn new() -> Self {
        let ids = RayMarchIds {
            vox_id: BufferId("voxels"),
            grid_id: BufferId("grid_bitmask"),

            pos_id: TextureId("coarse_positions"),
            norm_id: TextureId("coarse_normals"),
            depth_id: TextureId("coarse_depth"),
            mat_id: TextureId("coarse_material_ids"),

            vox_bg_id: NamedBindGroup::new("voxel_bind_group"),
        };

        Self { ids }
    }

    pub fn init(&self, graphics: &mut Graphics, world: &VoxelWorld) {
        let region_bytes = world.region_bytes();
        let voxel_buffer = Buffer::as_storage(BufferContents::WithData(region_bytes.voxels))
            .with_label("Voxel Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.ids.vox_id, voxel_buffer);

        let grid_buffer = Buffer::as_storage(BufferContents::WithData(region_bytes.grids))
            .with_label("Grid Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.ids.grid_id, grid_buffer);

        let voxel_bg = BindGroup::new()
            .with_label("Voxel Data")
            .with_entry(BufferBinding::as_storage(self.ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(self.ids.grid_id, true).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.ids.vox_bg_id.id, &self.ids.vox_bg_id.layout_id, &voxel_bg);

        self.create_textures(graphics);
    }

    pub fn on_resize(&self, graphics: &mut Graphics) {
        graphics.gpu.remove_texture(&self.ids.pos_id);
        graphics.gpu.remove_texture(&self.ids.norm_id);
        graphics.gpu.remove_texture(&self.ids.depth_id);
        graphics.gpu.remove_texture(&self.ids.mat_id);

        self.create_textures(graphics);
    }

    fn create_textures(&self, graphics: &mut Graphics) {
        // coarse pass runs at half-resolution
        let half_canvas_width = graphics.canvas.config.width / 2;
        let half_canvas_height = graphics.canvas.config.height / 2;

        let positions_texture = Texture::new(TextureType::Computed)
            .with_label("Position Texture")
            .with_size_2d(half_canvas_width, half_canvas_height)
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.gpu.request_texture(&self.ids.pos_id, positions_texture);

        let normals_texture = Texture::new(TextureType::Computed)
            .with_label("Normals Texture")
            .with_size_2d(half_canvas_width, half_canvas_height)
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.gpu.request_texture(&self.ids.norm_id, normals_texture);

        let depth_texture = Texture::new(TextureType::Computed)
            .with_label("Normals Texture")
            .with_size_2d(half_canvas_width, half_canvas_height)
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.gpu.request_texture(&self.ids.depth_id, depth_texture);

        let material_texture = Texture::new(TextureType::Computed)
            .with_label("Normals Texture")
            .with_size_2d(half_canvas_width, half_canvas_height)
            .with_format(wgpu::TextureFormat::R32Float)
            .with_additional_usage(wgpu::TextureUsages::STORAGE_BINDING)
            .with_additional_usage(wgpu::TextureUsages::COPY_SRC);
        graphics.gpu.request_texture(&self.ids.mat_id, material_texture);
    }
}

pub struct RayMarchFinePass {
    gb_ids: GlobalIds,
    rm_ids: RayMarchIds,
    fine_ids: FineIds,
}

impl RayMarchFinePass {
    pub fn new(gb_ids: GlobalIds, rm_ids: RayMarchIds) -> Self {
        let fine_ids = FineIds {
            gsam_id: TextureId("grass_side_alpha_mask"),
            atlas_id: TextureId("block_atlas"),
            samp_id: SamplerId("texture_sampler"),
            pal_id: BufferId("palette"),
            global_bg_id: NamedBindGroup::new("global_bind_group_fine"),
            material_bg_id: NamedBindGroup::new("material_bind_group"),
            screen_textures_bg: NamedBindGroup::new("screen_textures_bind_group_fine"),
            pip_id: PipelineId("rm_fine_pipeline"),
        };

        Self { gb_ids, rm_ids, fine_ids }
    }

    pub fn init(&mut self, graphics: &mut Graphics, world: &VoxelWorld) {
        let grass_alpha_mask = Texture::new(TextureType::OnDisk { path: "./assets/grass_block_side_overlay.png" })
            .with_label("Grass Side Alpha Mask")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::COPY_DST);
        graphics.gpu.request_texture(&self.fine_ids.gsam_id, grass_alpha_mask);

        let atlas_texture = Texture::new(TextureType::OnDisk { path: "./assets/textures.png" })
            .with_label("Block Atlas Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::COPY_DST);
        graphics.gpu.request_texture(&self.fine_ids.atlas_id, atlas_texture);

        let atlas_sampler = Sampler::new().with_label("Atlas Sampler");
        graphics.gpu.request_sampler(&self.fine_ids.samp_id, atlas_sampler);

        let env_data = world.env_uniform().to_bytes().to_vec();
        let env_buffer = Buffer::as_uniform(BufferContents::WithData(env_data))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.gb_ids.env_id, env_buffer);

        let palette_data = VoxelPalette::create().colors;
        let palette_buffer = Buffer::as_uniform(BufferContents::WithData(palette_data))
            .with_label("Palette Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.fine_ids.pal_id, palette_buffer);

        let globals_bg = BindGroup::new()
            .with_label("Global Uniforms (Fine)")
            .with_entry(BufferBinding::as_uniform(self.gb_ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.gb_ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.fine_ids.global_bg_id.id, &self.fine_ids.global_bg_id.layout_id, &globals_bg);

        let material_bg = BindGroup::new()
            .with_label("Material Uniforms")
            .with_entry(TextureBinding::as_sampled(self.fine_ids.gsam_id, TextureTypeSampled { filterable: true, multisampled: false }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.fine_ids.atlas_id, TextureTypeSampled { filterable: true, multisampled: false }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(SamplerBinding::new(self.fine_ids.samp_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.fine_ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.fine_ids.material_bg_id.id, &self.fine_ids.material_bg_id.layout_id, &material_bg);

        let deferred_textures_bg = BindGroup::new()
            .with_label("Deferred Texture Bind Group (Fine)")
            .with_entry(TextureBinding::as_sampled(self.rm_ids.pos_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.norm_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.depth_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.mat_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.fine_ids.screen_textures_bg.id, &self.fine_ids.screen_textures_bg.layout_id, &deferred_textures_bg);

        let raymarch_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("RM Fine Pipeline")
            .with_bg_layouts(&[
                self.fine_ids.global_bg_id.layout_id, 
                self.fine_ids.material_bg_id.layout_id, 
                self.rm_ids.vox_bg_id.layout_id,
                self.fine_ids.screen_textures_bg.layout_id,
            ])
            .with_shader("./shaders/fine_rm.wgsl");
        graphics.gpu.request_pipeline(&self.fine_ids.pip_id, &raymarch_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.fine_ids.screen_textures_bg.id);
        let deferred_textures_bg = BindGroup::new()
            .with_label("Deferred Texture Bind Group (Fine)")
            .with_entry(TextureBinding::as_sampled(self.rm_ids.pos_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.norm_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.depth_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.mat_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.fine_ids.screen_textures_bg.id, &self.fine_ids.screen_textures_bg.layout_id, &deferred_textures_bg);
    }

    pub fn get(&mut self, wx: u32, wy: u32) -> ComputeCommand {
        ComputeCommand::new(self.fine_ids.pip_id, (wx, wy, 1))
            .with_bind_groups(&[
                self.fine_ids.global_bg_id.id, 
                self.fine_ids.material_bg_id.id, 
                self.rm_ids.vox_bg_id.id,
                self.fine_ids.screen_textures_bg.id,
            ])
    } 
}

pub struct CoarsePass {
    gb_ids: GlobalIds,
    rm_ids: RayMarchIds,
    coarse_ids: CoarseIds,
}

impl CoarsePass {
    pub fn new(gb_ids: GlobalIds, rm_ids: RayMarchIds) -> Self {
        let coarse_ids = CoarseIds {
            global_bg_id: NamedBindGroup::new("global_bind_group_coarse"),
            deferred_tex_bg_id: NamedBindGroup::new("deferred_textures_bind_group_coarse"),
            pip_id: PipelineId("rm_coarse_pipeline"),
        };

        Self { gb_ids, rm_ids, coarse_ids }
    }

    pub fn init(&mut self, graphics: &mut Graphics) {
        let globals_bg = BindGroup::new()
            .with_label("Global Uniforms (Coarse)")
            .with_entry(BufferBinding::as_uniform(self.gb_ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.coarse_ids.global_bg_id.id, &self.coarse_ids.global_bg_id.layout_id, &globals_bg);

        let deferred_textures_bg = BindGroup::new()
            .with_label("Deferred Texture Bind Group (Coarse)")
            .with_entry(TextureBinding::as_storage(self.rm_ids.pos_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.norm_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.depth_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.mat_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.coarse_ids.deferred_tex_bg_id.id, &self.coarse_ids.deferred_tex_bg_id.layout_id, &deferred_textures_bg);

        let raymarch_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("RM Coarse Pipeline")
            .with_bg_layouts(&[self.coarse_ids.global_bg_id.layout_id, self.rm_ids.vox_bg_id.layout_id, self.coarse_ids.deferred_tex_bg_id.layout_id])
            .with_shader("./shaders/coarse_rm.wgsl");
        graphics.gpu.request_pipeline(&self.coarse_ids.pip_id, &raymarch_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.coarse_ids.deferred_tex_bg_id.id);
        let deferred_textures_bg = BindGroup::new()
            .with_label("Deferred Texture Bind Group (Coarse)")
            .with_entry(TextureBinding::as_storage(self.rm_ids.pos_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.norm_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.depth_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.rm_ids.mat_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.coarse_ids.deferred_tex_bg_id.id, &self.coarse_ids.deferred_tex_bg_id.layout_id, &deferred_textures_bg);
    }

    pub fn get(&mut self, wx: u32, wy: u32) -> ComputeCommand {
        ComputeCommand::new(self.coarse_ids.pip_id, (wx, wy, 1))
            .with_bind_groups(&[
                self.coarse_ids.global_bg_id.id, 
                self.rm_ids.vox_bg_id.id, 
                self.coarse_ids.deferred_tex_bg_id.id
            ])
    } 
}