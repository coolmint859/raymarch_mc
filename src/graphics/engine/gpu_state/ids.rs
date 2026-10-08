use uuid::Uuid;

/// unique identifier to a buffer
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct BufferId(pub Uuid);

impl BufferId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = BufferId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid {
        self.0
    }
}

/// unique identifier for a texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct TextureId(pub &'static str);

/// unique identifier for a sampler
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct SamplerId(pub &'static str);

/// unique identifier for a pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct PipelineId(pub &'static str);

/// unique identifier for a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct BindGroupId(pub &'static str);

/// unique identifier for a bind group layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct LayoutId(pub &'static str);

/// Helper struct encapsulating the id of a bind group and it's associated layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NamedBindGroup {
    pub layout_id: LayoutId,
    pub id: BindGroupId
}

impl NamedBindGroup {
    pub fn new(name: &'static str) -> Self {
        Self {
            id: BindGroupId(name),
            layout_id: LayoutId(
                Box::leak(Box::new(format!("{name}_layout")))
            )
        }
    }
}