use crate::{graphics::{Buffer, BufferId, GpuContext, Serializable}, utils::{GeometryData, Initialized, Transform, TransformAttribute, Vec2Attribute, Vec3Attribute, Vec4Attribute}};

/// A quad for rendering characters
#[derive(Debug)]
pub struct FontQuad {
    pub vertices: GeometryData<Initialized>,
    pub instances: GeometryData<Initialized>,
    pub idx_buf_id: BufferId,
}

impl FontQuad {
    pub fn new() -> Self {
        Self {
            vertices: GeometryData::placeholder(),
            instances: GeometryData::placeholder(),
            idx_buf_id: BufferId("quad_index_buffer"),
        }
    }

    /// Initialize the quad with the gpu, creating the vertex data that it represents. 
    pub fn init(&mut self, context: &mut GpuContext, max_instances: u64) {
        let vert_positions: Vec<[f32; 3]> = vec![
            [ 0.5,  0.5, 0.0], [-0.5,  0.5, 0.0], [-0.5, -0.5, 0.0], [ 0.5, -0.5, 0.0],
        ];

        let vert_uvs: Vec<[f32; 2]> = vec![
            [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]
        ];

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
        
        self.vertices = GeometryData::as_vertex_group(0)
            .with_label("Quad Vertices")
            .with_attribute(Vec3Attribute("positions"), vert_positions)
            .with_attribute(Vec2Attribute("uvs"), vert_uvs)
            .init(context, 4);

        self.instances = GeometryData::as_instance_group(2)
            .with_label("Font Quad Instances")
            .with_attribute(TransformAttribute("transform"), Vec::<Transform>::new())
            .with_attribute(Vec4Attribute("bounds"), Vec::<glam::Vec4>::new())            
            .init(context, max_instances);

        context.request_buffer(
            &self.idx_buf_id, 
            Buffer::as_index()
                .with_label("Quad Index Buffer")
                .with_byte_data(&indices.to_bytes())
                .writable()
        );
    }

    /// Update the instance buffer owned by this `Quad`
    pub fn update_instances(&mut self, context: &mut GpuContext) {
        self.instances.update(context);
    }

    /// Get the ids to the geometry (vertex/instance) buffers for this quad.
    pub fn geo_buf_ids(&self) -> [BufferId; 2] {
        return [*self.vertices.buf_id(), *self.instances.buf_id()]
    }
}