use std::num::NonZero;

use crate::graphics::{BufferId, SamplerId, TextureId};

/// The target ID for a bind group entry
#[derive(Clone, Debug)]
pub enum BindingTarget {
    Buffer(BufferId),
    Texture(TextureId),
    Sampler(SamplerId),
}

/// Represents gpu resources that can be used in a bind group
pub trait Bindable {
    /// Get the resource binding as it's entire wgpu binding type
    fn as_binding(&self) -> wgpu::BindingType;
    /// Get the id of the target resource the binding refers to
    fn target(&self) -> BindingTarget;
    /// Get the shader stage visibility of the resource binding
    fn visibility(&self) -> wgpu::ShaderStages;
}

#[derive(Clone, Debug)]
pub struct GroupEntry {
    pub target: BindingTarget,
    pub slot: u32
}

#[derive(Clone, Debug)]
pub struct BindGroup {
    pub label: String,
    pub layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
    pub bindings: Vec<GroupEntry>,
}

impl BindGroup {
    pub fn new() -> Self {
        Self {
            label: "bind_group".to_string(),
            layout_entries: Vec::new(),
            bindings: Vec::new(),
        }
    }

    /// Set the label for gpu profiling of the resultant bind group
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add an entry into the bind group
    pub fn with_entry(mut self, entry: impl Bindable) -> Self {
        let slot = self.bindings.len() as u32;
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: slot,
            visibility: entry.visibility(),
            ty: entry.as_binding(),
            count: None,
        });

        self.bindings.push(GroupEntry { 
            target: entry.target(),
            slot
        });

        self
    }
}


/// Options for configurating a storage texture binding
pub struct TextureTypeStorage {
    pub access: wgpu::StorageTextureAccess, 
    pub fmt: wgpu::TextureFormat
}

impl Default for TextureTypeStorage {
    fn default() -> Self {
        Self {
            access: wgpu::StorageTextureAccess::WriteOnly,
            fmt: wgpu::TextureFormat::Rgba16Float
        }
    }
}

/// options for configuring a sampled texture binding
pub struct TextureTypeSampled {
    pub filterable: bool, 
    pub multisampled: bool,
}

impl Default for TextureTypeSampled {
    fn default() -> Self {
        Self { filterable: false, multisampled: false }
    }
}

/// Represents a texture binding in a bind group
pub struct TextureBinding {
    tex_id: TextureId,
    ty: wgpu::BindingType,
    visibility: wgpu::ShaderStages,
    view_dimensions: wgpu::TextureViewDimension
}

impl TextureBinding {
    pub fn new(target: TextureId, ty: wgpu::BindingType) -> Self {
        Self {
            tex_id: target,
            ty,
            visibility: wgpu::ShaderStages::FRAGMENT,
            view_dimensions: wgpu::TextureViewDimension::D2
        }
    }

    /// Create a new storage texture binding
    pub fn as_storage(target: TextureId, options: TextureTypeStorage) -> Self {
        let ty = wgpu::BindingType::StorageTexture { 
            access: options.access,
            format: options.fmt, 
            view_dimension: wgpu::TextureViewDimension::D2
        };

        TextureBinding::new(target, ty)
    }

    /// Create a new sampled texture binding
    pub fn as_sampled(target: TextureId, options: TextureTypeSampled) -> Self {
        let ty = wgpu::BindingType::Texture { 
            sample_type: wgpu::TextureSampleType::Float { filterable: options.filterable }, 
            view_dimension: wgpu::TextureViewDimension::D2, 
            multisampled: options.multisampled
        };

        TextureBinding::new(target, ty)
    }

    /// Set the view dimensions for the texture binding
    pub fn with_view_dimensions(mut self, view_dimensions: wgpu::TextureViewDimension) -> Self {
        self.view_dimensions = view_dimensions;
        self
    }

    /// Set the shader stage visibility for the texture binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Bindable for TextureBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        self.ty
    }

    fn target(&self) -> BindingTarget {
        BindingTarget::Texture(self.tex_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}

/// Represents a buffer binding and entry in a bind group
pub struct BufferBinding {
    buf_id: BufferId,
    ty: wgpu::BufferBindingType,
    visibility: wgpu::ShaderStages,
    has_dyn_offset: bool,
    min_binding_size: Option<NonZero<u64>>
}

impl BufferBinding {
    pub fn new(target: BufferId, ty: wgpu::BufferBindingType) -> Self {
        Self {
            buf_id: target,
            ty,
            visibility: wgpu::ShaderStages::FRAGMENT,
            has_dyn_offset: false,
            min_binding_size: None,
        }
    }

    /// Create a new storage buffer binding
    pub fn as_storage(target: BufferId, read_only: bool) -> Self {
        let ty = wgpu::BufferBindingType::Storage { read_only };
        BufferBinding::new(target, ty)
    }

    /// Create a new uniform buffer binding
    pub fn as_uniform(target: BufferId) -> Self {
        let ty = wgpu::BufferBindingType::Uniform;
        BufferBinding::new(target, ty)
    }

    /// Set the shader stage visibility for the buffer binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the binding to have a dynamic offset
    pub fn with_dynamic_offset(mut self) -> Self {
        self.has_dyn_offset = true;
        self
    }

    /// Set the minimum buffer size for the binding. Must be greater than 0.
    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_binding_size = Some(NonZero::new(size)
            .expect("[Buffer Binding] Expected minimum binding size to be a non zero unsigned number."));
        self
    }
}

impl Bindable for BufferBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: self.ty,
            has_dynamic_offset: self.has_dyn_offset,
            min_binding_size: self.min_binding_size
        }
    }

    fn target(&self) -> BindingTarget {
        BindingTarget::Buffer(self.buf_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}


/// Represents the binding of a Sampler in a bind group
pub struct SamplerBinding {
    sampler_id: SamplerId,
    visibility: wgpu::ShaderStages,
    binding_type: wgpu::SamplerBindingType,
}

impl SamplerBinding {
    pub fn new(target: SamplerId) -> Self {
        Self {
            sampler_id: target,
            visibility: wgpu::ShaderStages::FRAGMENT,
            binding_type: wgpu::SamplerBindingType::NonFiltering
        }
    }

    /// Set the shader stage visibility for the sampler binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the binding type of the sampler binding
    pub fn with_binding_type(mut self, ty: wgpu::SamplerBindingType) -> Self{
        self.binding_type = ty;
        self
    }
}

impl Bindable for SamplerBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(self.binding_type)
    }

    fn target(&self) -> super::BindingTarget {
        BindingTarget::Sampler(self.sampler_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}