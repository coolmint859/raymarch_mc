use std::{ops::Deref, sync::Arc};

use crate::graphics::{BufferId, BufferRole, GpuHandle, TextureId, TextureRole};

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
pub enum BindingTarget {
    Buffer(BufferId),
    Texture(TextureId),
    // Sampler(SamplerId),
}

#[derive(Clone, Debug)]
pub struct GroupEntry {
    pub target: BindingTarget,
    pub slot: u32
}

#[derive(Clone, Debug)]
pub struct BindGroupBuilder {
    pub label: String,
    pub layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
    pub bindings: Vec<GroupEntry>,

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

    /// Add a buffer to the bind group
    pub fn with_buffer(
        mut self,
        id: BufferId,
        role: BufferRole,
        visibility: wgpu::ShaderStages,
    ) -> Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.curr_slot,
            visibility,
            ty: role.as_binding_type(),
            count: None
        });

        self.bindings.push(GroupEntry { 
            target: BindingTarget::Buffer(id), 
            slot: self.curr_slot 
        });

        self.curr_slot += 1;
        self
    }

    /// Add a texture to the bind group
    pub fn with_texture(
        mut self,
        id: TextureId,
        role: TextureRole,
        visibility: wgpu::ShaderStages,
    ) -> Self {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.curr_slot,
            visibility,
            ty: role.as_binding_type(),
            count: None
        });

        self.bindings.push(GroupEntry { 
            target: BindingTarget::Texture(id), 
            slot: self.curr_slot 
        });

        self.curr_slot += 1;
        self
    }
}

/// Create a new bind group from the given configuration builder and resource map
pub async fn create_bind_group(
    gpu: GpuHandle, 
    builder: BindGroupBuilder, 
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