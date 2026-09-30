use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{BindGroup, Buffer, BufferBinding, BufferId, DrawCommand, RenderingState, IndexedDraw, MultiBufferExecutor, NamedBindGroup, Pipeline, PipelineId, Sampler, SamplerBinding, SamplerId, SequentialExecutor, Texture, TextureBinding, TextureId, TextureTypeSampled, VertexBufferLayout}, utils::{Camera, MouseHandler, ScreenSpace, Transform, font_asset::{Quad, Vertex}}};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct QuadInstance {
    pub transform: [f32; 16],
}

impl QuadInstance {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::as_instance_step(2)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
    }
}

pub struct QuadTest {
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,
    camera: Camera<ScreenSpace>,
    quad: Quad,

    q1_instances: BufferId,
    q1_inst_data: Vec<QuadInstance>,
    blue_devils: TextureId,

    q2_instances: BufferId,
    q2_inst_data: Vec<QuadInstance>,
    scv: TextureId,

    samp: SamplerId,
    qbg1: NamedBindGroup,
    qbg2: NamedBindGroup,

    qpip: PipelineId,
}

impl QuadTest {
    pub fn new() -> Self {
        Self { 
            mouse: MouseHandler::new(),
            executor: MultiBufferExecutor::new(),
            camera: Camera::new(ScreenSpace),
            quad: Quad::new(),

            q1_instances: BufferId("q1_instances"),
            q1_inst_data: Vec::new(),
            blue_devils: TextureId("blue_devils"),

            q2_instances: BufferId("q2_instances"),
            q2_inst_data: Vec::new(),
            scv: TextureId("scv"),
            
            samp: SamplerId("tex_sampler"),
            qbg1: NamedBindGroup::new("qbg1"),
            qbg2: NamedBindGroup::new("qbg2"),

            qpip: PipelineId("qpip"),
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

        self.quad.request_buffers(&mut graphics.context);

        self.q1_inst_data.extend_from_slice(&[
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(0.60, 0.75, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(0.60, 0.25, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array()  
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(0.25, 0.75, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(0.25, 0.25, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            }
        ]);

        self.q2_inst_data.extend_from_slice(&[
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(1.25, 0.75, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(1.25, 0.25, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array()  
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(1.55, 0.75, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            },
            QuadInstance { 
                transform: Transform::from_position(glam::vec3(1.55, 0.25, 0.0))
                    .with_scale(glam::vec3(0.1, 0.1, 0.1))
                    .to_cols_array() 
            }
        ]);

        graphics.context.request_buffer(
            &self.q1_instances, 
            Buffer::as_vertex()
                .with_label("Quad1 Instances")
                .with_byte_data(&bytemuck::cast_slice(&self.q1_inst_data))
                .writable()
        );

        graphics.context.request_buffer(
            &self.q2_instances, 
            Buffer::as_vertex()
                .with_label("Quad2 Instances")
                .with_byte_data(&bytemuck::cast_slice(&self.q2_inst_data))
                .writable()
        );

        graphics.context.request_texture(
            &self.blue_devils,
            Texture::on_disk("./assets/BlueDevilsLogo.png")
                .with_label("Blue Devils")
                .writable()
        );

        graphics.context.request_texture(
            &self.scv,
            Texture::on_disk("./assets/vanguard.jpg")
                .with_label("Santa Clara Vanguard")
                .writable()
        );

        graphics.context.request_sampler(&self.samp, Sampler::nearest());

        graphics.context.request_bind_group(
            &self.qbg1.id, 
            &self.qbg1.layout_id, 
            BindGroup::new()
                .with_label("Quad1 Bind Group")
                .with_entry(BufferBinding::as_uniform(*self.camera.buf_id()))
                .with_entry(TextureBinding::as_sampled(self.blue_devils, TextureTypeSampled::default()))
                .with_entry(SamplerBinding::new(self.samp))
        );

        graphics.context.request_bind_group(
            &self.qbg2.id, 
            &self.qbg1.layout_id, 
            BindGroup::new()
                .with_label("Quad2 Bind Group")
                .with_entry(BufferBinding::as_uniform(*self.camera.buf_id()))
                .with_entry(TextureBinding::as_sampled(self.scv, TextureTypeSampled::default()))
                .with_entry(SamplerBinding::new(self.samp))
        );

        graphics.context.request_pipeline(
            &self.qpip, 
            Pipeline::as_render()
                .with_label("Quad Pipeline")
                .with_bg_layouts(&[self.qbg1.layout_id])
                .with_vertex_layout(Vertex::layout())
                .with_vertex_layout(QuadInstance::layout())
                .with_shader("./shaders/2d_draw.wgsl")
        );

        self.camera.init(graphics);
    }

    fn input_event(&mut self, event: InputEvent) {
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

    fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        self.camera.update(graphics, dt);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        let draw_q1 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.qbg1.id])
            .with_vertex_buffers(&[self.quad.vbuffer_id, self.q1_instances])
            .with_index_buffer(self.quad.ibuffer_id, wgpu::IndexFormat::Uint16)
            .with_instances(0..self.q1_inst_data.len() as u32);

        let draw_q2 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.qbg2.id])
            .with_vertex_buffers(&[self.quad.vbuffer_id, self.q2_instances])
            .with_index_buffer(self.quad.ibuffer_id, wgpu::IndexFormat::Uint16)
            .with_instances(0..self.q2_inst_data.len() as u32);

        let draw_cmd = DrawCommand::from_draws(
            RenderingState {
                output_view: frame.view.clone(),
                clear_color: Some(wgpu::Color::BLACK)
            }, 
            vec![draw_q1, draw_q2]
        );

        self.executor.add_command(draw_cmd);
        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}