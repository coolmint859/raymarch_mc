use crate::{game::{PassIds, VoxelWorld, rscs::ResourceIds}, graphics::{BufferId, ComputeCommand, DrawCommand, Graphics, IndexedDraw, MultiBufferExecutor, RenderingState, SequentialExecutor}};

pub struct VoxelRenderer {
    pub resources: ResourceIds,
    passes: PassIds
}

impl VoxelRenderer {
    pub fn init(graphics: &mut Graphics, world: &VoxelWorld, cam_id: BufferId) -> Self {
        let resources = ResourceIds::init(graphics, world);
        let passes = PassIds::init(graphics, &resources, cam_id);

        Self { resources, passes }
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        self.resources.on_resize(graphics);
        self.passes.on_resize(graphics, &self.resources);
    }

    pub fn record(
        &self, 
        executor: &mut MultiBufferExecutor, 
        state: RenderingState,
        canvas_dims: (u32, u32)
    ) {
        let (cw, ch) = canvas_dims;
        let fwx = (cw + 15) / 16;
        let fwy = (ch + 15) / 16;
        let hwx = ((cw / 2) + 15) / 16;
        let hwy = ((ch / 2) + 15) / 16;

        let (taa_bg, blit_bg) = self.passes.next_bgs();

        let coarse_pass = ComputeCommand::new(
            self.passes.rm_ids.coarse_pip, 
            (hwx, hwy, 1)
        ).with_bind_groups(&[
            self.passes.rm_ids.coarse_global_bg.id,
            self.passes.rm_ids.world_bg.id,
            self.passes.rm_ids.coarse_gbuffer_bg.id
        ]);

        let fine_pass = ComputeCommand::new(
            self.passes.rm_ids.fine_pip, 
            (fwx, fwy, 1)
        ).with_bind_groups(&[
            self.passes.rm_ids.fine_global_bg.id,
            self.passes.rm_ids.material_bg.id,
            self.passes.rm_ids.world_bg.id,
            self.passes.rm_ids.fine_gbuffer_bg.id
        ]);

        let taa_pass = ComputeCommand::new(
            self.passes.taa_ids.pip_id, 
            (fwx, fwy, 1)
        ).with_bind_groups(&[taa_bg]);

        let blit_pass = DrawCommand::from_draws(state, [
            IndexedDraw::new(
                self.passes.blit_ids.pip_id, 
                0..6
            ).with_bind_groups(&[blit_bg])
        ]);

        executor.add_command(coarse_pass);
        executor.add_command(fine_pass);
        executor.add_command(taa_pass);
        executor.add_command(blit_pass);
    }
}