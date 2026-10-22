use crate::{game::{PassIds, VoxelWorld, rscs::ResourceIds}, graphics::{BufferUpdate, CanvasDimensions, ComputeCommand, DrawCommand, Graphics, IndexedDraw, MultiBufferExecutor, RenderingState, SequentialExecutor}, utils::{Camera, CameraSpace, RelativePerspective}};

/// Represents the state of the voxel renderer
pub enum VoxelRendererState {
    Uninit,
    Initialized {
        resources: ResourceIds,
        passes: PassIds,
    }
}

/// Holds the gpu resources needed for rendering the voxel world
pub struct VoxelRenderer {
    pub state: VoxelRendererState,
}

impl VoxelRenderer {
    pub fn new() -> Self {
        Self { state: VoxelRendererState::Uninit }
    }

    /// initialize the renderer and create all voxel ray march resources
    pub fn init(&mut self, graphics: &mut Graphics, world: &VoxelWorld, camera: &Camera<RelativePerspective>) {
        let cam_id = *camera.buf_id();
        let cam_vp = camera.get_space().view_proj_mat(graphics);
        
        let resources = ResourceIds::init(graphics, world, cam_vp);
        let passes = PassIds::init(graphics, &resources, cam_id);

        self.state = VoxelRendererState::Initialized { resources, passes };
    }

    /// Resize screen-dependent resources
    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        match &mut self.state {
            VoxelRendererState::Uninit => { return; }
            VoxelRendererState::Initialized { resources, passes } => {
                resources.on_resize(graphics);
                passes.on_resize(graphics, &resources)
            }
        }
    }

    /// update environment uniforms
    pub fn update_environment(&self, graphics: &mut Graphics, update: impl BufferUpdate) {
        match &self.state {
            VoxelRendererState::Uninit => { return; }
            VoxelRendererState::Initialized { resources, passes: _ } => {
                let env_buffer_id = resources.world.env.id();
                let _ = graphics.context.update_buffer(&env_buffer_id, update);
            }
        }
    }

    /// update camera vp matrix uniforms
    pub fn update_cam_motion(&mut self, graphics: &mut Graphics, cam_vp: glam::Mat4) {
        match &mut self.state {
            VoxelRendererState::Uninit => { return; }
            VoxelRendererState::Initialized { resources, passes: _ } => {
                resources.update_cam_motion(graphics, cam_vp);
            }
        }
    }

    /// record this frame's draw commands into the provided executor
    pub fn record(
        &self, 
        executor: &mut MultiBufferExecutor, 
        state: RenderingState,
        canvas_dims: CanvasDimensions
    ) {
        match &self.state {
            VoxelRendererState::Uninit => { return; }
            VoxelRendererState::Initialized { resources: _, passes } => {
                VoxelRenderer::record_passes(executor, state, canvas_dims, passes);
            }
        }
    }

    fn record_passes(
        executor: &mut MultiBufferExecutor, 
        state: RenderingState,
        canvas_dims: CanvasDimensions,
        passes: &PassIds,
    ) {
        let dims = canvas_dims;
        let fwx = (dims.width + 15) / 16;
        let fwy = (dims.height + 15) / 16;
        let hwx = ((dims.width / 2) + 15) / 16;
        let hwy = ((dims.height / 2) + 15) / 16;

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