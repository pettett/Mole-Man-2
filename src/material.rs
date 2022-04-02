use std::sync::Arc;

use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::{GraphicsPipeline, Pipeline},
    shader::ShaderModule,
};

use crate::{engine::Engine, gl};

pub struct Material {
    pub vs: Arc<ShaderModule>,
    pub fs: Arc<ShaderModule>,
    descriptors: Arc<PersistentDescriptorSet>,

    pub pipeline: Arc<GraphicsPipeline>,
}

impl Material {
    pub fn new(
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        descriptor_wites: impl IntoIterator<Item = WriteDescriptorSet>,
        engine: &Engine,
    ) -> Self {
        let pipeline = gl::get_pipeline(
            engine.device(),
            vs.clone(),
            fs.clone(),
            engine.render_pass().render_pass(),
            engine.viewport().clone(),
        );

        Self {
            vs: vs.clone(),
            fs: fs.clone(),

            //we are creating the layout for set 0
            descriptors: PersistentDescriptorSet::new(
                pipeline.layout().set_layouts().get(0).unwrap().clone(),
                descriptor_wites,
            )
            .unwrap(),
            pipeline,
        }
    }

    pub fn descriptors(&self) -> Arc<PersistentDescriptorSet> {
        self.descriptors.clone()
    }
}
