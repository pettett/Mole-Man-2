use std::sync::Arc;

use glm::{mat4, Matrix4};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    pipeline::{GraphicsPipeline, Pipeline},
};

pub struct Transformations {
    m: Matrix4<f32>,
    buffer: Arc<CpuAccessibleBuffer<Structure>>,
}

type Structure = [[f32; 4]; 4];

impl Transformations {
    fn create_array(&self) -> Structure {
        self.m.as_array().map(|v| *v.as_array())
    }

    pub fn update_buffer(&self) {
        let mut lock = self.buffer.write().expect("failed to lock");

        *lock = self.create_array();
    }
    pub fn get_buffer(&self) -> Arc<CpuAccessibleBuffer<Structure>> {
        self.buffer.clone()
    }

    pub fn transform(&mut self) -> &mut Matrix4<f32> {
        &mut self.m
    }
}

impl Transformations {
    pub fn new(device: Arc<Device>, pipeline: Arc<GraphicsPipeline>) -> Self {
        let uniform_data_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::all(), //TODO: this should be more specific?
            false,
            [[0.; 4]; 4],
        )
        .expect("failed to create buffer");

        let s = Self {
            buffer: uniform_data_buffer,
            m: mat4(
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ),
        };

        s.update_buffer();

        s
    }
}
