use std::cell::Cell;

use crate::{game::rscs::ResourceIds, graphics::{BindGroup, BindGroupId, BufferBinding, BufferId, Graphics, LayoutId, NamedBindGroup, Pipeline, PipelineId, SamplerBinding, TextureBinding, TextureTypeSampled, TextureTypeStorage}};

pub struct PassIds {
    pub rm_ids: RayMarchIds,
    pub taa_ids: TaaPassIds,
    pub blit_ids: BlitPassIds,
}

impl PassIds {
    pub fn init(graphics: &mut Graphics, rsc_ids: &ResourceIds, cam_id: BufferId) -> Self {
        Self {
            rm_ids: RayMarchIds::init(graphics, rsc_ids, cam_id),
            taa_ids: TaaPassIds::init(graphics, rsc_ids),
            blit_ids: BlitPassIds::init(graphics, rsc_ids),
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        self.rm_ids.on_resize(graphics, rsc_ids);
        self.taa_ids.on_resize(graphics, rsc_ids);
        self.blit_ids.on_resize(graphics, rsc_ids);
    }

    pub fn next_bgs(&self) -> (BindGroupId, BindGroupId) {
        let taa_bg = self.taa_ids.next_bg();
        let blit_bg = self.blit_ids.next_bg();

        return (taa_bg, blit_bg);
    }
}

pub struct RayMarchIds {
    pub world_bg: NamedBindGroup,
    pub material_bg: NamedBindGroup,

    pub coarse_global_bg: NamedBindGroup,
    pub coarse_gbuffer_bg: NamedBindGroup,
    pub coarse_pip: PipelineId,
    
    pub fine_global_bg: NamedBindGroup,
    pub fine_gbuffer_bg: NamedBindGroup,
    pub fine_pip: PipelineId,
}

impl RayMarchIds {
    pub fn init(graphics: &mut Graphics, rsc_ids: &ResourceIds, cam_id: BufferId) -> Self {
        // --- Shared Bind Groups --- //

        let world_bg_id = NamedBindGroup::new("voxel_world_bg");
        let mat_bg_id = NamedBindGroup::new("material_bg");

        let world_bg = BindGroup::new()
            .with_label("Voxel World Data")
            .with_entry(BufferBinding::as_storage(rsc_ids.world.voxels, true).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_storage(rsc_ids.world.grid, true).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&world_bg_id.id, &world_bg_id.layout_id, world_bg);

        let mat_bg = BindGroup::new()
            .with_label("Material Uniforms")
            .with_entry(TextureBinding::as_sampled(rsc_ids.material.grass_alpha, TextureTypeSampled::filterable()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.material.block_atlas, TextureTypeSampled::filterable()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(SamplerBinding::new(rsc_ids.material.sampler).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(rsc_ids.world.palette).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&mat_bg_id.id, &mat_bg_id.layout_id, mat_bg);

        // --- Coarse Bind Groups/Pipeline --- //

        let coarse_global_bg_id = NamedBindGroup::new("coarse_global_bg");
        let coarse_gbuffer_bg_id = NamedBindGroup::new("coarse_gbuffer_bg");
        let coarse_pip_id= PipelineId("rm_coarse_pipeline");

        let coarse_global_bg = BindGroup::new()
            .with_label("Global Uniforms (Coarse)")
            .with_entry(BufferBinding::as_uniform(cam_id).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&coarse_global_bg_id.id, &coarse_global_bg_id.layout_id, coarse_global_bg);

        let coarse_gbuffer_bg = BindGroup::new()
            .with_label("GBuffer Textures (Coarse)")
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.pos, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.norm, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.depth, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.mat_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&coarse_gbuffer_bg_id.id, &coarse_gbuffer_bg_id.layout_id, coarse_gbuffer_bg);

        let coarse_rm_pipeline = Pipeline::as_compute()
            .with_label("RM Coarse Pipeline")
            .with_bg_layouts(&[coarse_global_bg_id.layout_id, world_bg_id.layout_id, coarse_gbuffer_bg_id.layout_id])
            .with_shader("./shaders/coarse_rm.wgsl");
        graphics.context.request_pipeline(&coarse_pip_id, coarse_rm_pipeline);

        // --- Fine Bind Groups/Pipeline --- //

        let fine_global_bg_id = NamedBindGroup::new("fine_global_bg");
        let fine_gbuffer_id = NamedBindGroup::new("fine_gbuffer_bg");
        let fine_pip_id = PipelineId("rm_fine_pipeline");

        let fine_global_bg = BindGroup::new()
            .with_label("Global Uniforms (Fine)")
            .with_entry(BufferBinding::as_uniform(cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(BufferBinding::as_uniform(rsc_ids.world.env).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&fine_global_bg_id.id, &fine_global_bg_id.layout_id, fine_global_bg);

        let fine_gbuffer_bg = BindGroup::new()
            .with_label("GBuffer Textures (Fine)")
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.pos, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.norm, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.depth, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.mat_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.rm_tex, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&fine_gbuffer_id.id, &fine_gbuffer_id.layout_id, fine_gbuffer_bg);

        let fine_rm_pipeline = Pipeline::as_compute()
            .with_label("RM Fine Pipeline")
            .with_bg_layouts(&[
                fine_global_bg_id.layout_id, 
                mat_bg_id.layout_id, 
                world_bg_id.layout_id,
                fine_gbuffer_id.layout_id,
            ])
            .with_shader("./shaders/fine_rm.wgsl");
        graphics.context.request_pipeline(&fine_pip_id, fine_rm_pipeline);

        Self {
            world_bg: world_bg_id,
            material_bg: mat_bg_id,
            coarse_global_bg: coarse_global_bg_id,
            coarse_gbuffer_bg: coarse_gbuffer_bg_id,
            coarse_pip: coarse_pip_id,
            fine_global_bg: fine_global_bg_id,
            fine_gbuffer_bg: fine_gbuffer_id,
            fine_pip: fine_pip_id
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.coarse_gbuffer_bg.id);
        graphics.context.remove_bind_group(&self.fine_gbuffer_bg.id);

        let coarse_gbuffer_bg = BindGroup::new()
            .with_label("GBuffer Textures (Coarse)")
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.pos, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.norm, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.depth, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.mat_id, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&self.coarse_gbuffer_bg.id, &self.coarse_gbuffer_bg.layout_id, coarse_gbuffer_bg);

        let fine_gbuffer_bg = BindGroup::new()
            .with_label("GBuffer Textures (Fine)")
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.pos, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.norm, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.depth, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.mat_id, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.rm_tex, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&self.fine_gbuffer_bg.id, &self.fine_gbuffer_bg.layout_id, fine_gbuffer_bg);
    }
}

pub struct TaaPassIds {
    pub layout_id: LayoutId,
    pub bg1_id: BindGroupId,
    pub bg2_id: BindGroupId,
    pub pip_id: PipelineId,

    is_bg1: Cell<bool>,
}

impl TaaPassIds {
    pub fn init(graphics: &mut Graphics, rsc_ids: &ResourceIds) -> Self {
        let layout_id = LayoutId("taa_layout");
        let bg1_id = BindGroupId("taa_bind_group_a");
        let bg2_id = BindGroupId("taa_bind_group_b");
        let pip_id = PipelineId("taa_pipeline");

        let taa_bg1 = BindGroup::new()
            .with_label("TAA Bind Group A")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex2, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&bg1_id, &layout_id, taa_bg1);

        let taa_bg2 = BindGroup::new()
            .with_label("TAA Bind Group B")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex1, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&bg2_id, &layout_id, taa_bg2);

        let taa_pipeline = Pipeline::as_compute()
            .with_label("TAA Pipeline")
            .with_bg_layouts(&[layout_id])
            .with_shader("./shaders/taa.wgsl");
        graphics.context.request_pipeline(&pip_id, taa_pipeline);

        Self {
            layout_id,
            bg1_id,
            bg2_id,
            pip_id,
            is_bg1: Cell::new(false)
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.bg1_id);
        graphics.context.remove_bind_group(&self.bg2_id);

        let taa_bg1 = BindGroup::new()
            .with_label("TAA Bind Group A")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex2, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&self.bg1_id, &self.layout_id, taa_bg1);

        let taa_bg2 = BindGroup::new()
            .with_label("TAA Bind Group B")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
            .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex1, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE));
        graphics.context.request_bind_group(&self.bg2_id, &self.layout_id, taa_bg2);
    }

    pub fn next_bg(&self) -> BindGroupId {
        let bg_id = if self.is_bg1.get() { self.bg1_id } else {self.bg2_id };

        self.is_bg1.set(!self.is_bg1.get());

        return bg_id;
    }
}

pub struct BlitPassIds {
    pub layout_id: LayoutId,
    pub bg1_id: BindGroupId,
    pub bg2_id: BindGroupId,
    pub pip_id: PipelineId,

    is_bg1: Cell<bool>,
}

impl BlitPassIds {
    pub fn init(graphics: &mut Graphics, rsc_ids: &ResourceIds) -> Self {
        let layout_id = LayoutId("blit_layout");
        let bg1_id = BindGroupId("blit_bg1");
        let bg2_id = BindGroupId("blit_bg2");
        let pip_id = PipelineId("blit_pipeline");

        let blit_bg1 = BindGroup::new()
            .with_label("Blit Bind Group A")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()));
        graphics.context.request_bind_group(&bg1_id, &layout_id, blit_bg1);

        let blit_bg2 = BindGroup::new()
            .with_label("Blit Bind Group B")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()));
        graphics.context.request_bind_group(&bg2_id, &layout_id, blit_bg2);

        let blit_pipeline = Pipeline::as_render()
            .with_label("Voxel Render Pipeline")
            .with_bg_layouts(&[layout_id])
            .with_shader("./shaders/blit.wgsl");
        graphics.context.request_pipeline(&pip_id, blit_pipeline);

        Self {
            layout_id,
            bg1_id,
            bg2_id,
            pip_id,
            is_bg1: Cell::new(true)
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.bg1_id);
        graphics.context.remove_bind_group(&self.bg2_id);

        let blit_bg1 = BindGroup::new()
            .with_label("Blit Bind Group A")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()));
        graphics.context.request_bind_group(&self.bg1_id, &self.layout_id, blit_bg1);

        let blit_bg2 = BindGroup::new()
            .with_label("Blit Bind Group B")
            .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()));
        graphics.context.request_bind_group(&self.bg2_id, &self.layout_id, blit_bg2);
    }

    pub fn next_bg(&self) -> BindGroupId {
        let bg_id = if self.is_bg1.get() { self.bg1_id } else {self.bg2_id };

        self.is_bg1.set(!self.is_bg1.get());

        return bg_id;
    }
}