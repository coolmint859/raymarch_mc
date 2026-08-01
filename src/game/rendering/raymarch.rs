use crate::{game::{GlobalIds, VoxelPalette, VoxelWorld}, graphics::*};

/// Ray March resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RayMarchIds {
    /// voxel buffer
    pub vox_id: BufferId,
    /// palette buffer
    pub pal_id: BufferId,
    /// ray march bind group layout
    pub bgl_id: LayoutId,
    /// ray march bind group
    pub bg_id: BindGroupId,
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
            vox_id: BufferId("voxels"),
            pal_id: BufferId("palette"),
            bgl_id: LayoutId("raymarch_layout"),
            bg_id: BindGroupId("raymarch_bind_group"),
            pip_id: PipelineId("raymarch_pipeline"),
        };

        Self { gb_ids, rm_ids }
    }

    pub fn init(&mut self, world: &VoxelWorld, graphics: &mut Graphics) {
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

        let voxel_data = world.voxel_data();
        let voxel_buffer = Buffer::as_storage(BufferContents::WithData(voxel_data))
            .with_label("Voxel Buffer")
            .with_additional_usage(wgpu::BufferUsages::COPY_DST);
        graphics.gpu.request_buffer(&self.rm_ids.vox_id, voxel_buffer);

        let raymarch_bind_group = BindGroup::new()
            .with_label("Raymarch Bind Group")
            .with_entry(BufferBinding::as_uniform(self.gb_ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.gb_ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.rm_ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE))// .with_entry(BufferBinding::as_storage(ids.reg_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(self.rm_ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.bg_id, &self.rm_ids.bgl_id, &raymarch_bind_group);

        let raymarch_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("Voxel Ray Marching Pipeline")
            .with_bg_layouts(&[self.rm_ids.bgl_id])
            .with_shader("./shaders/ray_march.wgsl");
        graphics.gpu.request_pipeline(&self.rm_ids.pip_id, &raymarch_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.rm_ids.bg_id);
        let raymarch_bind_group = BindGroup::new()
            .with_label("Raymarch Bind Group")
            .with_entry(BufferBinding::as_uniform(self.gb_ids.cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.gb_ids.env_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(self.rm_ids.pal_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(self.rm_ids.vox_id, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.rm_tex_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.rm_ids.bg_id, &self.rm_ids.bgl_id, &raymarch_bind_group);
    }

    pub fn get(&mut self, wx: u32, wy: u32) -> GpuCommand {
        GpuCommand::ComputePass(
            ComputePassInfo {
                pipeline_id: self.rm_ids.pip_id,
                bind_groups: vec![self.rm_ids.bg_id],
                work_groups: (wx, wy, 1)
            }
        )
    } 
}