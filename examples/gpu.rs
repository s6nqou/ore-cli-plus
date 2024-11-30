use solana_sdk::{
    keccak::{hashv, Hash},
    pubkey::Pubkey,
};
use std::{
    borrow::{BorrowMut, Cow},
    cell::{Cell, RefCell},
    str::FromStr,
    sync::Arc,
    time::Instant,
};
use wgpu::util::DeviceExt;

async fn run(input: &[u8; 64], difficulty: &[u8; 32]) -> Option<u64> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: {
                    let mut limits = wgpu::Limits::default();
                    limits.max_compute_workgroup_size_x = 512;
                    limits.max_compute_workgroup_size_y = 512;
                    limits.max_compute_invocations_per_workgroup = 512;
                    limits
                },
            },
            None,
        )
        .await
        .unwrap();
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("mine.wgsl"))),
    });
    // let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //     label: None,
    //     entries: &[
    //         wgpu::BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: wgpu::ShaderStages::COMPUTE,
    //             ty: wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Storage { read_only: false },
    //                 has_dynamic_offset: false,
    //                 min_binding_size: None,
    //             },
    //             count: None,
    //         },
    //         wgpu::BindGroupLayoutEntry {
    //             binding: 1,
    //             visibility: wgpu::ShaderStages::COMPUTE,
    //             ty: wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Storage { read_only: false },
    //                 has_dynamic_offset: false,
    //                 min_binding_size: None,
    //             },
    //             count: None,
    //         },
    //     ],
    // });
    // let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //     label: None,
    //     bind_group_layouts: &[&bind_group_layout],
    //     push_constant_ranges: &[wgpu::PushConstantRange {
    //         stages: wgpu::ShaderStages::COMPUTE,
    //         range: (0..104),
    //     }],
    // });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &cs_module,
        entry_point: "main",
    });
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 12,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut input_buffer: [u8; 16 * 4 + 2 * 4 + 8 * 4] = [0u8; 104];
    input_buffer[0..64].copy_from_slice(&bytemuck::cast_slice(input));
    input_buffer[72..104].copy_from_slice(&bytemuck::cast_slice(difficulty));
    let input_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(&input_buffer),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let found_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("found"),
        contents: bytemuck::cast_slice(&[0u32]),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
    });
    let nonce_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("nonce"),
        contents: bytemuck::cast_slice(&[0u32, 0u32]),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: found_storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: nonce_storage_buffer.as_entire_binding(),
            },
        ],
    });

    let (sender, receiver) = flume::unbounded();

    device.on_uncaptured_error(Box::new(|err| {
        println!("Error: {:?}", err);
    }));

    let mut times = 0;
    let workgroup_size = 16 * 16 * 2;
    let workgroup_count = 16 * 16 * 16 * 8;
    let nonce_step = workgroup_size * workgroup_count;
    for nonce1 in 0..u32::MAX {
        for nonce0 in (0..u32::MAX).step_by(nonce_step) {
            let now = Instant::now();
            times += 1;

            queue.write_buffer(&input_storage_buffer, 64, bytemuck::cast_slice(&[nonce0, nonce1]));
            queue.write_buffer(&found_storage_buffer, 0, bytemuck::cast_slice(&[0u32]));
            queue.write_buffer(
                &nonce_storage_buffer,
                0,
                bytemuck::cast_slice(&[0u32, 0u32]),
            );

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&found_storage_buffer, 0, &staging_buffer, 0, 4);
            encoder.copy_buffer_to_buffer(&nonce_storage_buffer, 0, &staging_buffer, 4, 8);

            queue.submit(Some(encoder.finish()));

            let staging_slice = staging_buffer.slice(..);
            let sender = sender.clone();
            staging_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

            device.poll(wgpu::Maintain::wait());
            println!("Pulled: {:?}", now.elapsed());

            if let Ok(result) = receiver.recv_async().await.unwrap() {
                let data = staging_slice.get_mapped_range();
                let bytes = data[4..].to_vec();
                let found = u32::from_le_bytes(data[..4].try_into().unwrap());
                let nonce = u64::from_le_bytes(data[4..].try_into().unwrap());

                println!("Times: {}", times);
                println!(
                    "Nonce: {}",
                    u64::from_le_bytes(bytemuck::cast_slice(&[nonce0, nonce1]).try_into().unwrap())
                );
                println!("Result: {:?}", result);
                println!("Bytes: {:?}", bytes);
                println!("Duration: {:?}", now.elapsed());

                if found > 0 {
                    return Some(nonce);
                }
            }

            staging_buffer.unmap();
        }
    }

    None
}

pub fn main() {
    env_logger::init();
    let content: [u8; 32] = Hash::from_str("11112edSRC7mDTWoWKAeHzMfwSzisJpsbEFcabjuNRj")
        .unwrap()
        .0;
    let pubkey = Pubkey::from_str("7DLZrjEsQ93KaqgX6s6d8pCVw2FAAQrK6oLMkm5Hx1cv")
        .unwrap()
        .to_bytes();
    let difficulty: [u8; 32] = [
        0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    ];

    let now = Instant::now();

    if let Some(nonce) = pollster::block_on(run(
        &[content, pubkey].concat().try_into().unwrap(),
        &difficulty,
    )) {
        let hash = hashv(&[&content, &pubkey, nonce.to_le_bytes().as_ref()]);
        println!("Hash: {:?}", hash);
        println!("Nonce: {}", nonce);
        println!("Hash bytes: {:?}", hash.0);
    }
    println!("Total duration: {:?}", now.elapsed());
}
