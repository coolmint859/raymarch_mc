/// Represents attributes in a vertex/instance buffer
pub trait VertexAttribute {
    /// Get the name of this attribute
    fn name(&self) -> impl Into<String>;

    /// The number of slots this attribute requires (default is 1).
    fn count(&self) -> u32 { 1 }

    /// The format of this attribute.
    fn format(&self) -> wgpu::VertexFormat;
}

/// A float vertex attribute. `f32` in shader
pub struct FloatAttribute(pub &'static str);
impl VertexAttribute for FloatAttribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32 }
}

/// A 2d vector vertex attribute. `vec2<f32>` in shader
pub struct Vec2Attribute(pub &'static str);
impl VertexAttribute for Vec2Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }
}

/// A 3d vector vertex attribute. `vec3<f32>` in shader
pub struct Vec3Attribute(pub &'static str);
impl VertexAttribute for Vec3Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }
}

/// A 4d vector vertex attribute. `vec4<f32>` in shader
pub struct Vec4Attribute(pub &'static str);
impl VertexAttribute for Vec4Attribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

/// Four 4d-vectors, each `vec4<f32>` in shader 
pub struct TransformAttribute(pub &'static str);
impl VertexAttribute for TransformAttribute {
    fn name(&self) -> impl Into<String> { self.0 }

    fn count(&self) -> u32 { 4 }

    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}
