pub mod compute;
pub mod engine;
pub mod gl;
pub mod imgui_vulkano_renderer;
pub mod material;
pub mod texture;
pub mod uniform;

use std::sync::Arc;

use imgui_vulkano_renderer::Renderer;
use vulkano::buffer::TypedBufferAccess;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents};
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType, QueueFamily};
use vulkano::device::DeviceExtensions;

use vulkano::instance::Instance;

use vulkano::pipeline::{Pipeline, PipelineBindPoint};

use vulkano::swapchain::{self, AcquireError, Surface};
use vulkano::sync::{self, FlushError, GpuFuture};

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;
mod clipboard;
use imgui::{self, Image};

use crate::texture::Texture;

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

    let (mut engine, event_loop) = engine::Engine::init();

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

    let screen_size = engine.surface().window().inner_size();

    let aspect = screen_size.width as f32 / screen_size.height as f32;

    *w_s = glm::mat4(
        0.1,
        0.,
        0.,
        0., //
        0.,
        0.1 * aspect,
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

    let mat_texture = engine.create_material(
        vs_texture,
        fs_texture,
        [
            // 0 is the binding in GLSL when we use this set
            WriteDescriptorSet::buffer(0, transform.get_buffer()),
            WriteDescriptorSet::buffer(1, uniform_data_buffer.clone()),
            cobblestone.describe(3),
        ],
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
        engine.surface().window(),
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

    let format = engine.swapchain().swapchain().image_format();

    let mut renderer = Renderer::init(&mut imgui, engine.device(), engine.queue(), format)
        .expect("Failed to initialize renderer");

    let ui_tex = renderer.make_ui_texture(spite_sheet.clone());
    let id = renderer.textures().insert(ui_tex);

    event_loop.run(move |event, _, control_flow| {
        platform.handle_event(imgui.io_mut(), engine.surface().window(), &event);

        match event {
            Event::RedrawEventsCleared => {
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;

                    // recreate the swapchain. this *may* result in a new sized image, in this case also update the viewport
                    let new_dimensions = match engine.recreate_swapchain() {
                        Err(()) => return,
                        Ok(new_dimensions) => new_dimensions,
                    };

                    if window_resized {
                        window_resized = false;

                        engine.update_viewport(new_dimensions.into());

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
                    match swapchain::acquire_next_image(engine.swapchain().swapchain(), None) {
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
                    .prepare_frame(imgui.io_mut(), engine.surface().window())
                    .unwrap();

                let ui = imgui.frame();

                imgui::Window::new("Hello world")
                    .size([300.0, 110.0], imgui::Condition::FirstUseEver)
                    .build(&ui, || {
                        ui.text("Hello world!");
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

                platform.prepare_render(&ui, engine.surface().window());

                let draw_data = ui.render();

                let framebuffer = &engine.render_pass().get_frame(image_i);

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

                    let m = engine.get_material(&mat_texture);

                    //render pass started, can now issue draw instructions
                    builder
                        .bind_pipeline_graphics(m.pipeline.clone())
                        .bind_index_buffer(index_buffer.clone())
                        .bind_vertex_buffers(0, vertex_buffer.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            m.pipeline.layout().clone(),
                            0,
                            m.descriptors(),
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
                        engine.swapchain().swapchain(),
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
