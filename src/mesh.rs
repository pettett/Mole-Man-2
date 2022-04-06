use std::sync::Arc;

use vulkano::{
    buffer::{BufferContents, BufferUsage, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    device::Queue,
    pipeline::graphics::{input_assembly::Index, vertex_input::Vertex},
};

use crate::gl::{self};

pub trait Mesh {
    fn bind(&self, cmd_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>);

    fn indices(&self) -> u32;
}

pub struct GPUMesh<
    Ib: TypedBufferAccess<Content = [I]> + 'static,
    I: Index + 'static,
    Vb: TypedBufferAccess<Content = [V]> + 'static,
    V: Vertex + 'static,
> {
    vertex_buffer: Arc<Vb>,
    index_buffer: Arc<Ib>,
    indexes: u32,
}

type ImmutableMesh<I, V> = GPUMesh<ImmutableBuffer<[I]>, I, ImmutableBuffer<[V]>, V>;

impl dyn Mesh {
    pub fn create_unit_square(queue: Arc<Queue>) -> Box<ImmutableMesh<u32, gl::Vertex>> {
        let vertex1 = gl::Vertex {
            position: [1., 0.],
            color: [0., 0., 1.],
        };
        let vertex2 = gl::Vertex {
            position: [0., 0.],
            color: [0., 1., 0.],
        };
        let vertex3 = gl::Vertex {
            position: [0., 1.],
            color: [1., 0., 0.],
        };
        let vertex4 = gl::Vertex {
            position: [1., 1.],
            color: [1., 1., 0.],
        };

        let (vertex_buffer, _vertex_future) = ImmutableBuffer::from_iter(
            vec![vertex1, vertex2, vertex3, vertex4].into_iter(),
            BufferUsage::vertex_buffer(),
            queue.clone(),
        )
        .unwrap();

        let (index_buffer, _index_future) = ImmutableBuffer::from_iter(
            vec![0u32, 1u32, 2u32, 2u32, 0u32, 3u32].into_iter(),
            BufferUsage::index_buffer(),
            queue.clone(),
        )
        .unwrap();

        Box::new(
            GPUMesh::<ImmutableBuffer<[u32]>, u32, ImmutableBuffer<[gl::Vertex]>, gl::Vertex> {
                vertex_buffer,
                index_buffer,
                indexes: 6,
            },
        )
    }
}

impl<I, V> Mesh for ImmutableMesh<I, V>
where
    I: Index + 'static,
    V: Vertex + 'static,
    ImmutableBuffer<[I]>: TypedBufferAccess<Content = [I]>,
    ImmutableBuffer<[V]>: TypedBufferAccess<Content = [V]>,
    [I]: BufferContents,
{
    fn bind(&self, cmd_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        cmd_builder
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .bind_index_buffer(self.index_buffer.clone());
    }

    fn indices(&self) -> u32 {
        self.indexes
    }
}
