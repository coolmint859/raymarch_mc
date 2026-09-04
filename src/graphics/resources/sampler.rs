use crate::graphics::{Bindable, BindingTarget, SamplerId};

pub struct Sampler {
    pub label: String,
    pub desc: wgpu::SamplerDescriptor<'static>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            label: "sampler".to_string(),
            desc: wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        }
    }

    /// Set the label for gpu profiling of the resultant texture
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = label.to_string();
        self.desc.label = Some(label);
        self
    }

    /// Set the address mode of the sampler
    pub fn with_address_mode(mut self, mode: wgpu::AddressMode) -> Self {
        self.desc.address_mode_u = mode;
        self.desc.address_mode_v = mode;
        self.desc.address_mode_w = mode;
        self
    }

    /// Set the min filter mode of the sampler
    pub fn with_min_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.desc.min_filter = filter;
        self
    }

    /// Set the mag filter mode of the sampler
    pub fn with_mag_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.desc.mag_filter = filter;
        self
    }
}

pub struct SamplerBinding {
    sampler_id: SamplerId,
    visibility: wgpu::ShaderStages,
}

impl SamplerBinding {
    pub fn new(target: SamplerId) -> Self {
        Self {
            sampler_id: target,
            visibility: wgpu::ShaderStages::FRAGMENT,
        }
    }

    /// Set the shader stage visibility for the sampler binding
    pub fn with_visibility(mut self, visibility: wgpu::ShaderStages) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Bindable for SamplerBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    }

    fn target(&self) -> super::BindingTarget {
        BindingTarget::Sampler(self.sampler_id)
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        self.visibility
    }
}