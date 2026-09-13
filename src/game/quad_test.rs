use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{BindGroup, Buffer, BufferBinding, BufferId, DrawCommand, MultiBufferExecutor, NamedBindGroup, OnDisk, Pipeline, PipelineId, Sampler, SamplerBinding, SamplerId, SequentialExecutor, StructuredUpdate, Texture, TextureBinding, TextureId, TextureTypeSampled, VertexBufferLayout}, utils::{CameraController, MouseHandler, OrthographicCamera, Transform}};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uvs: [f32; 2],
}

impl Vertex {
    /// Get the buffer layout of the vertices
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::as_vertex_step(0)
            .with_attribute(wgpu::VertexFormat::Float32x3)
            .with_attribute(wgpu::VertexFormat::Float32x2)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub transform: [f32; 16],
}

impl Instance {
    /// Get the buffer layout of the instances
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::as_instance_step(2)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
    }
}

pub struct QuadIds {
    v_buffer_id: BufferId,
    i_buffer_id: BufferId,
    q_buffer_id: BufferId,
    cam_buffer_id: BufferId,
    fire_tex_id: TextureId,
    fire_samp_id: SamplerId,
    globals_bg: NamedBindGroup,
    draw_pip_id: PipelineId
}

pub struct QuadTest {
    ids: QuadIds,
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,

    // controller: CameraController,
    camera: OrthographicCamera,
}

impl QuadTest {
    pub fn new() -> Self {
        let ids = QuadIds {
            v_buffer_id: BufferId("quad_vertices"),
            i_buffer_id: BufferId("quad_indices"),
            q_buffer_id: BufferId("quad_instances"),
            cam_buffer_id: BufferId("camera_uniform"),
            fire_tex_id: TextureId("fire_tex_id"),
            fire_samp_id: SamplerId("fire_samp_id"),
            globals_bg: NamedBindGroup::new("globals_bg"),
            draw_pip_id: PipelineId("render_pipeline")
        };

        Self { 
            ids,
            mouse: MouseHandler::new(),
            executor: MultiBufferExecutor::new(),
            // controller: CameraController::new(0.5, 0.003),
            camera: OrthographicCamera::new(),
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

        let quad_instances = [
            Instance { transform: Transform::default().with_position(glam::vec3(-0.5,  0.5, 0.1)).to_updated().to_cols_array()},
            Instance { transform: Transform::default().with_position(glam::vec3( 0.5,  0.5, 0.1)).to_updated().to_cols_array()},
            Instance { transform: Transform::default().with_position(glam::vec3(-0.5, -0.5, 0.1)).to_updated().to_cols_array()},
            Instance { transform: Transform::default().with_position(glam::vec3( 0.5, -0.5, 0.1)).to_updated().to_cols_array()},
        ];

        let quad_vertices = [
            Vertex { position: [-0.2, -0.2, 0.1], uvs: [0.0, 0.0] },
            Vertex { position: [-0.2,  0.2, 0.1], uvs: [0.0, 1.0] },
            Vertex { position: [ 0.2,  0.2, 0.1], uvs: [1.0, 1.0] },
            Vertex { position: [ 0.2, -0.2, 0.1], uvs: [1.0, 0.0] },
        ];

        let quad_indices: [u16; 6] = [
            0, 1, 2,
            0, 2, 3,
        ];

        let vertices = bytemuck::cast_slice(&quad_vertices).to_vec();
        graphics.context.request_buffer(
            &self.ids.v_buffer_id, 
            Buffer::as_vertex()
                .with_byte_data(&vertices)
                .with_label("Quad Vertex Buffer")
                .writable()
        );

        let instances = bytemuck::cast_slice(&quad_instances).to_vec();
        graphics.context.request_buffer(
            &self.ids.q_buffer_id, 
            Buffer::as_vertex()
                .with_byte_data(&instances)
                .with_label("Quad Instance Buffer")
                .writable()
        );

        let indices = bytemuck::cast_slice(&quad_indices).to_vec();
        graphics.context.request_buffer(
            &self.ids.i_buffer_id, 
            Buffer::as_index()
                .with_byte_data(&indices)
                .with_label("Quad Index Buffer")
                .writable()
        );

        self.camera.update(graphics.canvas.aspect());
        println!("view_proj: {:?}", self.camera.to_uniform(graphics.canvas.frame_count()).view_proj);
        graphics.context.request_buffer(
            &self.ids.cam_buffer_id,
            Buffer::as_uniform()
                .with_struct_data(self.camera.to_uniform(graphics.canvas.frame_count()))
                .with_label("Camera Uniform")
                .writable()
        );

        graphics.context.request_texture(
            &self.ids.fire_tex_id,
            Texture::on_disk("./assets/fire.png").writable()
        );

        graphics.context.request_sampler(
            &self.ids.fire_samp_id,
            Sampler::linear()
        );

        graphics.context.request_bind_group(
            &self.ids.globals_bg.id,
            &self.ids.globals_bg.layout_id,
            BindGroup::new()
                .with_entry(BufferBinding::as_uniform(self.ids.cam_buffer_id))
                .with_entry(TextureBinding::as_sampled(self.ids.fire_tex_id, TextureTypeSampled { filterable: true, multisampled: false}))
                .with_entry(SamplerBinding::new(self.ids.fire_samp_id).with_binding_type(wgpu::SamplerBindingType::Filtering))
        );

        graphics.context.request_pipeline(
            &self.ids.draw_pip_id, 
            Pipeline::as_render()
                .with_label("2D Render Pipeline")
                .with_bg_layouts(&[self.ids.globals_bg.layout_id])
                .with_vertex_layout(Vertex::layout())
                .with_vertex_layout(Instance::layout())
                .with_shader("./shaders/2d_draw.wgsl")
        );
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

    fn update(&mut self, graphics: &mut Graphics, _dt: f32) {
        self.camera.update(graphics.canvas.aspect());
        let _ = graphics.context.update_buffer(&self.ids.cam_buffer_id, StructuredUpdate { 
            data: &self.camera.to_uniform(graphics.canvas.frame_count()),
            offset: 0
        });
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        let quad_draw = DrawCommand::new(self.ids.draw_pip_id, frame.view.clone(), 0..6)
            .with_bind_groups(&[self.ids.globals_bg.id])
            .with_vertex_buffers(&[self.ids.v_buffer_id, self.ids.q_buffer_id])
            .with_index_buffer(self.ids.i_buffer_id, wgpu::IndexFormat::Uint16)
            .with_instances(0, 4);

        self.executor.add_command(quad_draw);
        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}