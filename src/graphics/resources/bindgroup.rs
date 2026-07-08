use std::{ops::Deref, sync::Arc};

use crate::graphics::{BufferId, GpuHandle, TextureId};

#[derive(Clone, Debug)]
pub enum BindingTarget {
    Buffer(BufferId),
    Texture(TextureId),
    // Sampler(SamplerId),
}

/// Represents resource bindings that can be used in a bind group
pub trait Bindable {
    /// Get the resource binding as it's entire wgpu binding type
    fn as_binding(&self) -> wgpu::BindingType;
    /// Get the id of the target resource the binding refers to
    fn target(&self) -> BindingTarget;
    /// Get the shader stage visibility of the resource binding
    fn visibility(&self) -> wgpu::ShaderStages;
}

/// A lightweight handle to a bind group and its associated layout
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

/// Create a new bind group from the given configuration builder and resource map
pub async fn create_bind_group(
    gpu: GpuHandle, 
    builder: BindGroup, 
    entries: Vec<wgpu::BindGroupEntry<'_>>
) -> Result<BindGroupHandle, String> {
    let layout = Arc::new(gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some(&format!("Layout: {}", builder.label)),
        entries: &builder.layout_entries
    }));

    let bind_group = Arc::new(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&builder.label),
        layout: &layout,
        entries: &entries,
    }));

    println!("[GpuContext] Created new bind group with label '{}'", builder.label);

    Ok(BindGroupHandle { 
        layout, 
        bind_group 
    })
}