pub mod compute;
pub mod gl;
pub mod imgui_vulkano_renderer;
pub mod texture;
pub mod uniform;

use std::ops::Mul;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use imgui_vulkano_renderer::Renderer;
use vulkano::buffer::TypedBufferAccess;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::descriptor_set::{
    DescriptorSet, DescriptorSetsCollection, PersistentDescriptorSet, WriteDescriptorSet,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily};
use vulkano::device::DeviceExtensions;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::format::{ClearValue, Format};
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{ImageAspects, ImageDimensions, ImageUsage, StorageImage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{ComputePipeline, GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    self, AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
mod clipboard;
use imgui::{self, Image};

use crate::texture::Texture;

struct Chain {
    swapchain: Arc<Swapchain<Window>>,
    images: Vec<Arc<SwapchainImage<Window>>>,
}
impl Chain {
    fn swapchain(&self) -> Arc<Swapchain<Window>> {
        self.swapchain.clone()
    }

    fn new(
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

        let (mut swapchain, images) = Swapchain::new(
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

struct Pass {
    render_pass: Arc<RenderPass>,
    //the swapchain is dependant on the render pass, and contains a set of windows that could be drawn to
    framebuffers: Vec<Arc<Framebuffer>>,
}
impl Pass {
    fn new(chain: &Chain, device: Arc<Device>) -> Self {
        let render_pass = gl::get_render_pass(device, chain.swapchain());
        let framebuffers = gl::get_framebuffers(&chain.images, render_pass.clone());
        //create the render pass and buffers
        Self {
            render_pass,
            framebuffers,
        }
    }
}

struct Engine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    viewport: Viewport,
    render_pass: Pass,
    instance: Arc<Instance>,
    chain: Chain,
}
impl Engine {
    fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
    fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    fn viewport(&self) -> Viewport {
        self.viewport.clone()
    }

    fn render_pass(&self) -> Arc<RenderPass> {
        self.render_pass.render_pass.clone()
    }
}

struct Material {
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    descriptors: Arc<PersistentDescriptorSet>,

    pipeline: Arc<GraphicsPipeline>,
}

impl Material {
    fn new(
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        descriptor_wites: impl IntoIterator<Item = WriteDescriptorSet>,
        engine: &Engine,
    ) -> Self {
        let pipeline = gl::get_pipeline(
            engine.device(),
            vs.clone(),
            fs.clone(),
            engine.render_pass(),
            engine.viewport(),
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

    fn update_pipeline(&mut self, engine: &Engine) {
        self.pipeline = gl::get_pipeline(
            engine.device(),
            self.vs.clone(),
            self.fs.clone(),
            engine.render_pass(),
            engine.viewport(),
        )
    }

    fn descriptors(&self) -> Arc<PersistentDescriptorSet> {
        self.descriptors.clone()
    }

    fn pipeline(&self) -> &Arc<GraphicsPipeline> {
        &self.pipeline
    }
}

fn get_physical<'a>(
    instance: &'a Arc<Instance>,
    device_extensions: DeviceExtensions,
    surface: &Surface<Window>,
) -> (PhysicalDevice<'a>, QueueFamily<'a>) {
    // pick the best physical device and queue1
    PhysicalDevice::enumerate(instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                // Find the first first queue family that is suitable.
                // If none is found, `None` is returned to `filter_map`,
                // which disqualifies this physical device.
                .find(|&q| q.supports_graphics() && q.supports_surface(surface).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .expect("no device available")
}

fn main() {
    println!("Hello, world!");

    let required_extensions = vulkano_win::required_extensions();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .expect("failed to create instance");

    let event_loop = EventLoop::new(); // ignore this for now
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    //In the previous section we created an instance and chose a physical device from this instance.
    let (physical_device, graphics_queue) = get_physical(&instance, device_extensions, &surface);
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
        physical_device,
        DeviceCreateInfo {
            // here we pass the desired queue families that we want to use
            queue_create_infos: vec![QueueCreateInfo::family(graphics_queue)],
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

    gl::copy_between_buffers(&device, &queue);

    compute::perform_compute(&device, &queue);

    let chain = Chain::new(device.clone(), &physical_device, surface.clone());

    let render_pass = Pass::new(&chain, device.clone());

    let mut engine = Engine {
        device,
        instance: instance.clone(),
        queue,
        viewport: Viewport {
            origin: [0.0, 0.0],
            dimensions: surface.window().inner_size().into(),
            depth_range: 0.0..1.0,
        },
        render_pass,
        chain,
    };

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
        color: [1., 0., 0.],
    };

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        engine.device(),
        BufferUsage::vertex_buffer(),
        false,
        vec![vertex1, vertex2, vertex3, vertex4].into_iter(),
    )
    .unwrap();

    let index_buffer = CpuAccessibleBuffer::from_iter(
        engine.device(),
        BufferUsage::index_buffer(),
        false,
        vec![0u32, 1u32, 2u32, 2u32, 0u32, 3u32].into_iter(),
    )
    .unwrap();

    //let vs = vs::load(device.clone()).unwrap();
    let vs_texture = vs_texture::load(engine.device()).unwrap();
    //    let fs = fs::load(device.clone()).unwrap();
    let fs_texture = fs_texture::load(engine.device()).unwrap();

    let mut tile_positions = [[1f32, 1f32], [1f32, 1f32], [1f32, 1f32]];

    let uniform_data_buffer =
        CpuAccessibleBuffer::from_iter(engine.device(), BufferUsage::all(), false, tile_positions)
            .expect("failed to create buffer");

    // let mut command_buffers = gl::get_draw_command_buffers(
    //     device.clone(),
    //     queue.clone(),
    //     pipeline.clone(),
    //     &framebuffers,
    //     vertex_buffer.clone(),
    //     index_buffer.clone(),
    //     uniform_set.clone(),
    // );

    let mut window_resized = false;
    let mut recreate_swapchain = false;

    let mut t = 0f32;

    let mut transform = uniform::Transformations::new(engine.device());

    let w_s = transform.transform();

    let screen_size = surface.window().inner_size();

    let aspect = screen_size.width as f32 / screen_size.height as f32;

    *w_s = glm::mat4(
        1.,
        0.,
        0.,
        0., //
        0.,
        1. * aspect,
        0.,
        0., //
        0.,
        0.,
        1.,
        0., //
        0.,
        0.,
        0.,
        1., //
    );

    transform.update_buffer();

    let cobblestone = Texture::load("assets/cobblestone.png", &engine);

    let mut mat_texture = Material::new(
        vs_texture,
        fs_texture,
        [
            // 0 is the binding in GLSL when we use this set
            WriteDescriptorSet::buffer(0, transform.get_buffer()),
            WriteDescriptorSet::buffer(1, uniform_data_buffer.clone()),
            cobblestone.describe(3),
        ],
        &engine,
    );

    let spite_sheet = Texture::load("assets/tileset.png", &engine);

    let mut dragging = false;

    let mut last_mouse_pos: Option<PhysicalPosition<f64>> = None;

    // Example with default allocator
    // IMGUI BS
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    if let Some(backend) = clipboard::init() {
        imgui.set_clipboard_backend(backend);
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        surface.window(),
        imgui_winit_support::HiDpiMode::Rounded,
    );

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: font_size,
                ..imgui::FontConfig::default()
            }),
        }]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    let format = engine.chain.swapchain.image_format();
    let mut renderer = Renderer::init(&mut imgui, engine.device(), engine.queue(), format)
        .expect("Failed to initialize renderer");

    let ui_tex = renderer.make_ui_texture(spite_sheet.clone());
    let id = renderer.textures().insert(ui_tex);

    event_loop.run(move |event, _, control_flow| {
        platform.handle_event(imgui.io_mut(), surface.window(), &event);

        match event {
            Event::RedrawEventsCleared => {
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;

                    let new_dimensions = surface.window().inner_size();

                    let (new_swapchain, new_images) =
                        match engine.chain.swapchain.recreate(SwapchainCreateInfo {
                            image_extent: new_dimensions.into(), // here, "image_extend" will correspond to the window dimensions
                            ..engine.chain.swapchain.create_info()
                        }) {
                            Ok(r) => r,
                            // This error tends to happen when the user is manually resizing the window.
                            // Simply restarting the loop is the easiest way to fix this issue.
                            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };
                    engine.chain.swapchain = new_swapchain;
                    engine.render_pass.framebuffers =
                        gl::get_framebuffers(&new_images, engine.render_pass().clone());

                    if window_resized {
                        window_resized = false;

                        engine.viewport.dimensions = new_dimensions.into();

                        mat_texture.update_pipeline(&engine);
                        // command_buffers = gl::get_draw_command_buffers(
                        //     device.clone(),
                        //     queue.clone(),
                        //     pipeline.clone(),
                        //     &new_framebuffers,
                        //     vertex_buffer.clone(),
                        //     index_buffer.clone(),
                        //     uniform_set.clone(),
                        // );
                    }
                }
                //To actually start drawing, the first thing that we need to do is to acquire an image to draw:
                let (image_i, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(engine.chain.swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                platform
                    .prepare_frame(imgui.io_mut(), surface.window())
                    .unwrap();

                let ui = imgui.frame();

                imgui::Window::new("Hello world")
                    .size([300.0, 110.0], imgui::Condition::FirstUseEver)
                    .build(&ui, || {
                        ui.text("Hello world!");
                        ui.text("こんにちは世界！");
                        ui.text("This...is...imgui-rs!");
                        ui.separator();
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0], mouse_pos[1]
                        ));

                        let [x, y] = spite_sheet.get_size();

                        Image::new(id, [x as f32, y as f32]).build(&ui);
                    });

                platform.prepare_render(&ui, surface.window());

                let draw_data = ui.render();

                let framebuffer = &engine.render_pass.framebuffers[image_i];

                let cmd_buffer = {
                    //build the command buffer
                    let mut builder = AutoCommandBufferBuilder::primary(
                        engine.device(),
                        engine.queue().family(),
                        CommandBufferUsage::OneTimeSubmit, // don't forget to write the correct buffer usage
                    )
                    .unwrap();

                    // begin render pass
                    builder
                        .begin_render_pass(
                            framebuffer.clone(),
                            SubpassContents::Inline,
                            vec![[0.0, 0.0, 0.0, 1.0].into()],
                        )
                        .unwrap();

                    //render pass started, can now issue draw instructions
                    builder
                        .bind_pipeline_graphics(mat_texture.pipeline().clone())
                        .bind_index_buffer(index_buffer.clone())
                        .bind_vertex_buffers(0, vertex_buffer.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            mat_texture.pipeline().layout().clone(),
                            0,
                            mat_texture.descriptors(),
                        )
                        .draw_indexed(index_buffer.len() as u32, 3, 0, 0, 0)
                        .unwrap();

                    renderer
                        .draw_commands(
                            &mut builder,
                            engine.queue(),
                            engine.viewport().dimensions,
                            draw_data,
                        )
                        .unwrap();

                    //finish off
                    builder.end_render_pass().unwrap();

                    //return the created command buffer
                    builder.build().unwrap()
                };

                let mut i = 0f32;
                for p in &mut tile_positions[1..] {
                    *p = [(t + i).cos(), (t + i).sin()];
                    i += 1.;
                }

                {
                    //update buffer data
                    let mut w = uniform_data_buffer.write().expect("failed to write buffer");

                    for (i, p) in tile_positions.iter().enumerate() {
                        w[i] = *p;
                    }
                }

                //create the future to execute our command buffer
                let cmd_future = sync::now(engine.device())
                    .join(acquire_future)
                    .then_execute(engine.queue(), cmd_buffer)
                    .unwrap();

                //fence is from GPU -> CPU sync, semaphore is GPU to GPU.

                let execution = cmd_future
                    .then_swapchain_present(
                        engine.queue().clone(),
                        engine.chain.swapchain.clone(),
                        image_i,
                    )
                    .then_signal_fence_and_flush();

                match execution {
                    Ok(future) => {
                        future.wait(None).unwrap(); // wait for the GPU to finish
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                    }
                }

                t += 0.02;
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } if dragging => {
                if let Some(last_pos) = last_mouse_pos {
                    let diff_x = ((position.x - last_pos.x) as f32) * 2. / screen_size.width as f32;
                    let diff_y =
                        ((position.y - last_pos.y) as f32) * 2. / screen_size.height as f32;

                    transform.transform().c0.w += diff_x;
                    transform.transform().c1.w += diff_y;

                    transform.update_buffer();
                }

                last_mouse_pos = Some(position);
            }

            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    },
                ..
            } => {
                dragging = state == ElementState::Pressed;

                if !dragging {
                    last_mouse_pos = None;
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                window_resized = true;
            }
            Event::MainEventsCleared => {}
            _ => (),
        }
    });
}

// mod vs {
//     vulkano_shaders::shader! {
//         ty: "vertex",
//         src: "
// #version 450

// layout(location = 0) in vec2 position;
// layout(location = 1) in vec3 color;

// layout(location = 0) out vec3 fragColor;

// layout(binding = 0) uniform Transforms{
// 	mat4 world_to_screen;
// };

// layout(binding = 1 ) buffer UniformBufferObject {
// 	vec2 offset[];
// };

// void main() {
// 	fragColor = color;
//     gl_Position = vec4(position + offset[gl_InstanceIndex] + 1 , 0.0, 1.0) * world_to_screen;
// }"
//     }
// }

// mod fs {
//     vulkano_shaders::shader! {
//         ty: "fragment",
//         src: "
// #version 450

// layout(location = 0) in vec3 color;

// layout(location = 0) out vec4 f_color;

// void main() {
//     f_color = vec4(color.rgb, 1.0);
// }"
//     }
// }

mod vs_texture {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;


layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 uv;


layout(binding = 0) uniform Transforms{
	mat4 world_to_screen;
};

layout(binding = 1 ) buffer UniformBufferObject {
	vec2 offset[];
};


void main() {
	uv = position.xy;
	fragColor = color;
    gl_Position = vec4(position + offset[gl_InstanceIndex] , 0.0, 1.0) * world_to_screen;
}"
    }
}

mod fs_texture {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450


layout(location = 0) in vec3 color;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 f_color;


layout(binding = 3) uniform sampler2D texSampler;


void main() {
    f_color = vec4(color.rgb , 1.0) *  texture(texSampler, uv);
}"
    }
}
