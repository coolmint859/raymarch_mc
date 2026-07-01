use crate::graphics::GpuHandle;

pub trait RenderTarget {
    /// Returns the texture view for which a Renderer should draw to
    fn get_view(&self) -> &wgpu::TextureView;

    /// Returns the texture format (e.g. Rgpa8Unorm, etc..)
    fn format(&self) -> wgpu::TextureFormat;

    /// Optional trait method for presenting to the window surface texture.
    /// 
    /// Note: This consumes self. It is recommended to call this at the very end of the target's lifecycle.
    fn present(self) where Self: Sized {}
}

/// Represents gpu resources that can be constructed through a builder
pub trait ResourceBuilder {
    type Resource;

    /// Construct the Resource type from this builder
    fn build(&self, gpu: GpuHandle) -> Self::Resource;
}

/// Represents handles to gpu resources as defined by wgpu
pub trait WgpuResource {
    /// Get the binding type that this resource fills
    fn binding_type(&self) -> wgpu::BindingType;

    /// Get the resource visibility in the shaders
    fn visibility(&self) -> wgpu::ShaderStages;

    /// Get this resource in its BindingResource form
    fn as_binding(&self) -> wgpu::BindingResource<'_>;
}