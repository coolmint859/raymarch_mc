use crate::{game::{PassIds, VoxelWorld, rscs::ResourceIds}, graphics::{BufferId, BufferUpdate, ComputeCommand, DrawCommand, Graphics, IndexedDraw, MultiBufferExecutor, RenderingState, SequentialExecutor}};

pub enum RendererState {
    Uninit,
    Initialized {
        resources: ResourceIds,
        passes: PassIds,
    }
}

pub struct VoxelRenderer {
    pub state: RendererState,
}

impl VoxelRenderer {
    pub fn new() -> Self {
        Self { state: RendererState::Uninit }
    }

    pub fn init(&mut self, graphics: &mut Graphics, world: &VoxelWorld, cam_id: BufferId) {
        let resources = ResourceIds::init(graphics, world);
        let passes = PassIds::init(graphics, &resources, cam_id);

        self.state = RendererState::Initialized { resources, passes };
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        match &mut self.state {
            RendererState::Uninit => { return; }
            RendererState::Initialized { resources, passes } => {
                resources.on_resize(graphics);
                passes.on_resize(graphics, &resources)
            }
        }
    }

    pub fn update_environment(&self, graphics: &mut Graphics, update: impl BufferUpdate) {
        match &self.state {
            RendererState::Uninit => { return; }
            RendererState::Initialized { resources, passes: _ } => {
                let env_buffer_id = resources.world.env;
                let _ = graphics.context.update_buffer(&env_buffer_id, update);
            }
        }
    }

    pub fn record(
        &self, 
        executor: &mut MultiBufferExecutor, 
        state: RenderingState,
        canvas_dims: (u32, u32)
    ) {
        match &self.state {
            RendererState::Uninit => { return; }
            RendererState::Initialized { resources: _, passes } => {
                VoxelRenderer::record_passes(executor, state, canvas_dims, passes);
            }
        }
    }

    fn record_passes(
        executor: &mut MultiBufferExecutor, 
        state: RenderingState,
        canvas_dims: (u32, u32),
        passes: &PassIds,
    ) {
        let (cw, ch) = canvas_dims;
        let fwx = (cw + 15) / 16;
        let fwy = (ch + 15) / 16;
        let hwx = ((cw / 2) + 15) / 16;
        let hwy = ((ch / 2) + 15) / 16;

        let (taa_bg, blit_bg) = passes.next_bgs();

        let coarse_pass = ComputeCommand::new(
            passes.rm_ids.coarse_pip, 
            (hwx, hwy, 1)
        ).with_bind_groups(&[
            passes.rm_ids.coarse_global_bg.id,
            passes.rm_ids.world_bg.id,
            passes.rm_ids.coarse_gbuffer_bg.id
        ]);

        let fine_pass = ComputeCommand::new(
            passes.rm_ids.fine_pip, 
            (fwx, fwy, 1)
        ).with_bind_groups(&[
            passes.rm_ids.fine_global_bg.id,
            passes.rm_ids.material_bg.id,
            passes.rm_ids.world_bg.id,
            passes.rm_ids.fine_gbuffer_bg.id
        ]);

        let taa_pass = ComputeCommand::new(
            passes.taa_ids.pip_id, 
            (fwx, fwy, 1)
        ).with_bind_groups(&[taa_bg]);

        let blit_pass = DrawCommand::from_draws(state, [
            IndexedDraw::new(
                passes.blit_ids.pip_id, 
                0..6
            ).with_bind_groups(&[blit_bg])
        ]);

        executor.add_command(coarse_pass);
        executor.add_command(fine_pass);
        executor.add_command(taa_pass);
        executor.add_command(blit_pass);
    }
}