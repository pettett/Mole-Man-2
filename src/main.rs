use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo};
use vulkano::instance::{Instance, InstanceCreateInfo};

use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::swapchain::Surface;
use vulkano::sync::{self, GpuFuture};

fn main() {
    println!("Hello, world!");

    let instance = Instance::new(InstanceCreateInfo::default()).expect("failed to create instance");

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no device available");

    //In the previous section we created an instance and chose a physical device from this instance.

    //But initialization isn't finished yet. Before being able to do anything, we have to create a device.
    //A device is an object that represents an open channel of communication with a physical device, and it is
    //probably the most important object of the Vulkan API.

    for family in physical.queue_families() {
        println!(
            "Found a queue family with {:?} queue(s)  [C:{:?},G:{:?},T:{:?}]",
            family.queues_count(),
            family.supports_compute(),
            family.supports_graphics(),             //supports vkDraw
            family.explicitly_supports_transfers(), //all queues can do this, but one does it better if some have this set as false
        );
    }

    let graphics_queue = physical
        .queue_families()
        .find(|q| q.supports_graphics())
        .expect("Could not find graphics queue");

    //Creating a device returns two things:
    //- the device itself,
    //- a list of queue objects that will later allow us to submit operations.

    //Once this function call succeeds we have an open channel of communication with a Vulkan device!

    let (device, mut queues) = Device::new(
        physical,
        DeviceCreateInfo {
            // here we pass the desired queue families that we want to use
            queue_create_infos: vec![QueueCreateInfo::family(graphics_queue)],

            //and everything else is set to default
            ..DeviceCreateInfo::default()
        },
    )
    .expect("failed to create device");

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

    copy_between_buffers(&device, &queue);

    perform_compute(&device, &queue);
}

fn copy_between_buffers(device: &Arc<Device>, queue: &Arc<Queue>) {
    let source_content: Vec<i32> = (0..64).collect();
    let source =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content)
            .expect("failed to create buffer");

    let destination_content: Vec<i32> = (0..64).map(|_| 0).collect();
    let destination = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        destination_content,
    )
    .expect("failed to create buffer");

    //Vulkan supports primary and secondary command buffers.
    //Primary command buffers can be sent directly to the GPU
    //while secondary command buffers allow you to store functionality that you can reuse multiple times in primary command buffers.
    //We won't cover secondary command buffers here, but you can read more about them.

    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    builder
        .copy_buffer(source.clone(), destination.clone())
        .unwrap();

    let command_buffer = builder.build().unwrap();

    //In order to read the content of destination and make sure that our copy succeeded,
    //we need to wait until the operation is complete. To do that,
    //we need to program the GPU to send back a special signal that will make us know it has finished.
    //This kind of signal is called a fence, and it lets us know whenever the GPU has reached a certain point of execution.
    run_commands_and_wait(device, queue, command_buffer);

    let src_content = source.read().unwrap();
    let destination_content = destination.read().unwrap();
    assert_eq!(&*src_content, &*destination_content);
    println!("Successfully copied two buffers on the GPU");
}

fn perform_compute(device: &Arc<Device>, queue: &Arc<Queue>) {
    let starting_nums = 0..65536u32;
    let data_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, starting_nums)
            .expect("failed to create buffer");

    let shader = cs::load(device.clone()).expect("failed to create shader module");

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        shader.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    //we are creating the layout for set 0
    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();
    let set = PersistentDescriptorSet::new(
        layout.clone(),
        [WriteDescriptorSet::buffer(0, data_buffer.clone())], // 0 is the binding in GLSL when we use this set
    )
    .unwrap();

    // so now we have set=0, binding=0

    // create an "empty" command buffer for a single use on this queue
    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    // create the commands
    //1. bind the pipeline, which contains out compute buffer, and so contains the bindings to the buffer we made
    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0, // 0 is the index of our set
            set,
        )
        .dispatch([1024, 1, 1])
        .unwrap();

    let command_buffer = builder.build().unwrap();

    //now we need to run the buffer as before
    run_commands_and_wait(device, queue, command_buffer);

    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        assert_eq!(*val, n as u32 * 12);
    }

    println!("Everything succeeded!");
}

fn run_commands_and_wait(
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    command_buffer: PrimaryAutoCommandBuffer,
) {
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush() // same as signal fence, and then flush
        .unwrap();

    future.wait(None).unwrap(); // None is an optional timeout
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}"
    }
}
