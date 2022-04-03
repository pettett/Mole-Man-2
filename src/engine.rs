use std::{
    collections::HashMap,
    iter::Map,
    marker::PhantomData,
    rc::{Rc, Weak},
    sync::{atomic::AtomicUsize, Arc},
};

use vulkano::{
    command_buffer::{
        pool::standard::StandardCommandPoolBuilder, AutoCommandBufferBuilder, CommandBufferUsage,
        SecondaryAutoCommandBuffer,
    },
    descriptor_set::WriteDescriptorSet,
    device::{
        physical::{PhysicalDevice, QueueFamily},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{compute, get_physical, gl, material::Material};

static NEXT_MATERIAL_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatID(usize);

pub fn get_instance() -> Arc<Instance> {
    let required_extensions = vulkano_win::required_extensions();

    Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .expect("failed to create instance")
}

pub struct Engine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    viewport: Viewport,
    render_pass: Pass,
    chain: Chain,
    surface: Arc<Surface<Window>>,

    materials: HashMap<MatID, Material>,
}
impl Engine {
    pub fn init(
        instance: Arc<Instance>,
        physical_device: &PhysicalDevice,
        graphics_queue: &QueueFamily,
        surface: Arc<Surface<Window>>,
        device_extensions: &DeviceExtensions,
    ) -> Self {
        //But initialization isn't finished yet. Before being able to do anything, we have to create a device.
        //A device is an object that represents an open channel of communication with a physical device, and it is
        //probably the most important object of the Vulkan API.

        for family in physical_device.queue_families() {
            println!(
                "Found a queue family with {:?} queue(s)  [C:{:?},G:{:?},T:{:?}]",
                family.queues_count(),
                family.supports_compute(),
                family.supports_graphics(),             //supports vkDraw
                family.explicitly_supports_transfers(), //all queues can do this, but one does it better if some have this set as false
            );
        }

        //Now that we have our desired physical device, the next step is to create a logical device that can support the swapchain.

        //Creating a device returns two things:
        //- the device itself,
        //- a list of queue objects that will later allow us to submit operations.

        //Once this function call succeeds we have an open channel of communication with a Vulkan device!

        let (device, mut queues) = Device::new(
            *physical_device,
            DeviceCreateInfo {
                // here we pass the desired queue families that we want to use
                queue_create_infos: vec![QueueCreateInfo::family(*graphics_queue)],
                enabled_extensions: physical_device
                    .required_extensions()
                    .union(&device_extensions), // new
                //and everything else is set to default
                ..DeviceCreateInfo::default()
            },
        )
        .expect("failed to create device");

        //  caps.min_image_count - normally 1, but all of these are effectively internal, so

        //Since it is possible to request multiple queues, the queues variable returned by the function is in fact an iterator.
        //In this example code this iterator contains just one element, so let's extract it:

        //Arc is Atomic RC, reference counted box
        let queue: Arc<Queue> = queues.next().unwrap();

        //When using Vulkan, you will very often need the GPU to read or write data in memory.
        //In fact there isn't much point in using the GPU otherwise,
        //as there is nothing you can do with the results of its work except write them to memory.

        //In order for the GPU to be able to access some data
        //	(either for reading, writing or both),
        //	we first need to create a buffer object and put the data in it.

        //The most simple kind of buffer that exists is the `CpuAccessibleBuffer`, which can be created like this:

        // let data: i32 = 12;
        // let buffer = CpuAccessibleBuffer::from_data(
        //     device.clone(), //acutally just cloning the arc<>
        //     BufferUsage::all(),
        //     false,
        //     data,
        // )
        // .expect("failed to create buffer");

        //The second parameter indicates which purpose we are creating the buffer for,
        //which can help the implementation perform some optimizations.
        //Trying to use a buffer in a way that wasn't indicated in its constructor will result in an error.
        //For the sake of the example, we just create a BufferUsage that allows all possible usages.

        // gl::copy_between_buffers(&device, &queue);

        // compute::perform_compute(&device, &queue);

        let chain = Chain::new(device.clone(), &physical_device, surface.clone());

        let render_pass = Pass::new(&chain, device.clone());

        Self {
            device,
            queue,
            viewport: Viewport {
                origin: [0.0, 0.0],
                dimensions: surface.window().inner_size().into(),
                depth_range: 0.0..1.0,
            },
            surface,
            render_pass,
            chain,
            materials: HashMap::new(),
        }
    }

    pub fn surface(&self) -> Arc<Surface<Window>> {
        self.surface.clone()
    }

    pub fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn render_pass(&self) -> &Pass {
        &self.render_pass
    }

    pub fn swapchain(&self) -> &Chain {
        &self.chain
    }

    pub fn create_secondary(
        &self,
        usage: CommandBufferUsage,
        subpass: Subpass,
    ) -> AutoCommandBufferBuilder<SecondaryAutoCommandBuffer, StandardCommandPoolBuilder> {
        AutoCommandBufferBuilder::secondary_graphics(
            self.device(),
            self.queue().family(),
            usage,
            subpass,
        )
        .unwrap()
    }

    pub fn recreate_swapchain(&mut self) -> Result<PhysicalSize<u32>, ()> {
        let new_dimensions = self.surface.window().inner_size();

        let (new_swapchain, new_images) = match self.chain.swapchain.recreate(SwapchainCreateInfo {
            image_extent: new_dimensions.into(), // here, "image_extend" will correspond to the window dimensions
            ..self.chain.swapchain.create_info()
        }) {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return Err(()),
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };
        self.chain.swapchain = new_swapchain;
        self.render_pass.framebuffers =
            gl::get_framebuffers(&new_images, self.render_pass().render_pass());

        Ok(new_dimensions)
    }

    pub fn create_material(
        &mut self,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        descriptor_wites: impl IntoIterator<Item = WriteDescriptorSet>,
    ) -> MatID {
        let mat = Material::new(vs, fs, descriptor_wites, self);

        let id = MatID(NEXT_MATERIAL_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed));

        self.materials.insert(id, mat);
        //safe to unwrap here as we just pushed a new element
        id
    }

    pub fn get_material(&self, id: &MatID) -> &Material {
        &self.materials[id]
    }

    pub fn update_viewport(&mut self, new_dimensions: [f32; 2]) {
        self.viewport.dimensions = new_dimensions;

        let device = self.device();
        let pass = self.render_pass().render_pass();
        let viewport = self.viewport().clone();

        // every material's pipeline is now invalid; fix all of them
        for m in self.materials.values_mut() {
            m.pipeline = gl::get_pipeline(
                device.clone(),
                m.vs.clone(),
                m.fs.clone(),
                pass.clone(),
                viewport.clone(),
            )
        }
    }
}

pub struct Chain {
    swapchain: Arc<Swapchain<Window>>,
    images: Vec<Arc<SwapchainImage<Window>>>,
}
impl Chain {
    pub fn swapchain(&self) -> Arc<Swapchain<Window>> {
        self.swapchain.clone()
    }

    pub fn new(
        device: Arc<Device>,
        physical_device: &PhysicalDevice,
        surface: Arc<Surface<Window>>,
    ) -> Self {
        let caps = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        // this size of the swapchain images
        let dimensions = surface.window().inner_size();
        let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::color_attachment(), // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        )
        .unwrap();

        Self { swapchain, images }
    }
}

pub struct Pass {
    render_pass: Arc<RenderPass>,
    //the swapchain is dependant on the render pass, and contains a set of windows that could be drawn to
    framebuffers: Vec<Arc<Framebuffer>>,
}
impl Pass {
    pub fn new(chain: &Chain, device: Arc<Device>) -> Self {
        let render_pass = gl::get_render_pass(device, chain.swapchain());
        let framebuffers = gl::get_framebuffers(&chain.images, render_pass.clone());
        //create the render pass and buffers
        Self {
            render_pass,
            framebuffers,
        }
    }
    pub fn render_pass(&self) -> Arc<RenderPass> {
        self.render_pass.clone()
    }

    pub fn get_frame(&self, id: usize) -> Arc<Framebuffer> {
        self.framebuffers[id].clone()
    }
}
