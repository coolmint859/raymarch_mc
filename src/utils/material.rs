use std::{any::Any, fmt::Debug};

use indexmap::IndexMap;

use crate::{graphics::{BindGroup, BindGroupId, BindGroupIdPair, Bindable, BindingTarget, Buffer, BufferBinding, BufferId, GpuContext, LayoutId, Sampler, SamplerBinding, SamplerId, SamplerType, Serializable, StructuredUpdate, Texture, TextureBinding, TextureId, TextureType, TextureTypeSampled, Uniform}, utils::{Building, Empty}};

#[derive(Debug)]
pub struct MatInit {
    pub mat_bg: BindGroupIdPair
}

#[derive(Debug)]
pub struct Material<State> {
    components: IndexMap<String, Box<dyn MaterialComponent>>,
    label: String,
    state: State,
}

impl Material<Empty> {
    pub fn new() -> Self {
        Self {
            components: IndexMap::new(),
            label: "material_bind_group".to_string(),
            state: Empty
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add a component to this material
    pub fn with_component<C>(mut self, name: impl Into<String>, component: C) -> Material<Building> 
    where C: MaterialComponent + 'static
    {
        self.components.insert(name.into(), Box::new(component));
        Material::from_empty(self)
    }
}

impl Material<Building> {
    fn from_empty(empty: Material<Empty>) -> Self {
        Self {
            components: empty.components,
            label: empty.label,
            state: Building
        }
    }

    /// Add a label for the buffer this `UniformComponent` represents for gpu profiling
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Add a component to this material
    pub fn with_component<C>(mut self, name: impl Into<String>, component: C) -> Self 
    where C: MaterialComponent + 'static
    {
        self.components.insert(name.into(), Box::new(component));
        self
    }

    /// Initialize the `Material` as a bind group with the gpu
    pub fn init(mut self, context: &mut GpuContext) -> Material<MatInit> {
        let mut bg_blueprint = BindGroup::new().with_label(&self.label);
        for comp in &mut self.components.values_mut() {
            comp.init(context);
            bg_blueprint.add_entry(comp.binding());
        }

        let mat_bg = context.request_bind_group(bg_blueprint);
        Material::from_uninit(self, MatInit { mat_bg })
    }
}

impl Material<MatInit> {
    pub fn placeholder() -> Self {
        Self {
            components: IndexMap::new(),
            label: "uninit_material".to_string(),
            state: MatInit { mat_bg: BindGroupIdPair::UNINIT }
        }
    }

    pub fn from_uninit(uninit: Material<Building>, state: MatInit) -> Self {
        Self {
            components: uninit.components,
            label: uninit.label,
            state
        }
    }

    pub fn bg_id(&self) -> BindGroupId {
        self.state.mat_bg.id
    }

    pub fn layout_id(&self) -> LayoutId {
        self.state.mat_bg.layout_id
    }

    /// Modify a uniform buffer on this `Material`, if exists, using the provided closure `F`.
    pub fn modify_uniform<T, F>(&mut self, name: impl Into<String>, modifer: F) 
    where 
        T: Serializable + Debug + 'static, 
        F: FnOnce(&mut T)
    {
        if let Some(component) = self.components.get_mut(&name.into()) {
            if let Some(buffer_comp) = component.downcast_mut::<UniformComponent<T>>() {
                buffer_comp.modify(modifer)
            }
        }
    }

    pub fn update(&mut self, context: &mut GpuContext) {
        for comp in self.components.values_mut() {
            comp.update(context);
        }
    }
}

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

pub trait MaterialComponent: Debug {
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

/// A `MaterialComponent` for uniform buffers
#[derive(Debug)]
pub struct UniformComponent<T: Serializable> {
    id: BufferId,
    blueprint: Buffer<Uniform>,
    is_dirty: bool,
    data: T,
    visibility: wgpu::ShaderStages,
}

impl<T: Serializable> UniformComponent<T> {
    pub fn new(blueprint: Buffer<Uniform>, data: T) -> Self {
        Self {
            id: BufferId::UNINIT,
            blueprint,
            data,
            is_dirty: true,
            visibility: wgpu::ShaderStages::FRAGMENT
        }
    }

    /// Set the shader visibility of the buffer when bound to the `Material`'s bind group
    pub fn with_visibility(mut self, vis: wgpu::ShaderStages) -> Self {
        self.visibility = vis;
        self
    } 

    /// Modify the data stored in this `UniformComponent` with a closure `F`
    pub fn modify(&mut self, modifier: impl FnOnce(&mut T)) {
        modifier(&mut self.data);
        self.is_dirty = true;
    }
}

impl<T> MaterialComponent for UniformComponent<T>
where T: Serializable + Debug + 'static
{
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        let bp = self.blueprint.clone()
            .with_byte_data(self.data.to_bytes());

        self.id = context.request_buffer(bp);
    }

    fn binding(&self) -> MaterialBinding {
        let binding = BufferBinding::as_uniform(self.id)
            .with_visibility(self.visibility);
        
        MaterialBinding::Buffer(binding)
    }

    fn update(&mut self, context: &mut GpuContext) {
        if self.is_dirty {
            let _ = context.update_buffer(&self.id, StructuredUpdate {
                data: &self.data,
                offset: 0
            });
            
            self.is_dirty = false;
        }
    }
}

/// A `MaterialComponent` for textures
#[derive(Debug)]
pub struct TextureComponent<T> {
    id: TextureId,
    blueprint: Texture<T>,
    sample_state: TextureTypeSampled,
    visibility: wgpu::ShaderStages,
}

impl<T> TextureComponent<T> {
    pub fn new(blueprint: Texture<T>) -> Self {
        Self {
            id: TextureId::UNINIT,
            blueprint,
            sample_state: TextureTypeSampled::default(),
            visibility: wgpu::ShaderStages::FRAGMENT,
        }
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
    
    /// Set the shader visibility of the buffer when bound to the `Material`'s bind group
    pub fn with_visibility(mut self, vis: wgpu::ShaderStages) -> Self {
        self.visibility = vis;
        self
    }
}

impl<T> MaterialComponent for TextureComponent<T> 
where 
    T: Debug + Clone + 'static,
    Texture<T>: TextureType
{
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        self.id = context.request_texture(self.blueprint.clone())
    }

    fn binding(&self) -> MaterialBinding {
        let binding = TextureBinding::as_sampled(self.id, self.sample_state)
            .with_visibility(self.visibility);

        MaterialBinding::Texture(binding)
    }
}

/// A `MaterialComponent` for samplers
#[derive(Debug)]
pub struct SamplerComponent<T> {
    id: SamplerId,
    blueprint: Sampler<T>,
    visibility: wgpu::ShaderStages,
    binding_type: wgpu::SamplerBindingType,
}

impl<T> SamplerComponent<T> {
    pub fn new(blueprint: Sampler<T>) -> Self {
        Self {
            id: SamplerId::UNINIT,
            blueprint,
            visibility: wgpu::ShaderStages::FRAGMENT,
            binding_type: wgpu::SamplerBindingType::NonFiltering,
        }
    }

    /// Set the shader visibility of the buffer when bound to the `Material`'s bind group
    pub fn with_visibility(mut self, vis: wgpu::ShaderStages) -> Self {
        self.visibility = vis;
        self
    }

    /// Set the binding type of the sampler binding
    pub fn with_binding_type(mut self, ty: wgpu::SamplerBindingType) -> Self{
        self.binding_type = ty;
        self
    }
}

impl<T> MaterialComponent for SamplerComponent<T> 
where
    T: Debug + Clone + 'static,
    Sampler<T>: SamplerType,
{
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn init(&mut self, context: &mut GpuContext) {
        self.id = context.request_sampler(self.blueprint.clone())
    }

    fn binding(&self) -> MaterialBinding {
        let binding = SamplerBinding::new(self.id)
            .with_binding_type(self.binding_type)
            .with_visibility(self.visibility);

        MaterialBinding::Sampler(binding)
    }
}
