use std::{hash::{DefaultHasher, Hash, Hasher}, marker::PhantomData, num::NonZero};

use uuid::Uuid;

use crate::{graphics::{ResourceId, SamplerId}, utils::HoldPolicy};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HashableSamplerDescriptor {
    /// used to convert lod clamps into floats for wgpu
    pub lod_divisor: u32,
    pub label: &'static str,
    pub address_mode: wgpu::AddressMode,
    pub min_filter: wgpu::FilterMode,
    pub mag_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::MipmapFilterMode,
    pub lod_min_clamp: u32,
    pub lod_max_clamp: u32, 
    pub anisotropy_clamp: u16,
    pub compare: Option<wgpu::CompareFunction>,
    pub border_color: Option<wgpu::SamplerBorderColor>,
}

impl HashableSamplerDescriptor {
    /// Converts `self` into a `wgpu::SamplerDescriptor`
    pub fn into_wgpu_desc(self) -> wgpu::SamplerDescriptor<'static> {
        wgpu::SamplerDescriptor {
            label: Some(self.label),
            address_mode_u: self.address_mode,
            address_mode_v: self.address_mode,
            address_mode_w: self.address_mode,
            min_filter: self.min_filter,
            mag_filter: self.mag_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: self.lod_min_clamp as f32 / self.lod_divisor as f32,
            lod_max_clamp: self.lod_max_clamp as f32 / self.lod_divisor as f32, 
            anisotropy_clamp: 1,
            compare: self.compare,
            border_color: self.border_color
        }
    }
}

pub struct SamplerPayload {
    pub desc: wgpu::SamplerDescriptor<'static>,
}

/// Represents resources that can be condensed into a sampler payload
pub trait SamplerType: Send + 'static {
    /// Convert the sampler into its full payload information, if possible.
    fn into_payload(self) -> Result<SamplerPayload, String>;

    /// Returns the hold policy associated with the sampler, if exists.
    fn hold_policy(&self) -> Option<HoldPolicy>;
}

/// A blueprint for constructing samplers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sampler<T: Hash> {
    pub(crate) desc: HashableSamplerDescriptor,
    pub(crate) hold_policy: Option<HoldPolicy>,
    ty: PhantomData<T>
}

impl<T: Hash> Sampler<T> {
    /// Set the label for gpu profiling of the resultant texture
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.desc.label = label;
        self
    }

    /// Set the address mode of the sampler
    pub fn with_address_mode(mut self, mode: wgpu::AddressMode) -> Self {
        self.desc.address_mode = mode;
        self
    }

    /// Set the lod clamp range
    /// 
    /// The `min` and `max` values are divided by `lod_divisor` to convert them into floats just before the `wgpu::Sampler` is created. 
    /// This allows the `Sampler` blueprint to be hashable without complex float hashing algorithms or losing precision.
    /// 
    /// Ex: min: 250, max: 250, lod_divisor: 1000 -> min: 0.25, max: 0.25
    pub fn with_lod_clamp(mut self, min: u32, max: u32, lod_divisor: u32) -> Self {
        self.desc.lod_min_clamp = min;
        self.desc.lod_max_clamp = max;
        self.desc.lod_divisor = lod_divisor;
        self
    }

    /// Set the hold policy for this sampler. This overrides the global policy on the `GpuContext`.
    pub fn with_hold_policy(mut self, policy: HoldPolicy) -> Self {
        self.hold_policy = Some(policy);
        self
    }
}

impl<T: Hash> ResourceId for Sampler<T> {
    type Id = SamplerId;

    fn create_id(&self) -> Self::Id {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        return SamplerId(Uuid::from_u128(hasher.finish() as u128));
    }
}

/// Sampler type for linear filtering
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Linear;

impl Sampler<Linear> {
    /// Create a sampler for linear filtering
    pub fn linear() -> Self {
        Self {
            ty: std::marker::PhantomData,
            hold_policy: None,
            desc: HashableSamplerDescriptor {
                label: "linear_sampler",
                address_mode: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0,
                lod_max_clamp: 32, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
                lod_divisor: 1,
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
        Ok(SamplerPayload { desc: self.desc.into_wgpu_desc() })
    }

    fn hold_policy(&self) -> Option<HoldPolicy> {
        self.hold_policy
    }
}

/// Sampler type for nearest filtering
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Nearest;

impl Sampler<Nearest> {
    /// Create a sampler for nearest filtering
    pub fn nearest() -> Self {
        Self {
            ty: std::marker::PhantomData,
            hold_policy: None,
            desc: HashableSamplerDescriptor {
                label: "nearest_sampler",
                address_mode: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                lod_min_clamp: 0,
                lod_max_clamp: 32, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
                lod_divisor: 1
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
        Ok(SamplerPayload { desc: self.desc.into_wgpu_desc() })
    }

    fn hold_policy(&self) -> Option<HoldPolicy> {
        self.hold_policy
    }
}

/// Sampler type for anisotropic filtering
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Anisotropic;

impl Sampler<Anisotropic> {
    /// Create a sampler for anisotropic filtering
    pub fn anisotropic(level: NonZero<u16>) -> Self {
        Self {
            ty: std::marker::PhantomData,
            hold_policy: None,
            desc: HashableSamplerDescriptor {
                label: "anisotropic_sampler",
                address_mode: wgpu::AddressMode::Repeat,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0,
                lod_max_clamp: 32, 
                anisotropy_clamp: level.get(),
                compare: None,
                border_color: None,
                lod_divisor: 1
            }
        }
    }
}

impl SamplerType for Sampler<Anisotropic> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        if self.desc.anisotropy_clamp > 16u16 {
            Err(format!("Cannot create sampler with label '{:?}'. Anisotropic level must be in the range [1, 16].", self.desc.label))
        } else {
            Ok(SamplerPayload { desc: self.desc.into_wgpu_desc() })
        }
    }

    fn hold_policy(&self) -> Option<HoldPolicy> {
        self.hold_policy
    }
}

/// Sampler type for borders around sampled texture
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Bordered;

impl Sampler<Bordered> {
    /// Create a sampler for clamp to border
    pub fn bordered() -> Self {
        Self {
            ty: std::marker::PhantomData,
            hold_policy: None,
            desc: HashableSamplerDescriptor {
                label: "bordered_sampler",
                address_mode: wgpu::AddressMode::ClampToBorder,
                min_filter: wgpu::FilterMode::Nearest,
                mag_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                lod_min_clamp: 0,
                lod_max_clamp: 32, 
                anisotropy_clamp: 1,
                compare: None,
                border_color: None,
                lod_divisor: 1
            }
        }
    }
}

impl SamplerType for Sampler<Bordered> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        match self.desc.address_mode {
            wgpu::AddressMode::ClampToBorder => Ok(SamplerPayload { desc: self.desc.into_wgpu_desc() }),
            _ => Err(format!("Cannot create sampler with label '{:?}'. Bordered sampler must have address mode ClampToBorder", self.desc.label))
        }
    }

    fn hold_policy(&self) -> Option<HoldPolicy> {
        self.hold_policy
    }
}

/// Sampler type for comparison between values in sampled texture
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Comparison;

impl Sampler<Comparison> {
    /// Create a sampler for value comparisons
    pub fn comparison(func: wgpu::CompareFunction) -> Self {
        Self {
            ty: std::marker::PhantomData,
            hold_policy: None,
            desc: HashableSamplerDescriptor {
                label: "comparison_sampler",
                address_mode: wgpu::AddressMode::ClampToEdge,
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                lod_min_clamp: 0,
                lod_max_clamp: 32, 
                anisotropy_clamp: 1,
                compare: Some(func),
                border_color: None,
                lod_divisor: 1
            }
        }
    }
}

impl SamplerType for Sampler<Comparison> {
    fn into_payload(self) -> Result<SamplerPayload, String> {
        Ok(SamplerPayload { desc: self.desc.into_wgpu_desc() })
    }

    fn hold_policy(&self) -> Option<HoldPolicy> {
        self.hold_policy
    }
}