use crate::{graphics::*, utils::PerspectiveCamera};

/// Global resource ids
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GlobalIds {
    pub cam_id: BufferId,
    /// environment buffer
    pub env_id: BufferId,
    /// the output of the ray march shader
    pub rm_tex_id: TextureId,
    /// texture a of the taa shader (history even frames, current odd frames)
    pub taa_tex_a_id: TextureId,
    /// texture b of the taa shader (current even frames, history odd frames)
    pub taa_tex_b_id: TextureId,
}

pub struct GlobalResources {
    pub ids: GlobalIds,
}

impl GlobalResources {
    pub fn new() -> Self {
        let ids = GlobalIds {
            cam_id: BufferId("main_camera"),
            env_id: BufferId("environment"),

            rm_tex_id: TextureId("ray_march_texture"),
            taa_tex_a_id: TextureId("taa_texture_a"),
            taa_tex_b_id: TextureId("taa_texture_b"),
        };

        Self { ids }
    }

    pub fn init(&self, graphics: &mut Graphics, camera: &PerspectiveCamera) {
        let (cw, ch) = graphics.canvas.dimensions();

        let camera_buffer = Buffer::as_uniform()
            .with_label("Camera Buffer")
            .with_struct_data(camera.to_uniform(graphics.canvas.frame_count()))
            .writable();
        graphics.context.request_buffer(&self.ids.cam_id, camera_buffer);

        let raymarch_texture = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("Raymarch Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable()
            .writable();
        graphics.context.request_texture(&self.ids.rm_tex_id, raymarch_texture);

        let taa_texture_a = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture A")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.ids.taa_tex_a_id, taa_texture_a);

        let taa_texture_b = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture B")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.ids.taa_tex_b_id, taa_texture_b);
    }

    pub fn on_resize(&mut self, graphics: &mut Graphics) {
        let (cw, ch) = graphics.canvas.dimensions();
        
        graphics.context.remove_texture(&self.ids.rm_tex_id);
        let raymarch_texture = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("Raymarch Texture")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.ids.rm_tex_id, raymarch_texture);

        graphics.context.remove_texture(&self.ids.taa_tex_a_id);
        let taa_texture_a = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture A")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.ids.taa_tex_a_id, taa_texture_a);

        graphics.context.remove_texture(&self.ids.taa_tex_b_id);
        let taa_texture_b = Texture::computed(TexDimensions::size_2d(cw, ch))
            .with_label("TAA Texture B")
            .with_format(wgpu::TextureFormat::Rgba16Float)
            .storage_bindable();
        graphics.context.request_texture(&self.ids.taa_tex_b_id, taa_texture_b);
    }
}