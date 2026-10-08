/// Blueprint for constructing vertex buffer layouts. Does automatic offset and
/// and location calculations.
#[derive(Clone, Debug, Hash)]
pub struct VertexBufferLayout {
    attributes: Vec<wgpu::VertexAttribute>,
    step_mode: wgpu::VertexStepMode,
    curr_loc: u32,
    curr_offset: u64,
}

impl VertexBufferLayout {
    fn new(step_mode: wgpu::VertexStepMode, start_loc: u32) -> Self {
        Self {
            attributes: Vec::new(),
            curr_offset: 0,
            step_mode,
            curr_loc: start_loc
        }
    }

    /// Create a vertex buffer layout with the vertex step mode
    pub fn as_vertex_step(start_loc: u32) -> Self {
        VertexBufferLayout::new(wgpu::VertexStepMode::Vertex, start_loc)
    }

    /// Create a vertex buffer layout with the instance step mode
    pub fn as_instance_step(start_loc: u32) -> Self {
        VertexBufferLayout::new(wgpu::VertexStepMode::Instance, start_loc)
    }

    /// Add an attribute to the buffer layout
    pub fn with_attribute(mut self, format: wgpu::VertexFormat) -> Self {
        self.add_attribute(format);
        self
    }

    /// Add an attribute to the buffer layout
    pub fn add_attribute(&mut self, format: wgpu::VertexFormat) {
        self.attributes.push(wgpu::VertexAttribute {
            format,
            offset: self.curr_offset,
            shader_location: self.curr_loc
        });
        self.curr_loc += 1;
        self.curr_offset += format.size();
    }

    /// Convert the layout into it's wgpu equivelant for use in a pipeline
    pub(crate) fn as_wgpu_layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.curr_offset,
            step_mode: self.step_mode,
            attributes: &self.attributes
        }
    }

    /// Get the stride in bytes of this vertex layout
    pub fn stride(&self) -> u64 { self.curr_offset }

    /// Get the number of attributes defined with this vertex layout
    pub fn count(&self) -> u32 { self.curr_loc }
}