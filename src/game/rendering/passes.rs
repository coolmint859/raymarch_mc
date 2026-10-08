use std::cell::Cell;

use crate::{game::rscs::ResourceIds, graphics::{BindGroup, BindGroupId, BindGroupIdPair, BufferBinding, BufferId, Graphics, LayoutId, Pipeline, PipelineId, SamplerBinding, TextureBinding, TextureTypeSampled, TextureTypeStorage}};

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
    pub world_bg: BindGroupIdPair,
    pub material_bg: BindGroupIdPair,

    pub coarse_global_bg: BindGroupIdPair,
    pub coarse_gbuffer_bg: BindGroupIdPair,
    pub coarse_pip: PipelineId,
    
    pub fine_global_bg: BindGroupIdPair,
    pub fine_gbuffer_bg: BindGroupIdPair,
    pub fine_pip: PipelineId,
}

impl RayMarchIds {
    pub fn init(graphics: &mut Graphics, rsc_ids: &ResourceIds, cam_id: BufferId) -> Self {
        // --- Shared Bind Groups --- //
        let world_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Voxel World Data")
                .with_entry(BufferBinding::as_storage(rsc_ids.world.voxels, true).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(BufferBinding::as_storage(rsc_ids.world.grid, true).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let mat_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Material Uniforms")
                .with_entry(TextureBinding::as_sampled(rsc_ids.material.grass_alpha, TextureTypeSampled::filterable()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.material.block_atlas, TextureTypeSampled::filterable()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(SamplerBinding::new(rsc_ids.material.sampler).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(BufferBinding::as_uniform(rsc_ids.world.palette).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        // --- Coarse Bind Groups/Pipeline --- //
        let coarse_global_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Global Uniforms (Coarse)")
                .with_entry(BufferBinding::as_uniform(cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let coarse_gbuffer_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("GBuffer Textures (Coarse)")
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.pos, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.norm, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.depth, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.mat, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let coarse_pip = graphics.context.request_pipeline(
            Pipeline::as_compute()
                .with_label("RM Coarse Pipeline")
                .with_bg_layouts(&[coarse_global_bg.layout_id, world_bg.layout_id, coarse_gbuffer_bg.layout_id])
                .with_shader("./shaders/coarse_rm.wgsl")
        );

        // --- Fine Bind Groups/Pipeline --- //
        let fine_global_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Global Uniforms (Fine)")
                .with_entry(BufferBinding::as_uniform(cam_id).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(BufferBinding::as_uniform(rsc_ids.world.env).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let fine_gbuffer_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("GBuffer Textures (Fine)")
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.pos, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.norm, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.depth, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.mat, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.rm_tex, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let fine_pip = graphics.context.request_pipeline(
            Pipeline::as_compute()
                .with_label("RM Fine Pipeline")
                .with_bg_layouts(&[
                    fine_global_bg.layout_id, 
                    mat_bg.layout_id, 
                    world_bg.layout_id,
                    fine_gbuffer_bg.layout_id,
                ])
                .with_shader("./shaders/fine_rm.wgsl")
        );

        Self {
            world_bg,
            material_bg: mat_bg,
            coarse_global_bg,
            coarse_gbuffer_bg,
            coarse_pip,
            fine_global_bg,

            fine_gbuffer_bg,
            fine_pip
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.coarse_gbuffer_bg.id);
        graphics.context.remove_bind_group(&self.fine_gbuffer_bg.id);

        self.coarse_gbuffer_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("GBuffer Textures (Coarse)")
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.pos, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba16Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.norm, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::Rgba8Unorm }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.depth, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.g_buffer.mat, TextureTypeStorage { access: wgpu::StorageTextureAccess::WriteOnly, fmt: wgpu::TextureFormat::R32Float }).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        self.fine_gbuffer_bg = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("GBuffer Textures (Fine)")
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.pos, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.norm, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.depth, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.g_buffer.mat, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.rm_tex, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );
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
        let taa_bg1 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("TAA Bind Group A")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex2, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let taa_bg2 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("TAA Bind Group B")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex1, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        assert!(taa_bg1.layout_id == taa_bg2.layout_id);

        let pip_id = graphics.context.request_pipeline(
            Pipeline::as_compute()
                .with_label("TAA Pipeline")
                .with_bg_layouts(&[taa_bg1.layout_id])
                .with_shader("./shaders/taa.wgsl")
        );

        Self {
            layout_id: taa_bg1.layout_id,
            bg1_id: taa_bg1.id,
            bg2_id: taa_bg2.id,
            pip_id,
            is_bg1: Cell::new(false)
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.bg1_id);
        graphics.context.remove_bind_group(&self.bg2_id);

        let taa_bg1 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("TAA Bind Group A")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex2, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        let taa_bg2 =graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("TAA Bind Group B")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.rm_tex, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
                .with_entry(TextureBinding::as_storage(rsc_ids.taa.tex1, TextureTypeStorage::default()).with_visibility(wgpu::ShaderStages::COMPUTE))
        );

        assert!(taa_bg1.layout_id == taa_bg2.layout_id);

        self.layout_id = taa_bg1.layout_id;
        self.bg1_id = taa_bg1.id;
        self.bg2_id = taa_bg2.id;
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
        let blit_bg1 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Blit Bind Group A")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()))
        );

        let blit_bg2 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Blit Bind Group B")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()))
        );

        assert!(blit_bg1.layout_id == blit_bg2.layout_id);

        let pip_id = graphics.context.request_pipeline(
            Pipeline::as_render()
                .with_label("Voxel Render Pipeline")
                .with_bg_layouts(&[blit_bg1.layout_id])
                .with_shader("./shaders/blit.wgsl")
        );

        Self {
            layout_id: blit_bg1.layout_id,
            bg1_id: blit_bg1.id,
            bg2_id: blit_bg2.id,
            pip_id,
            is_bg1: Cell::new(true)
        }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics, rsc_ids: &ResourceIds) {
        graphics.context.remove_bind_group(&self.bg1_id);
        graphics.context.remove_bind_group(&self.bg2_id);

        let blit_bg1 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Blit Bind Group A")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex1, TextureTypeSampled::default()))
        );

        let blit_bg2 = graphics.context.request_bind_group(
            BindGroup::new()
                .with_label("Blit Bind Group B")
                .with_entry(TextureBinding::as_sampled(rsc_ids.taa.tex2, TextureTypeSampled::default()))
        );

        assert!(blit_bg1.layout_id == blit_bg2.layout_id);

        self.layout_id = blit_bg1.layout_id;
        self.bg1_id = blit_bg1.id;
        self.bg2_id = blit_bg2.id;
    }

    pub fn next_bg(&self) -> BindGroupId {
        let bg_id = if self.is_bg1.get() { self.bg1_id } else {self.bg2_id };

        self.is_bg1.set(!self.is_bg1.get());

        return bg_id;
    }
}