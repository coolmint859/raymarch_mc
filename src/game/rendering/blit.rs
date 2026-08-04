use crate::{game::GlobalIds, graphics::*};

/// Blit resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlitIds {
    /// blit bind group layout
    pub bgl_id: LayoutId,
    /// blit bind group A
    pub bg_a_id: BindGroupId,
    /// blit bind group B
    pub bg_b_id: BindGroupId,
    /// blit pipeline
    pub pip_id: PipelineId,
}

pub struct BlitPass {
    gb_ids: GlobalIds,
    blit_ids: BlitIds,
    is_bg_a: bool,
}

impl BlitPass {
    pub fn new(gb_ids: GlobalIds) -> Self {
        let blit_ids = BlitIds {
            bgl_id: LayoutId("blit_layout"),
            bg_a_id: BindGroupId("blit_bind_group_a"),
            bg_b_id: BindGroupId("blit_bind_group_b"),
            pip_id: PipelineId("blit_pipeline"),
        };

        Self {
            gb_ids,
            blit_ids,
            is_bg_a: true,
        }
    }

    pub fn init(&mut self, graphics: &mut Graphics) {
        let blit_bind_group_a = BindGroup::new()
            .with_label("Blit Bind Group A")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_a_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&self.blit_ids.bg_a_id, &self.blit_ids.bgl_id, &blit_bind_group_a);

        let blit_bind_group_b = BindGroup::new()
            .with_label("Blit Bind Group B")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_b_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&self.blit_ids.bg_b_id, &self.blit_ids.bgl_id, &blit_bind_group_b);

        let blit_pipeline = Pipeline::new(PipelineType::Render(RenderPipelineType::default()))
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[self.blit_ids.bgl_id])
            .with_shader("./shaders/blit.wgsl");
        graphics.gpu.request_pipeline(&self.blit_ids.pip_id, &blit_pipeline);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        graphics.gpu.remove_bind_group(&self.blit_ids.bg_a_id);
        let blit_bind_group_a = BindGroup::new()
            .with_label("Blit Bind Group A")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_a_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&self.blit_ids.bg_a_id, &self.blit_ids.bgl_id, &blit_bind_group_a);

        graphics.gpu.remove_bind_group(&self.blit_ids.bg_b_id);
        let blit_bind_group_b = BindGroup::new()
            .with_label("Blit Bind Group B")
            .with_entry(TextureBinding::as_sampled(self.gb_ids.taa_tex_b_id, TextureTypeSampled::default()));
        graphics.gpu.request_bind_group(&self.blit_ids.bg_b_id, &self.blit_ids.bgl_id, &blit_bind_group_b);

        self.is_bg_a = true;
    }

    pub fn get(&mut self) -> GpuCommand {
        let blit_bg = if self.is_bg_a {
            self.blit_ids.bg_a_id
        } else {
            self.blit_ids.bg_b_id
        };

        self.is_bg_a = !self.is_bg_a;

        GpuCommand::RenderPass(
            RenderPassInfo { 
                pipeline_id: self.blit_ids.pip_id,
                bind_groups: vec![blit_bg], 
                vertex_count: 3, 
                instance_count: 1 
            }
        )
    }
}