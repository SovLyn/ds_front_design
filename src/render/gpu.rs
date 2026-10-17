//! WebGPU background renderer.
//!
//! One fullscreen fragment shader draws the scene into an HDR texture, a
//! cheap bloom chain softens the highlights, and a final pass tone-maps the
//! result into egui's render pass. The pipeline is deliberately independent
//! from the UI: adding a page means adding a uniform mode, not new buffers.

use eframe::egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu;
use egui::PaintCallbackInfo;

use crate::render::uniform::SceneFrame;

const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const SHADER: &str = include_str!("shaders/atlas.wgsl");
const BLUR_DIVISOR: u32 = 4;

pub struct SceneRenderer {
    target_format: wgpu::TextureFormat,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    scene_pipeline: wgpu::RenderPipeline,
    down_pipeline: wgpu::RenderPipeline,
    blur_h_pipeline: wgpu::RenderPipeline,
    blur_v_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    post_bind_group_layout: wgpu::BindGroupLayout,
    targets: Option<Targets>,
}

struct Targets {
    size: [u32; 2],
    bloom_size: [u32; 2],
    scene_view: wgpu::TextureView,
    bloom_a_view: wgpu::TextureView,
    bloom_b_view: wgpu::TextureView,
    down_bind_group: wgpu::BindGroup,
    blur_h_bind_group: wgpu::BindGroup,
    blur_v_bind_group: wgpu::BindGroup,
    composite_bind_group: wgpu::BindGroup,
}

impl SceneRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("atlas scene shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let scene_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas scene bind group layout"),
                entries: &[uniform],
            });

        let texture = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let texture_2 = wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler = wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let post_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas post bind group layout"),
                entries: &[uniform, texture, texture_2, sampler],
            });

        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("atlas scene layout"),
            bind_group_layouts: &[Some(&scene_bind_group_layout)],
            immediate_size: 0,
        });
        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("atlas post layout"),
            bind_group_layouts: &[Some(&post_bind_group_layout)],
            immediate_size: 0,
        });

        let scene_pipeline = make_pipeline(
            device,
            &shader,
            &scene_layout,
            "fs_scene",
            SCENE_FORMAT,
            "atlas scene pipeline",
        );
        let down_pipeline = make_pipeline(
            device,
            &shader,
            &post_layout,
            "fs_down",
            SCENE_FORMAT,
            "atlas bloom downsample",
        );
        let blur_h_pipeline = make_pipeline(
            device,
            &shader,
            &post_layout,
            "fs_blur_h",
            SCENE_FORMAT,
            "atlas bloom blur horizontal",
        );
        let blur_v_pipeline = make_pipeline(
            device,
            &shader,
            &post_layout,
            "fs_blur_v",
            SCENE_FORMAT,
            "atlas bloom blur vertical",
        );
        let composite_pipeline = make_pipeline(
            device,
            &shader,
            &post_layout,
            "fs_composite",
            target_format,
            "atlas composite",
        );

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas frame uniform"),
            size: std::mem::size_of::<crate::render::uniform::FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas uniform bind group"),
            layout: &scene_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self {
            target_format,
            uniform_buffer,
            uniform_bind_group,
            sampler,
            scene_pipeline,
            down_pipeline,
            blur_h_pipeline,
            blur_v_pipeline,
            composite_pipeline,
            post_bind_group_layout,
            targets: None,
        }
    }

    /// Encode scene + bloom. Must run before egui's own render pass.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_size: [u32; 2],
        frame: &SceneFrame,
        render_scale: f32,
    ) {
        let [out_w, out_h] = output_size;
        if out_w == 0 || out_h == 0 {
            return;
        }

        let scale = render_scale.clamp(0.3, 1.0);
        let scene_w = ((out_w as f32 * scale).round() as u32).clamp(64, out_w.max(64));
        let scene_h = ((out_h as f32 * scale).round() as u32).clamp(64, out_h.max(64));
        let bloom_w = (scene_w / BLUR_DIVISOR).max(8);
        let bloom_h = (scene_h / BLUR_DIVISOR).max(8);

        self.ensure_targets(device, [scene_w, scene_h], [bloom_w, bloom_h]);

        let uniform = frame.uniform(
            [scene_w, scene_h],
            [bloom_w, bloom_h],
            output_size,
            !self.target_format.is_srgb(),
        );
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let Some(targets) = &self.targets else {
            return;
        };

        // Scene -> HDR target.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.scene_view,
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
            });
            pass.set_pipeline(&self.scene_pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Bright pass + half-resolution blur.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas bloom downsample"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.bloom_a_view,
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
            });
            pass.set_pipeline(&self.down_pipeline);
            pass.set_bind_group(0, &targets.down_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas bloom blur h"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.bloom_b_view,
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
            });
            pass.set_pipeline(&self.blur_h_pipeline);
            pass.set_bind_group(0, &targets.blur_h_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas bloom blur v"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.bloom_a_view,
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
            });
            pass.set_pipeline(&self.blur_v_pipeline);
            pass.set_bind_group(0, &targets.blur_v_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Draw the composited background into egui's render pass.
    pub fn paint(&self, pass: &mut wgpu::RenderPass<'static>) {
        let Some(targets) = &self.targets else {
            return;
        };
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &targets.composite_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn scene_size(&self) -> Option<[u32; 2]> {
        self.targets.as_ref().map(|t| t.size)
    }

    fn ensure_targets(
        &mut self,
        device: &wgpu::Device,
        scene_size: [u32; 2],
        bloom_size: [u32; 2],
    ) {
        if let Some(targets) = &self.targets
            && targets.size == scene_size
            && targets.bloom_size == bloom_size
        {
            return;
        }

        let scene = create_texture(device, "atlas scene target", scene_size, SCENE_FORMAT);
        let bloom_a = create_texture(device, "atlas bloom target a", bloom_size, SCENE_FORMAT);
        let bloom_b = create_texture(device, "atlas bloom target b", bloom_size, SCENE_FORMAT);
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom_a_view = bloom_a.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom_b_view = bloom_b.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = |label: &str, src: &wgpu::TextureView, second: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &self.post_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(second),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };

        let down_bind_group = bind_group("atlas down bind group", &scene_view, &scene_view);
        let blur_h_bind_group = bind_group("atlas blur h bind group", &bloom_a_view, &bloom_a_view);
        let blur_v_bind_group = bind_group("atlas blur v bind group", &bloom_b_view, &bloom_b_view);
        let composite_bind_group =
            bind_group("atlas composite bind group", &scene_view, &bloom_a_view);

        self.targets = Some(Targets {
            size: scene_size,
            bloom_size,
            scene_view,
            bloom_a_view,
            bloom_b_view,
            down_bind_group,
            blur_h_bind_group,
            blur_v_bind_group,
            composite_bind_group,
        });
    }
}

fn create_texture(
    device: &wgpu::Device,
    label: &str,
    size: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0].max(1),
            height: size[1].max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn make_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fragment_entry: &str,
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_fullscreen"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// One frame's worth of scene parameters, submitted through egui's paint
/// callback so WebGPU draws underneath the egui UI in the same pass.
#[derive(Clone, Copy, Debug)]
pub struct BackgroundCallback {
    pub frame: SceneFrame,
    pub render_scale: f32,
}

impl BackgroundCallback {
    pub fn new(frame: SceneFrame, render_scale: f32) -> Self {
        Self {
            frame,
            render_scale,
        }
    }
}

impl CallbackTrait for BackgroundCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(renderer) = callback_resources.get_mut::<SceneRenderer>() {
            renderer.prepare(
                device,
                queue,
                egui_encoder,
                screen_descriptor.size_in_pixels,
                &self.frame,
                self.render_scale,
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        if let Some(renderer) = callback_resources.get::<SceneRenderer>() {
            renderer.paint(render_pass);
        }
    }
}
