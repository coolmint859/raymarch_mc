use winit::event::MouseButton;

use crate::{Graphics, InputEvent, game::{PlayerMouseAction, Screen, ScreenTransition}, graphics::{BindGroup, BindGroupId, BufferBinding, BufferId, DrawCommand, GpuContext, IndexedDraw, LayoutId, MultiBufferExecutor, NamedBindGroup, OnDisk, Pipeline, PipelineId, RenderingState, SequentialExecutor, VertexBufferLayout}, utils::{Camera, FontReader, GeoInit, GeometryData, MatInit, Material, MouseHandler, SamplerComponent, ScreenSpace, TextOptions, FontManager, TextureComponent, Transform, TransformAttribute, font::font::{FontId, Quad}}};

pub struct QuadPrimitive {
    pub quad: Quad,
    pub instances: GeometryData<GeoInit>,
    pub material: Material<MatInit>,
}

impl QuadPrimitive {
    pub fn uninit() -> Self {
        Self {
            quad: Quad::new(),
            instances: GeometryData::placeholder(),
            material: Material::placeholder(),
        }
    }
    
    pub fn update(&mut self, context: &mut GpuContext) {
        self.instances.update(context);
        self.material.update(context);
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

    pub fn mat_bg_id(&self) -> BindGroupId {
        self.material.bg_id()
    }

    pub fn mat_layout_id(&self) -> LayoutId {
        self.material.layout_id()
    }
}

pub struct QuadTest {
    mouse: MouseHandler<PlayerMouseAction>,
    executor: MultiBufferExecutor,

    text_renderer: FontManager,
    arial: FontId,
    mono: FontId,

    camera: Camera<ScreenSpace>,
    cam_bg: NamedBindGroup,

    quad1: QuadPrimitive,
    quad2: QuadPrimitive,

    qpip: PipelineId,
}

impl QuadTest {
    pub fn new() -> Self {
        Self { 
            mouse: MouseHandler::new(),
            executor: MultiBufferExecutor::new(),

            text_renderer: FontManager::new(),
            arial: FontId::uninit(),
            mono: FontId::uninit(),

            camera: Camera::new(ScreenSpace),
            cam_bg: NamedBindGroup::new("camera_bg"),

            quad1: QuadPrimitive::uninit(),
            quad2: QuadPrimitive::uninit(),

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
        self.camera.init(graphics);

        let mut q1_geometry = Quad::new();
        q1_geometry.init(&mut graphics.context);

        let q1_transforms = vec![
            Transform::from_position(glam::vec3(0.60, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(0.60, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(0.25, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(0.25, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1))
        ];
        let q1_instances = GeometryData::as_instance_group(q1_geometry.next_vertex_location())
            .with_label("Quad1 Instances")
            .with_attribute(TransformAttribute("transform"), q1_transforms)
            .init(&mut graphics.context, 4);

        let q1_material = Material::new()
            .with_label("Quad1 Material")
            .with_component("blue_devils", TextureComponent::on_disk(
                OnDisk { path: "./assets/BlueDevilsLogo.png".to_string() }
            ))
            .with_component("sampler", SamplerComponent::nearest())
            .init(&mut graphics.context);

        self.quad1 = QuadPrimitive { 
            quad: q1_geometry, 
            instances: q1_instances, 
            material: q1_material 
        };

        let mut q2_geometry = Quad::new();
        q2_geometry.init(&mut graphics.context);

        let q2_transforms = vec![
            Transform::from_position(glam::vec3(1.20, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(1.20, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(1.55, 0.75, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1)),
            Transform::from_position(glam::vec3(1.55, 0.25, 0.0)).with_scale(glam::vec3(0.1, 0.1, 0.1))
        ];
        let q2_instances = GeometryData::as_instance_group(q2_geometry.next_vertex_location())
            .with_label("quad2 Instances")
            .with_attribute(TransformAttribute("transform"), q2_transforms)
            .init(&mut graphics.context, 4);

        let q2_material = Material::new()
            .with_label("Quad2 Material")
            .with_component("vanguard", TextureComponent::on_disk(
                OnDisk { path: "./assets/vanguard.jpg".to_string() }
            ))
            .with_component("sampler", SamplerComponent::nearest())
            .init(&mut graphics.context);

        self.quad2 = QuadPrimitive { 
            quad: q2_geometry, 
            instances: q2_instances, 
            material: q2_material 
        };

        graphics.context.request_bind_group(
            &self.cam_bg.id, 
            &self.cam_bg.layout_id, 
            BindGroup::new()
                .with_label("Camera Bind Group")
                .with_entry(BufferBinding::as_uniform(*self.camera.buf_id()))
        );

        graphics.context.request_pipeline(
            &self.qpip, 
            Pipeline::as_render()
                .with_label("Quad Pipeline")
                .with_bg_layouts(&[self.cam_bg.layout_id, self.quad1.mat_layout_id()])
                .with_vertex_layout(self.quad1.vertex_layout())
                .with_vertex_layout(self.quad1.instance_layout())
                .with_shader("./shaders/2d_draw.wgsl")
        );

        self.arial = self.text_renderer.request_font(
            "./assets/arial.ttf", 
            FontReader::as_sdf(16.0)
        );

        self.mono = self.text_renderer.request_font(
            "./assets/Monopack.ttf", 
            FontReader::as_sdf(16.0)
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

    fn update(&mut self, graphics: &mut Graphics, dt: f32) {
        let screen_height = graphics.canvas.dimensions().1 as f32;

        self.text_renderer.stage_text(
            &self.mono, 
            &format!("fps: {:.2}", 1.0 / dt), 
            TextOptions { 
                transform: Transform::from_position(glam::vec3(0.05, 0.95, 0.0)),
                height: 40.0 / screen_height
            }
        );

        self.text_renderer.stage_text(
            &self.arial, 
            "The quick brown fox jumps over the lazy dog.", 
            TextOptions { 
                transform: Transform::from_position(glam::vec3(0.05, 0.05, 0.0)),
                height: 30.0 / screen_height
            }
        );

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

        self.quad1.update(&mut graphics.context);
        self.quad2.update(&mut graphics.context);
        self.text_renderer.update(graphics);
    }

    fn render(&mut self, graphics: &mut Graphics) -> Result<(), wgpu::SurfaceError> {
        let frame = graphics.canvas.next_frame()?;

        let draw_q1 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.cam_bg.id, self.quad1.mat_bg_id()])
            .with_vertex_buffers(&self.quad1.geo_buf_ids())
            .with_index_buffer(self.quad1.idx_buf_id(), wgpu::IndexFormat::Uint16)
            .with_instances(0..self.quad1.instances.len() as u32);
        
        let draw_q2 = IndexedDraw::new(self.qpip, 0..6)
            .with_bind_groups(&[self.cam_bg.id, self.quad2.mat_bg_id()])
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

        self.text_renderer.record(
            RenderingState {
                output_view: frame.view.clone(),
                clear_color: None
            }, 
            &mut self.executor
        );

        self.executor.record_and_submit(&graphics.context);

        frame.present();

        Ok(())
    }
}