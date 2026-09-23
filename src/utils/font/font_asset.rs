use std::collections::HashMap;

use crate::{graphics::{BindGroup, Buffer, BufferBinding, BufferId, GpuContext, NamedBindGroup, Pipeline, Sampler, SamplerBinding, SamplerId, Serializable, TexDimensions, Texture, TextureBinding, TextureId, TextureTypeSampled, VertexBufferLayout}, utils::{Camera, CameraSpace, CharacterGlyph, FontPipeline, font_registry::CHAR_LIMIT}};

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
pub struct CharInstance {
    pub transform: [f32; 16],
    pub bounds: [f32; 4],
}

impl CharInstance {
    /// Get the buffer layout of the instances
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::as_instance_step(2)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
            .with_attribute(wgpu::VertexFormat::Float32x4)
    }

    /// Return the length of an instance in bytes
    pub fn byte_len() -> u64 {
        return 20;
    }
}

/// A simple rectangle
pub(crate) struct Quad {
    pub vbuffer_id: BufferId,
    pub ibuffer_id: BufferId,
    vertices: [Vertex; 4],
    indices: [u16; 6]
}

impl Quad {
    pub fn new() -> Self {
        Self {
            vbuffer_id: BufferId("quad_vertices"),
            ibuffer_id: BufferId("quad_indices"),
            vertices: [
                Vertex { position: [ 0.5,  0.5, 0.1], uvs: [1.0, 0.0] },
                Vertex { position: [-0.5,  0.5, 0.1], uvs: [0.0, 0.0] },
                Vertex { position: [-0.5, -0.5, 0.1], uvs: [0.0, 1.0] },
                Vertex { position: [ 0.5, -0.5, 0.1], uvs: [1.0, 1.0] },
            ],
            indices: [0, 1, 2, 2, 3, 0]
        }
    }

    /// Request the buffers the quad represents to be created
    pub fn request_buffers(&mut self, context: &mut GpuContext) {
        context.request_buffer(
            &self.vbuffer_id,
            Buffer::as_vertex()
                .with_label("quad_vertices")
                .with_byte_data(self.vertices.to_bytes())
                .writable()
        );

        context.request_buffer(
            &self.ibuffer_id, 
            Buffer::as_index()
                .with_label("quad_indices")
                .with_byte_data(self.indices.to_bytes())
                .writable()
        );
    }
}

/// A handle to a font read through a specific font reader. 
/// 
/// The reader determines the way the atlas is constructed, so it also provides the best shader to interpret the atlas
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontId {
    pub path: String,
    pub pip: FontPipeline,
}

/// Represents the low level gpu resources associated with a specific font
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct FontAssets {
    /// the id of this specific font (path and pipeline id)
    pub(crate) font_id: FontId,
    /// the id to the instance buffer
    pub(crate) cbuffer_id: BufferId,
    /// the id of the atlas texture
    pub(crate) atlas_tex_id: TextureId,
    /// the id of the atlas sampler
    pub(crate) atlas_samp_id: SamplerId,
    /// the ids of the bind group / layout
    pub(crate) bg: NamedBindGroup,
}

impl FontAssets {
    pub fn new(font_id: FontId) -> Self {
        Self {
            font_id,
            cbuffer_id: BufferId("char_instances"),
            atlas_tex_id: TextureId("font_altas"),
            atlas_samp_id: SamplerId("font_altas"),
            bg: NamedBindGroup::new("font_bind_group"),
        }
    }

    /// create the gpu assets that this font uses
    pub fn create_assets<S: CameraSpace>(
        &self,
        camera: &Camera<S>,
        atlas: (Vec<u8>, u32),
        context: &mut GpuContext
    ) {
        let (atlas_data, atlas_size) = atlas;

        context.request_buffer(
            &self.cbuffer_id, 
            Buffer::as_vertex()
                .with_label(&format!("Font Instance Buffer @{:?}", self.font_id))
                .with_capacity(CHAR_LIMIT * CharInstance::byte_len())
                .writable()
        );

        let atlas_dim = TexDimensions::size_2d(atlas_size, atlas_size);
        context.request_texture(
            &self.atlas_tex_id,
            Texture::procedural(atlas_data, atlas_dim)
                .with_label(&format!("Font Atlas Texture @{:?}", self.font_id))
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
                .with_label(&format!("Font Bind Group @{:?}", self.font_id))
                .with_entry(BufferBinding::as_uniform(*camera.buf_id()))
                .with_entry(TextureBinding::as_sampled(self.atlas_tex_id, TextureTypeSampled { filterable: true, multisampled: false }))
                .with_entry(SamplerBinding::new(self.atlas_samp_id).with_binding_type(wgpu::SamplerBindingType::Filtering))
        );

        context.request_pipeline(
            &self.font_id.pip.id,
            Pipeline::as_render()
                .with_label(&format!("Font Render Pipeline @{:?}", self.font_id.pip.id))
                .with_bg_layouts(&[self.bg.layout_id])
                .with_vertex_layout(Vertex::layout())
                .with_vertex_layout(CharInstance::layout())
                .with_shader(&self.font_id.pip.shader)
        );
    }
}

/// The complete information about a font, including the glyph map and gpu asset ids
pub(crate) struct Font {
    pub assets: FontAssets,
    pub glyph_map: HashMap<char, CharacterGlyph>,
    pub line_height: f32,
    pub scale: f32,
}