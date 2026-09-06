use crate::{game::GlobalIds, graphics::*};

/// Temporal Anti-aliasing resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TaaIds {
    /// taa bind group layout
    pub bgl_id: LayoutId,
    /// taa bind group A
    pub bg_a_id: BindGroupId,
    /// taa bind group B
    pub bg_b_id: BindGroupId,
    /// taa pipeline
    pub pip_id: PipelineId,
}

pub struct TaaPass {
    gb_ids: GlobalIds,
    taa_ids: TaaIds,
    is_bg_a: bool,
}

impl TaaPass {
    pub fn new(gb_ids: GlobalIds) -> Self {
        let taa_ids = TaaIds {
            bgl_id: LayoutId("taa_layout"),
            bg_a_id: BindGroupId("taa_bind_group_a"),
            bg_b_id: BindGroupId("taa_bind_group_b"),
            pip_id: PipelineId("taa_pipeline"),
        };

        Self {
            gb_ids,
            taa_ids,
            is_bg_a: false,
        }
    }

    pub fn init(&mut self, graphics: &mut Graphics) {
        let taa_bind_group_a = BindGroup::new()
            .with_label("TAA Bind Group A")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.rm_tex_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_a_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.taa_tex_b_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.taa_ids.bg_a_id, &self.taa_ids.bgl_id, &taa_bind_group_a);

        let taa_bind_group_b = BindGroup::new()
            .with_label("TAA Bind Group B")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.rm_tex_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_b_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.taa_tex_a_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.taa_ids.bg_b_id, &self.taa_ids.bgl_id, &taa_bind_group_b);

        let taa_pipeline = Pipeline::new(PipelineType::Compute(ComputePipelineType::default()))
            .with_label("TAA Pipeline")
            .with_bg_layouts(&[self.taa_ids.bgl_id])
            .with_shader("./shaders/taa.wgsl");
        graphics.gpu.request_pipeline(&self.taa_ids.pip_id, &taa_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.taa_ids.bg_a_id);
        let taa_bind_group_a = BindGroup::new()
            .with_label("TAA Bind Group A")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.rm_tex_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_a_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.taa_tex_b_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.taa_ids.bg_a_id, &self.taa_ids.bgl_id, &taa_bind_group_a);

        graphics.gpu.remove_bind_group(&self.taa_ids.bg_b_id);
        let taa_bind_group_b = BindGroup::new()
            .with_label("TAA Bind Group B")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.rm_tex_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_b_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(self.gb_ids.taa_tex_a_id, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.gpu.request_bind_group(&self.taa_ids.bg_b_id, &self.taa_ids.bgl_id, &taa_bind_group_b);

        self.is_bg_a = false;
    }

    pub fn get(&mut self, wx: u32, wy: u32) -> ComputeCommand {
        let taa_bg = if self.is_bg_a {
            self.taa_ids.bg_a_id
        } else {
            self.taa_ids.bg_b_id
        };

        self.is_bg_a = !self.is_bg_a;

        ComputeCommand::new(self.taa_ids.pip_id, (wx, wy, 1))
            .with_bind_groups(&[taa_bg])
    }
}