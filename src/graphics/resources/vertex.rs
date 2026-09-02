pub trait VertexAttribute {
    /// The format of the attribute
    fn format(&self) -> wgpu::VertexFormat;
    /// The number of locations the attribute requires (default is 1)
    fn count(&self) -> u32 { 1 }
}

pub struct ScalarAttribute;
impl VertexAttribute for ScalarAttribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32 }
}

pub struct Vec2Attribute;
impl VertexAttribute for Vec2Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }
}

pub struct Vec3Attribute;
impl VertexAttribute for Vec3Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }
}

pub struct Vec4Attribute;
impl VertexAttribute for Vec4Attribute {
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

pub struct TransformAttribute;
impl VertexAttribute for TransformAttribute {
    fn count(&self) -> u32 { 4 }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
}

#[derive(Clone, Debug)]
pub struct VertexBufferLayout {
    pub label: String,
    pub attributes: Vec<wgpu::VertexAttribute>,
    pub step_mode: wgpu::VertexStepMode,
    pub curr_loc: u32,
    
    curr_offset: u64,
}

impl VertexBufferLayout {
    pub fn new(step_mode: wgpu::VertexStepMode) -> Self {
        Self {
            label: "vertex_buffer_layout".to_string(),
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

    /// Set the label for gpu profiling of the resultant buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

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

    /// convert the layout into it's wgpu equivelant for use in a pipeline
    pub fn desc(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.curr_offset,
            step_mode: self.step_mode,
            attributes: &self.attributes
        }
    }
}