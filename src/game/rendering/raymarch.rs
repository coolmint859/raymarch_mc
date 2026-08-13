use crate::{game::{GlobalIds, VoxelPalette, VoxelWorld}, graphics::*};

/// Ray March resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RayMarchIds {
    pub gsol_id: TextureId,
    /// block texture atlas sampler
    pub samp_id: SamplerId,
    /// block texture atlas
    pub atlas_id: TextureId,
    /// voxel buffer
    pub vox_id: BufferId,
    /// 16x16 brickmap
    pub grid16_id: BufferId,
    /// palette buffer
    pub pal_id: BufferId,

    /// global bind group
    pub global_bg_id: NamedBindGroup,
    /// material bind group
    pub material_bg_id: NamedBindGroup,
    /// voxel bind group
    pub voxel_bg_id: NamedBindGroup,

    /// ray march pipeline
    pub pip_id: PipelineId,
}

pub struct RayMarchPass {
    gb_ids: GlobalIds,
    rm_ids: RayMarchIds,
}

impl RayMarchPass {
    pub fn new(gb_ids: GlobalIds) -> Self {
        let rm_ids = RayMarchIds {
            gsol_id: TextureId("grass_side_alpha_mask"),
            atlas_id: TextureId("block_atlas"),
            samp_id: SamplerId("texture_sampler"),
            grid16_id: BufferId("grid16"),
            vox_id: BufferId("voxels"),
            pal_id: BufferId("palette"),
            global_bg_id: NamedBindGroup::new("global_bind_group"),
            material_bg_id: NamedBindGroup::new("material_bind_group"),
            voxel_bg_id: NamedBindGroup::new("voxel_bind_group"),
            pip_id: PipelineId("raymarch_pipeline"),
        };

        Self { gb_ids, rm_ids }
    }

    pub fn init(&mut self, world: &VoxelWorld, graphics: &mut Graphics) {
        let grass_alpha_mask = Texture::new(TextureType::OnDisk { path: "./assets/grass_block_side_overlay.png" })
            .with_label("Grass Side Overlay")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::COPY_DST);
        graphics.gpu.request_texture(&self.rm_ids.gsol_id, grass_alpha_mask);

        let atlas_texture = Texture::new(TextureType::OnDisk { path: "./assets/textures.png" })
            .with_label("Block Atlas Texture")
            .with_format(wgpu::TextureFormat::Rgba8Unorm)
            .with_additional_usage(wgpu::TextureUsages::COPY_DST);
        graphics.gpu.request_texture(&self.rm_ids.atlas_id, atlas_texture);

        let atlas_sampler = Sampler::new().with_label("Atlas Sampler");
        graphics.gpu.request_sampler(&self.rm_ids.samp_id, atlas_sampler);

        let env_data = world.env_uniform().to_bytes().to_vec();
        let env_buffer = Buffer::as_uniform(BufferContents::WithData(env_data))
            .with_label("Environment Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.gb_ids.env_id, env_buffer);

        let palette_data = VoxelPalette::create().colors;
        let palette_buffer = Buffer::as_uniform(BufferContents::WithData(palette_data))
            .with_label("Palette Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.rm_ids.pal_id, palette_buffer);

        let region_bytes = world.region_bytes();
        let voxel_buffer = Buffer::as_storage(BufferContents::WithData(region_bytes.voxels))
            .with_label("Voxel Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.rm_ids.vox_id, voxel_buffer);

        let grid_buffer = Buffer::as_storage(BufferContents::WithData(region_bytes.grids))
            .with_label("Grid Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.rm_ids.grid16_id, grid_buffer);

        let globals_bg = BindGroup::new()
            .with_label("Global Uniforms")
            .with_entry(BufferBinding::as_uniform(self.gb_ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.gb_ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.global_bg_id.id, &self.rm_ids.global_bg_id.layout_id, &globals_bg);

        let material_bg = BindGroup::new()
            .with_label("Material Uniforms")
            .with_entry(TextureBinding::as_sampled(self.rm_ids.gsol_id, TextureTypeSampled { filterable: true, multisampled: false }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.rm_ids.atlas_id, TextureTypeSampled { filterable: true, multisampled: false }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(SamplerBinding::new(self.rm_ids.samp_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.rm_ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.material_bg_id.id, &self.rm_ids.material_bg_id.layout_id, &material_bg);

        let voxel_bg = BindGroup::new()
            .with_label("Voxel Data")
            .with_entry(BufferBinding::as_storage(self.rm_ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(self.rm_ids.grid16_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.voxel_bg_id.id, &self.rm_ids.voxel_bg_id.layout_id, &voxel_bg);
        
        let raymarch_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[self.rm_ids.global_bg_id.layout_id, self.rm_ids.material_bg_id.layout_id, self.rm_ids.voxel_bg_id.layout_id])
            .with_shader("./shaders/ray_march.wgsl");
        graphics.gpu.request_pipeline(&self.rm_ids.pip_id, &raymarch_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.rm_ids.voxel_bg_id.id);
        let voxel_bg = BindGroup::new()
            .with_label("Voxel Data")
            .with_entry(BufferBinding::as_storage(self.rm_ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(self.rm_ids.grid16_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.voxel_bg_id.id, &self.rm_ids.voxel_bg_id.layout_id,&voxel_bg);
    }

    pub fn get(&mut self, wx: u32, wy: u32) -> GpuCommand {
        GpuCommand::ComputePass(
            ComputePassInfo {
                pipeline_id: self.rm_ids.pip_id,
                bind_groups: vec![self.rm_ids.global_bg_id.id, self.rm_ids.material_bg_id.id, self.rm_ids.voxel_bg_id.id],
                work_groups: (wx, wy, 1)
            }
        )
    } 
}