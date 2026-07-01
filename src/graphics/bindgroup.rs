use std::{ops::Deref, sync::Arc};

use crate::graphics::{BufferHandle, ResourceBuilder, WgpuResource};

pub enum ResourceHandle {
    Buffer(BufferHandle)
    // Texture(TextureHandle),
    // Sampler(SamplerHandle),
}

impl WgpuResource for ResourceHandle {
    fn binding_type(&self) -> wgpu::BindingType {
        match self {
            ResourceHandle::Buffer(buffer) => buffer.binding_type()
        }
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        match self {
            ResourceHandle::Buffer(buffer) => buffer.visibility()
        }
    }

    fn as_binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            ResourceHandle::Buffer(buffer) => buffer.as_binding()
        }
    }
}

// This allows users to pass in the concrete handle type and have it convert into the
// corresponding resource handle variant automatically.
impl From<BufferHandle> for ResourceHandle {
    fn from(handle: BufferHandle) -> Self {
        ResourceHandle::Buffer(handle)
    }
}

// A lightweight handle to a bind group and its associated layout
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindGroupHandle {
    pub layout: Arc<wgpu::BindGroupLayout>,
    pub bind_group: Arc<wgpu::BindGroup>
}

impl Deref for BindGroupHandle {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &*self.bind_group
    }
}

struct GroupEntry {
    pub bindslot: u32,
    pub resource: ResourceHandle,
}

pub struct BindGroupBuilder {
    label: String,
    layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
    bindings: Vec<GroupEntry>,

    curr_slot: u32,
}

impl BindGroupBuilder {
    pub fn new() -> Self {
        Self {
            label: "bind_group".to_string(),
            layout_entries: Vec::new(),
            bindings: Vec::new(),
            curr_slot: 0
        }
    }

    /// Set the label for gpu profiling of the resultant bind group
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add a resource to the bind group
    pub fn with_resource(mut self, visibility: wgpu::ShaderStages, resource: ResourceHandle) -> Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.curr_slot,
            visibility,
            ty: resource.binding_type(),
            count: None
        });

        self.bindings.push(GroupEntry {
            bindslot: self.curr_slot,
            resource,
        });

        self
    }

    /// Convert the bindings provided to the builder into BindGroupEntries
    fn create_entries(&self) -> Vec<wgpu::BindGroupEntry<'_>> {
        let entries: Vec<wgpu::BindGroupEntry> = self.bindings.iter()
            .map(|binding| {
                wgpu::BindGroupEntry {
                    binding: binding.bindslot,
                    resource: binding.resource.as_binding(),
                }
            })
            .collect();

        entries
    }
}

impl ResourceBuilder for BindGroupBuilder {
    type Resource = BindGroupHandle;

    fn build(&self, gpu: super::GpuHandle) -> Self::Resource {
        let layout = Arc::new(gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some(&format!("Layout: {}", self.label)),
            entries: &self.layout_entries
        }));

        let bind_group = Arc::new(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&self.label),
            layout: &layout,
            entries: &self.create_entries(),
        }));

        BindGroupHandle { layout, bind_group }
    }
}