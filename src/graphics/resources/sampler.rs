use std::{marker::PhantomData, num::NonZero};

pub struct SamplerPayload {
    pub desc: wgpu::SamplerDescriptor<'static>,
}

/// Represents resources that can be condensed into a sampler payload
pub trait SamplerType: Send + 'static {
    /// Convert the sampler into its full payload information, if possible.
    fn into_payload(self) -> Result<SamplerPayload, String>;
}

/// A blueprint for constructing samplers
pub struct Sampler<T> {
    desc: wgpu::SamplerDescriptor<'static>,
    ty: PhantomData<T>
}

impl<T> Sampler<T> {
    /// Set the label for gpu profiling of the resultant texture
    pub fn with_label(mut self, label: &'static str) -> Self {
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

    /// Set the lod clamp range
    pub fn with_lod_clamp(mut self, min: f32, max: f32) -> Self {
        self.desc.lod_min_clamp = min;
        self.desc.lod_max_clamp = max;
        self
    }
}

/// Sampler type for linear filtering
pub struct Linear;

impl Sampler<Linear> {
    /// Create a sampler for linear filtering
    pub fn linear() -> Self {
        Self {
            ty: std::marker::PhantomData,
            desc: wgpu::SamplerDescriptor {
                label: Some("linear_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
            }
        }
    }

    /// Set the mipmap filter to nearest. (min and mag filter remain linear)
    pub fn with_mipmap_nearest(mut self) -> Self {
        self.desc.mipmap_filter = wgpu::MipmapFilterMode::Nearest;
        self
    }
}

impl SamplerType for Sampler<Linear> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        Ok(SamplerPayload { desc: self.desc })
    }
}

/// Sampler type for nearest filtering
pub struct Nearest;

impl Sampler<Nearest> {
    /// Create a sampler for nearest filtering
    pub fn nearest() -> Self {
        Self {
            ty: std::marker::PhantomData,
            desc: wgpu::SamplerDescriptor {
                label: Some("nearest_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
            }
        }
    }

    /// Set the mipmap filter to linear. (min and mag filter remain nearest)
    pub fn with_mipmap_linear(mut self) -> Self {
        self.desc.mipmap_filter = wgpu::MipmapFilterMode::Linear;
        self
    }
}

impl SamplerType for Sampler<Nearest> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        Ok(SamplerPayload { desc: self.desc })
    }
}

/// Sampler type for anisotropic filtering
pub struct Anisotropic;

impl Sampler<Anisotropic> {
    /// Create a sampler for anisotropic filtering
    pub fn anisotropic(level: NonZero<u16>) -> Self {
        Self {
            ty: std::marker::PhantomData,
            desc: wgpu::SamplerDescriptor {
                label: Some("anisotropic_sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0, 
                anisotropy_clamp: level.get(),
                compare: None,
                border_color: None,
            }
        }
    }
}

impl SamplerType for Sampler<Anisotropic> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        if self.desc.anisotropy_clamp.gt(&16u16) {
            Err(format!("Cannot create sampler with label '{:?}'. Anisotropic level must be in the range [1, 16].", self.desc.label))
        } else {
            Ok(SamplerPayload { desc: self.desc })
        }
    }
}

/// Sampler type for borders around sampled texture
pub struct Bordered;

impl Sampler<Bordered> {
    /// Create a sampler for clamp to border
    pub fn bordered() -> Self {
        Self {
            ty: std::marker::PhantomData,
            desc: wgpu::SamplerDescriptor {
                label: Some("bordered_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToBorder,
                address_mode_v: wgpu::AddressMode::ClampToBorder,
                address_mode_w: wgpu::AddressMode::ClampToBorder,
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
            }
        }
    }
}

impl SamplerType for Sampler<Bordered> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        match self.desc.address_mode_u {
            wgpu::AddressMode::ClampToBorder => Ok(SamplerPayload { desc: self.desc }),
            _ => Err(format!("Cannot create sampler with label '{:?}'. Bordered sampler must have address mode ClampToBorder", self.desc.label))
        }
    }
}

/// Sampler type for comparison between values in sampled texture
pub struct Comparison;

impl Sampler<Comparison> {
    /// Create a sampler for texture value comparisons
    pub fn comparison(func: wgpu::CompareFunction) -> Self {
        Self {
            ty: std::marker::PhantomData,
            desc: wgpu::SamplerDescriptor {
                label: Some("comparison_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0.0,
                lod_max_clamp: 32.0, 
                anisotropy_clamp: 1,
                compare: Some(func),
                border_color: None,
            }
        }
    }
}

impl SamplerType for Sampler<Comparison> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        Ok(SamplerPayload { desc: self.desc })
    }
}