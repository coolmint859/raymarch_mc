use std::{collections::HashMap, format};

use glam::{Quat, Vec3};

use crate::{graphics::{BindGroup, BufferBinding, DrawCommand, GpuContext, IndexedDraw, NamedBindGroup, Pipeline, PipelineId, Sampler, SamplerBinding, SamplerId, TexDimensions, Texture, TextureBinding, TextureId, TextureTypeSampled}, utils::{Camera, CameraSpace, CharacterGlyph, FontPipeline, FontQuad, TextOptions, Transform}};

/// The maximum number of renderable characters per font
const CHAR_LIMIT: u64 = 500;

/// A handle to a font read through a specific font reader. 
/// 
/// The reader determines the way the atlas is constructed, so it also provides the best shader to interpret the atlas
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontId {
    pub path: String,
    pub pip: FontPipeline,
}

impl FontId {
    /// Create an uninitialized `FontId`. This is useful for architectures with lazy initialization.
    /// 
    /// Note that the returned `FontId` doesn't refer to any actual font.
    pub fn uninit() -> Self {
        Self {
            path: "no_path".to_string(),
            pip: FontPipeline { 
                id: PipelineId("no id"),
                shader: "no_shader".to_string()
            }
        }
    }
}

/// Represents the low level gpu resources associated with a specific font
#[derive(Debug)]
pub(crate) struct FontPrimitive {
    /// the id of this specific font (path and pipeline id)
    pub(crate) id: FontId,
    /// the quad that font characters will be rendered on
    pub(crate) quad: FontQuad,
    /// the id of the atlas texture
    pub(crate) atlas_tex_id: TextureId,
    /// the id of the atlas sampler
    pub(crate) atlas_samp_id: SamplerId,
    /// the ids of the bind group / layout
    pub(crate) bg: NamedBindGroup,
}

impl FontPrimitive {
    pub fn new(id: FontId) -> Self {
        let path = id.path.clone();
        Self {
            id,
            quad: FontQuad::new(),
            atlas_tex_id: TextureId(Box::leak(Box::new(format!("{}@font_atlas", path)))),
            atlas_samp_id: SamplerId(Box::leak(Box::new(format!("{}@atlas_sampler", path)))),
            bg: NamedBindGroup::new(Box::leak(Box::new(format!("{}@bind_group", path)))),
        }
    }

    /// create the gpu assets that this font uses
    pub fn init<S: CameraSpace>(
        &mut self,
        camera: &Camera<S>,
        atlas: (Vec<u8>, u32),
        context: &mut GpuContext
    ) {
        let (atlas_data, atlas_size) = atlas;

        self.quad.init(context, CHAR_LIMIT);

        let atlas_dim = TexDimensions::size_2d(atlas_size, atlas_size);
        context.request_texture(
            &self.atlas_tex_id,
            Texture::procedural(atlas_data, atlas_dim)
                .with_label(&format!("Font Atlas Texture @{:?}", self.id))
                .with_format(wgpu::TextureFormat::R8Unorm)
                .writable()
        );

        context.request_sampler(
            &self.atlas_samp_id, 
            Sampler::linear()
        );

        context.request_bind_group(
            &self.bg.id, &self.bg.layout_id, 
            BindGroup::new()
                .with_label(&format!("Font Bind Group @{:?}", self.id))
                .with_entry(BufferBinding::as_uniform(*camera.buf_id()))
                .with_entry(TextureBinding::as_sampled(self.atlas_tex_id, TextureTypeSampled { filterable: true, multisampled: false }))
                .with_entry(SamplerBinding::new(self.atlas_samp_id).with_binding_type(wgpu::SamplerBindingType::Filtering))
        );

        context.request_pipeline(
            &self.id.pip.id,
            Pipeline::as_render()
                .with_label(&format!("Font Render Pipeline @{:?}", self.id.pip.id))
                .with_bg_layouts(&[self.bg.layout_id])
                .with_vertex_layout(self.quad.vertices.layout().clone())
                .with_vertex_layout(self.quad.instances.layout().clone())
                .with_shader(&self.id.pip.shader)
        );
    }
}

/// The complete information about a font, including the glyph map and rendering primitive
pub(crate) struct Font {
    pub primitive: FontPrimitive,
    pub glyph_map: HashMap<char, CharacterGlyph>,
    pub line_height: f32,
    pub scale: f32,
}

impl Font {
    /// Stage character instances with this `Font`
    pub fn stage_text(&mut self, text: &str, options: TextOptions) {
        let size = options.height / (self.line_height * self.scale);

        let char_instances = self.primitive.quad.instances.borrow_mut();

        if let (Some(mut transforms), Some(mut bounds)) = (
            char_instances.get_attribute_mut::<Transform>("transform"),
            char_instances.get_attribute_mut::<glam::Vec4>("bounds")
        ) {
            transforms.clear();
            bounds.clear();

            let mut cursor = Vec3::ZERO;
            for character in text.chars() {
                if character == '\n' {
                    cursor.x = 0.0;
                    cursor.y -= self.line_height * size;

                    continue;
                }

                if let Some(glyph) = self.glyph_map.get(&character) {
                    if character == ' ' {
                        cursor.x += glyph.advance * size;
                        continue;
                    }

                    let x_scale = glyph.plane_bounds.z * size;
                    let y_scale = glyph.plane_bounds.w * size;

                    let x_pos = cursor.x + (glyph.plane_bounds.x * size) + (x_scale * 0.5);
                    let y_pos = cursor.y + (glyph.plane_bounds.y * size) + (y_scale * 0.5);

                    let local_transform = Transform::new(
                        Vec3::new(x_pos, y_pos, cursor.z), 
                        Quat::IDENTITY, 
                        Vec3::new(x_scale, y_scale, 1.0)
                    );
                    let world_transform = options.transform.mult(&local_transform);

                    transforms.push(world_transform);
                    bounds.push(glyph.uv_bounds);

                    cursor.x += glyph.advance * size;
                }
            }
        }
    }

    pub fn update(&mut self, context: &mut GpuContext) {
        self.primitive.quad.update_instances(context);
    }

    /// Issue a draw call of this font to a `DrawCommand`
    pub fn render(&self, draw_cmd: &mut DrawCommand) {
        let draw_call = IndexedDraw::new(self.primitive.id.pip.id, 0..6)
            .with_bind_groups(&[self.primitive.bg.id])
            .with_vertex_buffers(&self.primitive.quad.geo_buf_ids())
            .with_index_buffer(self.primitive.quad.idx_buf_id, wgpu::IndexFormat::Uint16)
            .with_instances(0..self.primitive.quad.instances.len() as u32);
        
        draw_cmd.add_draw(draw_call);
    }
}