use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glm::{mat4, Matrix4};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    pipeline::{GraphicsPipeline, Pipeline},
};

pub struct Transformations {
    m: Matrix4<f32>,
}

type Structure = [[f32; 4]; 4];

impl Transformations {
    fn create_array(&self) -> Structure {
        self.m.as_array().map(|v| *v.as_array())
    }

    pub fn update_buffer(&self, buffer: Arc<CpuAccessibleBuffer<Structure>>) {
        let mut lock = buffer.write().expect("failed to lock");

        *lock = self.m.as_array().map(|v| *v.as_array());
    }

    pub fn create_descriptor(
        &self,
        device: Arc<Device>,
        pipeline: Arc<GraphicsPipeline>,
    ) -> (
        Arc<CpuAccessibleBuffer<Structure>>,
        Arc<PersistentDescriptorSet>,
    ) {
        let uniform_data_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::all(), //TODO: this should be more specific?
            false,
            self.create_array(),
        )
        .expect("failed to create buffer");

        //we are creating the layout for set 0
        let layout = pipeline.layout().set_layouts().get(0).unwrap();

        let uniform_set = PersistentDescriptorSet::new(
            layout.clone(),
            [WriteDescriptorSet::buffer(0, uniform_data_buffer.clone())], // 0 is the binding in GLSL when we use this set
        )
        .unwrap();

        (uniform_data_buffer, uniform_set)
    }
}

impl Default for Transformations {
    fn default() -> Self {
        Self {
            m: mat4(
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ),
        }
    }
}
