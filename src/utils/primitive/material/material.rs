use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexMap;

use crate::{graphics::{BindGroup, BindGroupId, GpuContext, LayoutId, NamedBindGroup, Serializable}, utils::{Building, Empty, MaterialComponent, UniformComponent}};

static MATERIAL_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct MatInit {
    pub bg: NamedBindGroup
}

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
        let id_num = MATERIAL_COUNTER.fetch_add(1, Ordering::SeqCst);
        let id = Box::new(format!("mat_bind_group_{}", id_num));

        let mut bg_blueprint = BindGroup::new().with_label(&self.label);
        for comp in &mut self.components.values_mut() {
            comp.init(context);
            bg_blueprint.add_entry(comp.binding());
        }

        let bg = NamedBindGroup::new(Box::leak(id));
        context.request_bind_group(
            &bg.id, 
            &bg.layout_id, 
            bg_blueprint
        );
         
        Material::from_uninit(self, MatInit { bg })
    }
}

impl Material<MatInit> {
    pub fn placeholder() -> Self {
        Self {
            components: IndexMap::new(),
            label: "uninit_material".to_string(),
            state: MatInit { bg: NamedBindGroup::new("uninit_material") }
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
        self.state.bg.id
    }

    pub fn layout_id(&self) -> LayoutId {
        self.state.bg.layout_id
    }

    /// Modify a uniform buffer on this `Material`, if exists, using the provided closure `F`.
    pub fn modify_uniform<T, F>(&mut self, name: impl Into<String>, modifer: F) 
    where 
        T: Serializable + 'static, 
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
