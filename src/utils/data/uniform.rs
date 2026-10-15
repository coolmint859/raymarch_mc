use crate::graphics::{Buffer, BufferId, BufferSource, GpuContext, Serializable, StructuredUpdate};

/// Stores uniform data for a buffer used by a `GpuContext`.
pub struct UniformBuffer<T> {
    uniform: T,
    label: String,
    id: BufferId,
    is_dirty: bool,
}

impl<T> UniformBuffer<T> 
where T: Serializable + Clone + 'static
{
    /// Create a new UniformBuffer. This does not create a corresponding gpu buffer. For that, call `init()`
    pub fn new(uniform: T) -> Self {
        Self {
            uniform,
            label: "uniform_buffer".to_string(),
            id: BufferId::UNINIT,
            is_dirty: true,
        }
    }
    
    /// Set the label for gpu profiling of this buffer
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Initialize this buffer with a `GpuContext`. 
    pub fn init(mut self, context: &mut GpuContext) -> Self {
        self.id = context.request_buffer(
            Buffer::as_uniform().with_label(&self.label).writable(),
            BufferSource::from_pod(self.uniform.clone())
        );
        self.is_dirty = false;

        self
    }

    /// Returns a reference to the uniform data struct stored in this buffer.
    pub fn get_uniform(&self) -> &T {
        &self.uniform
    }

    /// Returns a mutable reference to the uniform data struct stored in this buffer.
    pub fn get_uniform_mut(&mut self) -> &mut T {
        self.is_dirty = true;

        &mut self.uniform
    }

    /// Set the uniform data
    pub fn set_uniform(&mut self, uniform: T) {
        self.uniform = uniform;
        self.is_dirty = true;
    }

    /// Update the corresponding buffer on the gpu. This should be called whenever the 'T' data is modified.
    pub fn update(&mut self, context: &mut GpuContext) {
        if !self.is_dirty { return; }

        // the buffer was created with initial data, so no sizing error is possible here.
        let _ = context.update_buffer(&self.id, StructuredUpdate {
            data: &self.uniform,
            offset: 0,
        });

        self.is_dirty = false;
    }

    /// Update the buffer with new uniform data
    pub fn update_with(&mut self, context: &mut GpuContext, uniform: T) {
        self.set_uniform(uniform);
        self.update(context);
    }

    /// Returns the id to this buffer.
    pub fn id(&self) -> &BufferId {
        &self.id
    }

    /// Unintialize this buffer from a `GpuContext`.
    /// 
    /// Note that even if the buffer didn't exist in the provided context, this method overwrites the buffer's id to the unitialized value.
    /// 
    /// To reinitialize the buffer again, call `init()`
    pub fn uninit(&mut self, context: &mut GpuContext) {
        context.remove_buffer(&self.id);
        self.id = BufferId::UNINIT;
    }
}