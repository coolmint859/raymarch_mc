use uuid::Uuid;

/// unique identifier to a buffer
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct BufferId(pub Uuid);

impl BufferId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = BufferId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// unique identifier for a texture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureId(pub Uuid);

impl TextureId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = TextureId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// unique identifier for a sampler
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct SamplerId(pub Uuid);

impl SamplerId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = SamplerId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// unique identifier for a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct BindGroupId(pub Uuid);

impl BindGroupId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = BindGroupId(Uuid::nil());

    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// unique identifier for a bind group layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct LayoutId(pub Uuid);

impl LayoutId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = LayoutId(Uuid::nil());
    
    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}

/// unique identifiers for a bind group and it's layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct BindGroupIdPair {
    pub id: BindGroupId,
    pub layout_id: LayoutId,
}

impl BindGroupIdPair {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = BindGroupIdPair {
        id: BindGroupId::UNINIT,
        layout_id: LayoutId::UNINIT
    };
}

/// unique identifier for a pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] 
pub struct PipelineId(pub Uuid);

impl PipelineId {
    /// A uninitialized `BufferId` (points to nothing). 
    pub const UNINIT: Self = PipelineId(Uuid::nil());
    
    /// Returns a copy of the inner `Uuid`
    pub fn get(&self) -> Uuid { self.0 }
}