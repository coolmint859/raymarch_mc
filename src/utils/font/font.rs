use std::{collections::HashMap, format};

use glam::{Quat, Vec3};
use uuid::Uuid;

use crate::{graphics::{BindGroup, BindGroupIdPair, BufferBinding, BufferId, DrawCommand, GpuContext, IndexedDraw, Pipeline, PipelineId, Sampler, TexDimensions, Texture}, utils::{BufferInit, CharacterGlyph, GeometryData, IndexBuffer, MatInit, Material, SamplerComponent, TextOptions, TextureComponent, Transform, TransformAttribute, Vec2Attribute, Vec3Attribute, Vec4Attribute}};

/// The maximum number of renderable characters per font
const CHAR_LIMIT: usize = 500;

/// A handle to a font read through a specific font reader. 
/// 
/// The reader determines the way the atlas is constructed, so it also provides the best shader to interpret the atlas
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontId(pub Uuid);

impl FontId {
    pub const UNINIT: FontId = FontId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// Geometry for a quad
#[derive(Debug)]
pub struct Quad {
    pub vertices: GeometryData<BufferInit>,
    pub indices: IndexBuffer,
}

impl Quad {
    pub fn new() -> Self {
        Self {
            vertices: GeometryData::placeholder(),
            indices: IndexBuffer::new_u16(vec![0, 1, 2, 2, 3, 0]).with_label("Quad Index Buffer"),
        }
    }

    /// Initialize the quad with the gpu, creating the vertex data that it represents. 
    pub fn init(&mut self, context: &mut GpuContext) {
        let vert_positions: Vec<[f32; 3]> = vec![
            [ 0.5,  0.5, 0.0], [-0.5,  0.5, 0.0], [-0.5, -0.5, 0.0], [ 0.5, -0.5, 0.0],
        ];

        let vert_uvs: Vec<[f32; 2]> = vec![
            [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]
        ];
        
        self.vertices = GeometryData::as_vertex_group(0)
            .with_label("Quad Vertices")
            .with_attribute(Vec3Attribute("positions"), vert_positions)
            .with_attribute(Vec2Attribute("uvs"), vert_uvs)
            .init(context, 4);

        self.indices.init(context);
    }

    /// Get the next available vertex location
    pub fn next_vertex_location(&self) -> u32 { 
        self.vertices.attr_count()
    }
}

/// Represents the low level gpu resources associated with a specific font
#[derive(Debug)]
pub(crate) struct FontPrimitive {
    /// the quad that font characters will be rendered on
    pub(crate) quad: Quad,
    /// the individual characters staged to be rendered using this font
    pub(crate) instances: GeometryData<BufferInit>,
    /// global bind group
    pub(crate) global_bg: BindGroupIdPair,
    /// font material
    pub(crate) material: Material<MatInit>,
    /// the id to the font pipeline
    pub(crate) pipeline: PipelineId,
}

impl FontPrimitive {
    /// create the gpu assets that this font uses
    pub fn init(
        cam_buf_id: &BufferId,
        atlas: (Vec<u8>, u32),
        context: &mut GpuContext,
        shader_path: &str
    ) -> Self {
        let (atlas_data, atlas_size) = atlas;

        let mut quad = Quad::new();
        quad.init(context);

        let start_loc = quad.vertices.attr_count();
        let instances = GeometryData::as_instance_group(start_loc)
            .with_label("Font Quad Instances")
            .with_attribute(TransformAttribute("transform"), Vec::<Transform>::new())
            .with_attribute(Vec4Attribute("bounds"), Vec::<glam::Vec4>::new())            
            .init(context, CHAR_LIMIT);

        let atlas_dim = TexDimensions::size_2d(atlas_size, atlas_size);
        let global_bg = context.request_bind_group(
            BindGroup::new()
                .with_label(&format!("Font Bind Group shader@{:?}", shader_path))
                .with_entry(BufferBinding::as_uniform(*cam_buf_id))
        );

        let atlas = Texture::procedural(atlas_data, atlas_dim)
            .with_format(wgpu::TextureFormat::R8Unorm)
            .writable();

        let material = Material::new()
            .with_label("Font Material")
            .with_component("atlas", TextureComponent::new(atlas))
            .with_component("samp", SamplerComponent::new(Sampler::nearest()))
            .init(context);

        let pipeline = context.request_pipeline(
            Pipeline::as_render()
                .with_label(&format!("Font Render Pipeline shader@{:?}", shader_path))
                .with_bg_layouts(&[global_bg.layout_id, material.layout_id()])
                .with_vertex_layout(quad.vertices.layout().clone())
                .with_vertex_layout(instances.layout().clone())
                .with_shader(shader_path)
        );

        Self {
            quad,
            instances,
            global_bg,
            material,
            pipeline
        }
    }

    /// Get the vertex/instance buffer ids of this `FontPrimitive`
    pub fn geo_buf_ids(&self) -> [BufferId; 2] {
        [*self.quad.vertices.buf_id(), *self.instances.buf_id()]
    }

    /// Get the index buffer id of this `FontPrimitive`
    pub fn idx_buf_id(&self) -> BufferId {
        *self.quad.indices.id()
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

        let char_instances = self.primitive.instances.borrow_mut();

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
        self.primitive.instances.update(context);
    }

    /// Issue a draw call of this font to a `DrawCommand`
    pub fn render(&self, draw_cmd: &mut DrawCommand) {
        // println!("font instance count: {}", self.primitive.instances.elem_count());

        let draw_call = IndexedDraw::new(self.primitive.pipeline, 0..6)
            .with_bind_groups(&[self.primitive.global_bg.id, self.primitive.material.bg_id()])
            .with_vertex_buffers(&self.primitive.geo_buf_ids())
            .with_index_buffer(self.primitive.idx_buf_id(), wgpu::IndexFormat::Uint16)
            .with_instances(0..self.primitive.instances.elem_count() as u32);
        
        draw_cmd.add_draw(draw_call);
    }
}