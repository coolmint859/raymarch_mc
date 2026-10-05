use std::{any::Any, marker::PhantomData, num::NonZero, sync::atomic::{AtomicU32, Ordering}};

use crate::graphics::{Anisotropic, Bindable, BindingTarget, Bordered, Buffer, BufferBinding, BufferId, Comparison, Computed, GpuContext, Linear, Nearest, OnDisk, Procedural, Sampler, SamplerBinding, SamplerId, Serializable, StructuredUpdate, Texture, TextureBinding, TextureId, TextureTypeSampled};

static BUFFER_COUNTER: AtomicU32 = AtomicU32::new(0);
static TEXTURE_COUNTER: AtomicU32 = AtomicU32::new(0);
static SAMPLER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A high level wrapper over Bindings in a `BindGroup`, for use by a `MaterialComponent`
pub enum MaterialBinding {
    Buffer(BufferBinding),
    Texture(TextureBinding),
    Sampler(SamplerBinding)
}

impl Bindable for MaterialBinding {
    fn as_binding(&self) -> wgpu::BindingType {
        match self {
            MaterialBinding::Buffer(b) => b.as_binding(),
            MaterialBinding::Texture(t) => t.as_binding(),
            MaterialBinding::Sampler(s) => s.as_binding(),
        }
    }

    fn target(&self) -> BindingTarget {
        match self {
            MaterialBinding::Buffer(b) => b.target(),
            MaterialBinding::Texture(t) => t.target(),
            MaterialBinding::Sampler(s) => s.target(),
        }
    }

    fn visibility(&self) -> wgpu::ShaderStages {
        match self {
            MaterialBinding::Buffer(b) => b.visibility(),
            MaterialBinding::Texture(t) => t.visibility(),
            MaterialBinding::Sampler(s) => s.visibility(),
        }
    }
}

/// Represents bindable entries into a material (bind group)
pub trait MaterialComponent {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Initialize the `MaterialComponent` with a `GpuContext`
    fn init(&mut self, context: &mut GpuContext);

    /// Returns the binding for the `MaterialComponent`
    fn binding(&self) -> MaterialBinding;

    /// Updates the `MaterialComponent`, if needed. Does nothing by default.
    fn update(&mut self, _context: &mut GpuContext) { }
}

impl dyn MaterialComponent {
    /// Downcast this `MaterialComponent` into a reference of the concrete type
    pub fn downcast_ref<C: MaterialComponent + 'static>(&self) -> Option<&C> {
        self.as_any().downcast_ref::<C>()
    }

    /// Downcast this `MaterialComponent` into a mutable reference of the concrete type
    pub fn downcast_mut<C: MaterialComponent + 'static>(&mut self) -> Option<&mut C> {
        self.as_any_mut().downcast_mut::<C>()
    }
}

pub struct UniformComponent<T: Serializable> {
    buf_id: BufferId,
    label: String,
    is_dirty: bool,
    data: T,
}

impl<T: Serializable> UniformComponent<T> {
    pub fn new(uniform: T) -> Self {
        let id_num = BUFFER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("uniform_buffer_{}", id_num));
        
        Self {
            buf_id: BufferId(Box::leak(id)),
            label: "uniform_buffer".to_string(),
            data: uniform,
            is_dirty: true,
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Modify the data stored in this `UniformComponent` with a closure `F`
    pub fn modify(&mut self, modifier: impl FnOnce(&mut T)) {
        modifier(&mut self.data);

        self.is_dirty = true;
    }
}

impl<T: Serializable + 'static> MaterialComponent for UniformComponent<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_buffer(
            &self.buf_id, 
            Buffer::as_uniform()
                .with_label(&self.label)
                .with_byte_data(self.data.to_bytes())
                .writable()
        );
    }

    fn binding(&self) -> MaterialBinding {
        let binding = BufferBinding::as_uniform(self.buf_id);
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);
        
        MaterialBinding::Buffer(binding)
    }

    fn update(&mut self, context: &mut GpuContext) {
        if self.is_dirty {
            let _ = context.update_buffer(&self.buf_id, StructuredUpdate {
                data: &self.data,
                offset: 0
            });
            
            self.is_dirty = false;
        }
    }
}

pub struct TextureComponent<T> {
    tex_id: TextureId,
    label: String,
    sample_state: TextureTypeSampled,
    format: wgpu::TextureFormat,
    ty: T,
}

impl TextureComponent<OnDisk> {
    pub fn on_disk(ty: OnDisk) -> Self {
        let id_num = TEXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("texture_{}", id_num));

        Self {
            tex_id: TextureId(Box::leak(id)),
            label: "texture_comp".to_string(),
            sample_state: TextureTypeSampled::default(),
            format: OnDisk::default_fmt(),
            ty,
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Allow the texture to be filtered when sampled
    pub fn filterable(mut self) -> Self {
        self.sample_state.filterable = true;
        self
    }

    /// Allow the texture to be multisampled
    pub fn multisampled(mut self) -> Self {
        self.sample_state.multisampled = true;
        self
    }
}

impl MaterialComponent for TextureComponent<OnDisk> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_texture(
            &self.tex_id, 
            Texture::on_disk(self.ty.path.clone())
                .with_label(&self.label)
                .with_format(self.format)
                .writable()
        );
    }

    fn binding(&self) -> MaterialBinding {
        let binding = TextureBinding::as_sampled(self.tex_id, self.sample_state.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Texture(binding)
    }
}

impl TextureComponent<Procedural> {
    pub fn procedural(ty: Procedural) -> Self {
        let id_num = TEXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("texture_{}", id_num));

        Self {
            tex_id: TextureId(Box::leak(id)),
            label: "texture_comp".to_string(),
            sample_state: TextureTypeSampled::default(),
            format: Procedural::default_fmt(),
            ty
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Allow the texture to be filtered when sampled
    pub fn filterable(mut self) -> Self {
        self.sample_state.filterable = true;
        self
    }

    /// Allow the texture to be multisampled
    pub fn multisampled(mut self) -> Self {
        self.sample_state.multisampled = true;
        self
    }
}

impl MaterialComponent for TextureComponent<Procedural> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        // NOTE: texture data will eventually not be cloned on upload, but instead be copy-on-write
        // This requires a significant refactor the context, so I'm not worrying about it for now
        context.request_texture(
            &self.tex_id, 
            Texture::procedural(self.ty.data.clone(), self.ty.dim)
                .with_label(&self.label)
                .with_format(self.format)
                .writable()
        );
    }

    fn binding(&self) -> MaterialBinding {
        let binding = TextureBinding::as_sampled(self.tex_id, self.sample_state.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Texture(binding)
    }
}

impl TextureComponent<Computed> {
    pub fn procedural(ty: Computed) -> Self {
        let id_num = TEXTURE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("texture_{}", id_num));

        Self {
            tex_id: TextureId(Box::leak(id)),
            label: "texture_comp".to_string(),
            sample_state: TextureTypeSampled::default(),
            format: Computed::default_fmt(),
            ty
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Allow the texture to be filtered when sampled
    pub fn filterable(mut self) -> Self {
        self.sample_state.filterable = true;
        self
    }

    /// Allow the texture to be multisampled
    pub fn multisampled(mut self) -> Self {
        self.sample_state.multisampled = true;
        self
    }
}

impl MaterialComponent for TextureComponent<Computed> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        // NOTE: texture data will eventually not be cloned on upload, but instead be copy-on-write
        // This requires a significant refactor the context, so I'm not worrying about it for now
        context.request_texture(
            &self.tex_id, 
            Texture::computed(self.ty.dim)
                .with_label(&self.label)
                .with_format(self.format)
                .writable()
        );
    }

    fn binding(&self) -> MaterialBinding {
        let binding = TextureBinding::as_sampled(self.tex_id, self.sample_state.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Texture(binding)
    }
}

pub struct SamplerComponent<T> {
    samp_id: SamplerId,
    anisotrophic_level: NonZero<u16>,
    compare_fn: Option<wgpu::CompareFunction>,
    _ty: PhantomData<T>
}

impl SamplerComponent<Linear> {
    pub fn linear() -> Self {
        let id_num = SAMPLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("sampler_{}", id_num));

        Self {
            samp_id: SamplerId(Box::leak(id)),
            anisotrophic_level: NonZero::new(1).unwrap(),
            compare_fn: None,
            _ty: std::marker::PhantomData
        }
    }
}

impl MaterialComponent for SamplerComponent<Linear> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_sampler(&self.samp_id, Sampler::linear());
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.samp_id.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Sampler(binding)
    }
}

impl SamplerComponent<Nearest> {
    pub fn nearest() -> Self {
        let id_num = SAMPLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("sampler_{}", id_num));

        Self {
            samp_id: SamplerId(Box::leak(id)),
            anisotrophic_level: NonZero::new(1).unwrap(),
            compare_fn: None,
            _ty: std::marker::PhantomData
        }
    }
}

impl MaterialComponent for SamplerComponent<Nearest> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_sampler(&self.samp_id, Sampler::nearest());
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.samp_id.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Sampler(binding)
    }
}

impl SamplerComponent<Anisotropic> {
    pub fn anisotropic(level: NonZero<u16>) -> Self {
        let id_num = SAMPLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("sampler_{}", id_num));

        Self {
            samp_id: SamplerId(Box::leak(id)),
            anisotrophic_level: level,
            compare_fn: None,
            _ty: std::marker::PhantomData
        }
    }
}

impl MaterialComponent for SamplerComponent<Anisotropic> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_sampler(
            &self.samp_id, 
            Sampler::anisotropic(self.anisotrophic_level)
        );
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.samp_id.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Sampler(binding)
    }
}

impl SamplerComponent<Bordered> {
    pub fn bordered() -> Self {
        let id_num = SAMPLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("sampler_{}", id_num));

        Self {
            samp_id: SamplerId(Box::leak(id)),
            anisotrophic_level: NonZero::new(1).unwrap(),
            compare_fn: None,
            _ty: std::marker::PhantomData
        }
    }
}

impl MaterialComponent for SamplerComponent<Bordered> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_sampler(&self.samp_id, Sampler::bordered());
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.samp_id.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Sampler(binding)
    }
}

impl SamplerComponent<Comparison> {
    pub fn comparison() -> Self {
        let id_num = SAMPLER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("sampler_{}", id_num));

        Self {
            samp_id: SamplerId(Box::leak(id)),
            anisotrophic_level: NonZero::new(1).unwrap(),
            compare_fn: Some(wgpu::CompareFunction::Greater),
            _ty: std::marker::PhantomData
        }
    }
}

impl MaterialComponent for SamplerComponent<Comparison> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        context.request_sampler(&self.samp_id, Sampler::comparison(self.compare_fn.unwrap()));
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.samp_id.clone());
            // .with_visibility(wgpu::ShaderStages::FRAGMENT);

        MaterialBinding::Sampler(binding)
    }
}