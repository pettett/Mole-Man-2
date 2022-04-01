use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::WriteDescriptorSet,
    device::{Device, Queue},
    format::{ClearValue, Format},
    image::{
        view::{ImageView, ImageViewCreateInfo},
        ImageAccess, ImageAspects, ImageDimensions, ImageViewAbstract, ImmutableImage,
        StorageImage,
    },
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
    shader::spirv::StorageClass,
    sync::{self, GpuFuture},
};

use crate::Engine;

pub trait AnyTexture: ImageAccess {}

impl AnyTexture for StorageImage {}

impl AnyTexture for ImmutableImage {}

pub struct Texture<I: AnyTexture + 'static> {
    image: Arc<I>,
    view: Arc<ImageView<I>>,
    sample: Arc<Sampler>,
}

impl<I: AnyTexture + 'static> Clone for Texture<I> {
    fn clone(&self) -> Self {
        Self {
            image: self.image.clone(),
            view: self.view.clone(),
            sample: self.sample.clone(),
        }
    }
}

impl<I: AnyTexture + 'static> Texture<I> {
    pub fn get_size(&self) -> [u32; 2] {
        self.image.dimensions().width_height()
    }

    pub fn new(image: Arc<I>, device: Arc<Device>) -> Self {
        let mut aspects = ImageAspects::none();
        aspects.color = true;

        let view = ImageView::new(
            image.clone(),
            ImageViewCreateInfo {
                format: Some(image.format()),
                aspects,
                ..ImageViewCreateInfo::default()
            },
        )
        .unwrap();

        let sample = Sampler::new(
            device,
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                lod: 0.0..=1.0,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            view,
            sample,
            image,
        }
    }

    pub fn describe(&self, binding: u32) -> WriteDescriptorSet {
        WriteDescriptorSet::image_view_sampler(binding, self.view.clone(), self.sample.clone())
    }
}

impl Texture<StorageImage> {
    pub(crate) fn load(path: &str, engine: &Engine) -> Self {
        let image = upload_image(path, engine.device(), engine.queue());

        Texture::new(image, engine.device())
    }
}

fn upload_image(filename: &str, device: Arc<Device>, queue: Arc<Queue>) -> Arc<StorageImage> {
    let img = image::io::Reader::open(filename).unwrap().decode().unwrap();

    let buf = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::transfer_source(),
        false,
        img.as_bytes().iter().map(|x| *x),
    )
    .expect("failed to create buffer");

    let image: Arc<StorageImage> = StorageImage::new(
        device.clone(),
        ImageDimensions::Dim2d {
            width: img.width(),
            height: img.height(),
            array_layers: 1, // images can be arrays of layers
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.family()),
    )
    .unwrap();

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit, // don't forget to write the correct buffer usage
    )
    .unwrap();

    builder
        .clear_color_image(image.clone(), ClearValue::Float([0.0, 0.0, 1.0, 1.0]))
        .unwrap()
        .copy_buffer_to_image(buf.clone(), image.clone()) // new
        .unwrap();

    let command_buffer = builder.build().unwrap();

    sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .flush()
        .unwrap();

    image
}
