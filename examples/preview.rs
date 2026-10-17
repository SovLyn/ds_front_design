//! Offline renderer used to inspect the procedural scenes without a browser.
//!
//!   cargo run --release --example preview -- 1600 900
//!
//! Writes `preview/scene-XX.png` next to the project root. This is a
//! development tool only; the shipped application renders through eframe and
//! the egui paint callback.

use std::path::PathBuf;

use eframe::wgpu;

use deep_space_atlas::{
    render::{SceneFrame, SceneRenderer},
    scenes::SCENES,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let width: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1600);
    let height: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(900);
    let only: Option<usize> = args.get(3).and_then(|s| s.parse().ok());

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .expect("no suitable GPU adapter");

    let info = adapter.get_info();
    eprintln!(
        "preview adapter: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("atlas preview device"),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("failed to create device");

    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        eprintln!("wgpu error: {error}");
    }));

    let mut renderer = SceneRenderer::new(&device, FORMAT);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("preview output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());

    let out_dir = PathBuf::from("preview");
    std::fs::create_dir_all(&out_dir).expect("could not create preview directory");

    for (index, spec) in SCENES.iter().enumerate() {
        if let Some(only) = only
            && only != index
        {
            continue;
        }

        let time = 21.5 + index as f32 * 0.7;
        let frame = SceneFrame {
            time,
            mode_a: spec.id.mode(),
            mode_b: spec.id.mode(),
            blend: 0.0,
            color_a: spec.color,
            color_b: spec.color,
            energy_a: spec.energy,
            energy_b: spec.energy,
            accent_a: spec.accent,
            accent_b: spec.accent,
            accent_energy_a: spec.accent_energy,
            accent_energy_b: spec.accent_energy,
            camera_a: spec.camera.drift(time, spec.drift),
            camera_b: spec.camera.drift(time, spec.drift),
            phase_a: spec.phase,
            phase_b: spec.phase,
            pointer: [0.0, 0.0],
            motion: 1.0,
            quality: 1.0,
            reduced_motion: false,
            veil: 0.0,
            boot: 1.0,
            composition: [0.64, 0.5],
            mobile: false,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("preview encoder"),
        });
        renderer.prepare(&device, &queue, &mut encoder, [width, height], &frame, 1.0);

        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("preview composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &output_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            renderer.paint(&mut pass);
        }

        let unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("preview readback"),
            size: (padded as u64) * (height as u64),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll failed");
        rx.recv().expect("map channel closed").expect("map failed");

        let data = slice.get_mapped_range().expect("mapped range unavailable");
        let mut image = image::RgbaImage::new(width, height);
        for y in 0..height {
            let start = (y * padded) as usize;
            let row = &data[start..start + unpadded as usize];
            for x in 0..width {
                let i = (x * 4) as usize;
                image.put_pixel(x, y, image::Rgba([row[i], row[i + 1], row[i + 2], 255]));
            }
        }
        drop(data);
        buffer.unmap();

        let path = out_dir.join(format!("scene-{index:02}.png"));
        image.save(&path).expect("failed to write preview");
        eprintln!("wrote {}", path.display());
    }
}
