pub trait VertexAttribute {
    /// The format of the attribute
    fn format(&self) -> wgpu::VertexFormat;
    /// The number of locations the attribute requires (default is 1)
    fn count(&self) -> u32 { 1 }
}

/// an f32 (scalar) vertex attribute. f32 in shader
pub struct ScalarAttribute;
impl VertexAttribute for ScalarAttribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32 }
}

/// a 2-value vertex attribute. vec2f in shader
pub struct Vec2Attribute;
impl VertexAttribute for Vec2Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }
}

/// a 3-value vertex attribute. vec3f in shader
pub struct Vec3Attribute;
impl VertexAttribute for Vec3Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }
}

/// a 4-value vertex attribute. vec4f in shader
pub struct Vec4Attribute;
impl VertexAttribute for Vec4Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

/// A 16-value matrix vertex attribute. 4 vec4f in shader
pub struct Mat4Attribute;
impl VertexAttribute for Mat4Attribute {
    fn count(&self) -> u32 { 4 }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

/// Blueprint for constructing vertex buffer layouts. Does automatic offset and
/// and location calculations.
#[derive(Clone, Debug)]
pub struct VertexBufferLayout {
    pub attributes: Vec<wgpu::VertexAttribute>,
    pub step_mode: wgpu::VertexStepMode,
    pub curr_loc: u32,
    
    curr_offset: u64,
}

impl VertexBufferLayout {
    pub fn new(step_mode: wgpu::VertexStepMode) -> Self {
        Self {
            attributes: Vec::new(),
            curr_offset: 0,
            step_mode,
            curr_loc: 0
        }
    }

    /// Create a vertex buffer layout with the vertex step mode
    pub fn as_vertex_step() -> Self {
        VertexBufferLayout::new(wgpu::VertexStepMode::Vertex)
    }

    /// Create a vertex buffer layout with the instance step mode
    pub fn as_instance_step() -> Self {
        VertexBufferLayout::new(wgpu::VertexStepMode::Instance)
    }

    /// Set the starting location for the attributes. This should be called prior to any attribute additions.
    pub fn with_start_loc(mut self, start_loc: u32) -> Self {
        self.curr_loc = start_loc;
        self
    }

    /// Add an attribute to the buffer layout
    pub fn with_attribute(mut self, attr: impl VertexAttribute) -> Self {
        for _ in 0..attr.count() {
            self.attributes.push(wgpu::VertexAttribute {
                format: attr.format(),
                offset: self.curr_offset,
                shader_location: self.curr_loc
            });
            self.curr_loc += 1;
            self.curr_offset += attr.format().size()
        }

        self
    }

    /// Convert the layout into it's wgpu equivelant for use in a pipeline
    pub fn as_wgpu_layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.curr_offset,
            step_mode: self.step_mode,
            attributes: &self.attributes
        }
    }
}