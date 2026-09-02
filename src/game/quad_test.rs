use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{Buffer, BufferContents, BufferId, GpuCommand, Pipeline, PipelineId, PipelineType, RenderPassInfo, RenderPipelineType, Vec2Attribute, VertexBufferLayout}, utils::MouseHandler};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uvs: [f32; 2],
}

pub struct QuadIds {
    v_buffer_id: BufferId,
    i_buffer_id: BufferId,

    draw_pip_id: PipelineId
}

pub struct QuadTest {
    ids: QuadIds,
    mouse: MouseHandler<PlayerMouseAction>,
}

impl QuadTest {
    pub fn new() -> Self {
        let ids = QuadIds {
            v_buffer_id: BufferId("quad_vertices"),
            i_buffer_id: BufferId("quad_indices"),
            draw_pip_id: PipelineId("render_pipeline")
        };

        Self { 
            ids,
            mouse: MouseHandler::new(),
        }
    }

    pub fn init_input(&mut self) {
        self.mouse.register_button(MouseButton::Left, PlayerMouseAction::LockMouse);
        self.mouse.register_button(MouseButton::Right, PlayerMouseAction::UnlockMouse);
    }
}

impl Screen for QuadTest {
    fn init(&mut self, graphics: &mut Graphics) {
        self.init_input();

        let v_buffer_layout = VertexBufferLayout::as_vertex_step()
            .with_label("2d_shape_layout")
            .with_attribute(Vec2Attribute)
            .with_attribute(Vec2Attribute);

        let quad_vertices = [
            Vertex { position: [-0.5, -0.5], uvs: [0.0, 0.0] },
            Vertex { position: [-0.5,  0.5], uvs: [0.0, 1.0] },
            Vertex { position: [ 0.5,  0.5], uvs: [1.0, 1.0] },
            Vertex { position: [ 0.5, -0.5], uvs: [1.0, 0.0] },
        ];

        let quad_indices: [u16; 6] = [
            0, 1, 2,
            0, 2, 3,
        ];

        let vertices = bytemuck::cast_slice(&quad_vertices).to_vec();
        graphics.gpu.request_buffer(
            &self.ids.v_buffer_id, 
            Buffer::as_vertex(BufferContents::WithData(vertices))
                .with_label("Quad Vertex Buffer")
        );

        let indices = bytemuck::cast_slice(&quad_indices).to_vec();
        graphics.gpu.request_buffer(
            &self.ids.i_buffer_id, 
            Buffer::as_index(BufferContents::WithData(indices))
                .with_label("Quad Index Buffer")
        );

        graphics.gpu.request_pipeline(
            &self.ids.draw_pip_id, 
            &Pipeline::new(PipelineType::Render(RenderPipelineType::default().with_vertex_layout(&v_buffer_layout)))
                .with_label("2D Render Pipeline")
                .with_shader("./shaders/2d_draw.wgsl")
        );
    }

    fn input_event(&mut self, event: crate::InputEvent) {
        match event {
            InputEvent::MouseButton { state, button } => {
                self.mouse.button_event(state, button);
            },
            InputEvent::MouseMotion { dx, dy } => {
                self.mouse.motion_event(dx, dy);
            },
            _ => {}
        }
    }

    fn process_input(&mut self, graphics: &mut Graphics, _dt: f32) -> ScreenTransition {
        for action in self.mouse.poll_on_press() {
            match action {
                PlayerMouseAction::LockMouse => graphics.canvas.set_cursor_lock(true),
                PlayerMouseAction::UnlockMouse => graphics.canvas.set_cursor_lock(false),
            }
        }
        self.mouse.clear_events();

        ScreenTransition::None
    }

    fn update(&mut self, _graphics: &mut Graphics, _dt: f32) {}

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let draw_command = GpuCommand::RenderPass(
            RenderPassInfo { 
                pipeline_id: self.ids.draw_pip_id,
                bind_groups: Vec::new(),
                vertex_buffers: vec![self.ids.v_buffer_id],
                index_buffer: Some(self.ids.i_buffer_id),
                vertex_count: 6, 
                instance_count: 1 
            }
        );

        graphics.gpu.add_command(draw_command);
        graphics.gpu.finish(&graphics.canvas)
    }
}