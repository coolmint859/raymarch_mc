use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{BindGroup, BufferBinding, BufferId, DrawCommand, GpuContext, IndexedDraw, MultiBufferExecutor, NamedBindGroup, Pipeline, PipelineId, RenderingState, Sampler, SamplerBinding, SamplerId, SequentialExecutor, Texture, TextureBinding, TextureId, TextureTypeSampled, VertexBufferLayout}, utils::{Camera, GeometryData, Initialized, MouseHandler, ScreenSpace, Transform, TransformAttribute, font::font::Quad}};

pub struct QuadPrimitive {
    pub quad: Quad,
    pub instances: GeometryData<Initialized>
}

impl QuadPrimitive {
    pub fn new() -> Self {
        Self {
            quad: Quad::new(),
            instances: GeometryData::placeholder()
        }
    }

    pub fn init(&mut self, context: &mut GpuContext) {
        self.quad.init(context);

        let start_loc = self.quad.vertices.attr_count();
        self.instances = GeometryData::as_instance_group(start_loc)
            .with_label("Font Quad Instances")
            .with_attribute(TransformAttribute("transform"), Vec::<Transform>::new())            
            .init(context, 4);
    }

    /// Get the ids to the geometry (vertex/instance) buffers for this quad.
    pub fn geo_buf_ids(&self) -> [BufferId; 2] {
        [*self.quad.vertices.buf_id(), *self.instances.buf_id()]
    }

    pub fn idx_buf_id(&self) -> BufferId {
        self.quad.idx_buf_id
    }

    pub fn vertex_layout(&self) -> VertexBufferLayout {
        self.quad.vertices.layout().clone()
    }

    pub fn instance_layout(&self) -> VertexBufferLayout {
        self.instances.layout().clone()
    }
}

pub struct QuadTest {
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,
    camera: Camera<ScreenSpace>,

    quad1: QuadPrimitive,
    quad2: QuadPrimitive,
    blue_devils: TextureId,
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

            quad1: QuadPrimitive::new(),
            quad2: QuadPrimitive::new(),
            blue_devils: TextureId("blue_devils"),
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

        self.quad1.init(&mut graphics.context);
        self.quad1.instances.extend_attribute::<Transform>(
            "transform", 
            vec![
                Transform::from_position(glam::vec3(0.60, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(0.60, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(0.25, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(0.25, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1))
            ]
        );

        self.quad2.init(&mut graphics.context);
        self.quad2.instances.extend_attribute::<Transform>(
            "transform", 
            vec![
                Transform::from_position(glam::vec3(1.20, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(1.20, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(1.55, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
                Transform::from_position(glam::vec3(1.55, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1))
            ]
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
                .with_vertex_layout(self.quad1.vertex_layout())
                .with_vertex_layout(self.quad1.instance_layout())
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

        {
            let q1_inst_proxy = self.quad1.instances.borrow_mut();
            let q2_inst_proxy = self.quad2.instances.borrow_mut();

            if let (Some(mut q1_transforms), Some(mut q2_transforms)) = (
                q1_inst_proxy.get_attribute_mut::<Transform>("transform"),
                q2_inst_proxy.get_attribute_mut::<Transform>("transform")
            ) {
                for i in 0..q1_transforms.len() {
                    q1_transforms[i].translate(glam::vec3( 0.1*dt, 0.0, 0.0));
                    q2_transforms[i].translate(glam::vec3(-0.1*dt, 0.0, 0.0));
                }
            }
        }

        self.quad1.instances.update(&mut graphics.context);
        self.quad2.instances.update(&mut graphics.context);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        let draw_q1 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.qbg1.id])
            .with_vertex_buffers(&self.quad1.geo_buf_ids())
            .with_index_buffer(self.quad1.idx_buf_id(), wgpu::IndexFormat::Uint16)
            .with_instances(0..self.quad1.instances.len() as u32);
        
        let draw_q2 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.qbg2.id])
            .with_vertex_buffers(&self.quad2.geo_buf_ids())
            .with_index_buffer(self.quad2.idx_buf_id(), wgpu::IndexFormat::Uint16)
            .with_instances(0..self.quad2.instances.len() as u32);

        let draw_cmd = DrawCommand::from_draws(
            RenderingState {
                output_view: frame.view.clone(),
                clear_color: Some(wgpu::Color {r: 0.39, g: 0.58, b: 0.93, a: 1.0})
            }, 
            vec![draw_q1, draw_q2]
        );

        self.executor.add_command(draw_cmd);
        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}