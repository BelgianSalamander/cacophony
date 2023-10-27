use wasm_bindgen::prelude::{Closure, wasm_bindgen};
use web_sys::HtmlCanvasElement;
use wgpu::Device;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::console_log;
use crate::noise::source::{TestSource, NoiseSource, Coord};
use crate::util::get_expected_size;

use super::camera::Camera;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2]
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0], uv: [0.0, 0.0] },
    Vertex { position: [ 1.0, -1.0], uv: [1.0, 0.0] },
    Vertex { position: [ 1.0,  1.0], uv: [1.0, 1.0] },
    Vertex { position: [-1.0,  1.0], uv: [0.0, 1.0] }
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0,
];

const TEX_SIZE: u32 = 512;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RenderSettings {
    view_proj: [[f32; 4]; 4],
    height_scale: f32,
    tex_size: u32,
    _padding: [u8; 8]
}

impl RenderSettings {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            height_scale: 1.0,
            tex_size: TEX_SIZE,
            _padding: [0; 8]
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

struct ChunkBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32
}

impl ChunkBuffers {
    pub fn generate(device: &wgpu::Device, size: u32, density: f32) -> Self {
        let mut points = vec![];

        //Add border
        for i in 0..size {
            points.push(
                delaunator::Point {
                    x: i as f64,
                    y: 0.0
                }
            );

            points.push(
                delaunator::Point {
                    x: i as f64,
                    y: size as f64 - 1.0
                }
            );
        }

        for i in 1..size-1 {
            points.push(
                delaunator::Point {
                    x: 0.0,
                    y: i as f64
                }
            );

            points.push(
                delaunator::Point {
                    x: size as f64 - 1.0,
                    y: i as f64
                }
            );
        }

        let inner_size = size - 2;
        let num_inner_points = (inner_size as f32 * density).ceil() as u32;

        for i in  0..num_inner_points {
            for j in 0..num_inner_points {
                let ti = (i + 1) as f64 / (size as f64 - 1.0);
                let tj = (j + 1) as f64 / (size as f64 - 1.0);

                let x = ti * (size + 1) as f64;
                let y = tj * (size + 1) as f64;

                points.push(
                    delaunator::Point { x, y }
                );
            }
        }

        let indices: Vec<_> = delaunator::triangulate(&points).triangles.into_iter().map(|i| i as u32).collect();
        let num_indices = indices.len() as u32;
        let vertices: Vec<_> = points.into_iter().map(|p| {
            Vertex {
                position: [p.x as f32, p.y as f32],
                uv: [p.x as f32 / (size - 1) as f32, p.y as f32 / (size - 1) as f32]
            }
        }).collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        console_log!("Generated {} vertices and {} indices", vertices.len(), indices.len());

        Self {
            vertex_buffer,
            index_buffer,
            num_indices
        }
    }
}

pub struct WgpuContext {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,

    chunk_buffers: ChunkBuffers,

    render_settings_uniform: RenderSettings,
    render_settings_uniform_buffer: wgpu::Buffer,
    render_settings_uniform_bind_group: wgpu::BindGroup,

    noise_texture_bind_group: wgpu::BindGroup,
}

impl WgpuContext {
    pub async fn new(canvas: &HtmlCanvasElement, camera: &Camera)-> Self {
        let (width, height) = get_expected_size(canvas);
        console_log!("Surface size: {} {}", width, height);
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface_from_canvas(canvas.clone()).expect("Could not create surface :(");

        let adpater = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        console_log!("Adapter: {:?}", adpater.get_info());

        let (device, queue) = adpater
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: None
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adpater);
        let surface_format = surface_caps.formats.iter().copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![]
        };
        surface.configure(&device, &config);

        let (render_settings_uniform, render_settings_uniform_buffer, render_settings_uniform_bind_group, render_settings_bind_group_layout) = Self::create_render_settings_uniform(camera, &device);

        let chunk_buffers = ChunkBuffers::generate(&device, 100, 1.0);

        let noise_texture_size = TEX_SIZE;
        let noise_res = 0.1;
        let src = TestSource;

        let noise_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: noise_texture_size,
                height: noise_texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Noise texture"),
            view_formats: &[]
        };
        let noise_texture = device.create_texture(&noise_texture_desc);
        
        let pixel_size = std::mem::size_of::<f32>() as u32;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bytes_per_row = pixel_size * noise_texture_size;
        let padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padding;

        if padded_bytes_per_row % pixel_size != 0 {
            panic!("Padded bytes per row is not a multiple of pixel size");
        }

        let padded_pixels_per_row = padded_bytes_per_row / pixel_size;

        let mut noise_texture_data = vec![0.0; padded_pixels_per_row as usize * noise_texture_size as usize];

        for x in 0..noise_texture_size {
            for y in 0..noise_texture_size {
                let noise = src.sample(x as Coord * noise_res, y as Coord * noise_res, 0);
                let normed = noise * 0.5 + 0.5;

                let idx = padded_pixels_per_row as usize * y as usize + x as usize;
                noise_texture_data[idx] = normed;
            }
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &noise_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&noise_texture_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(noise_texture_size),
            },
            noise_texture_desc.size
        );

        let noise_texture_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let noise_texture_sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );

        let noise_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { 
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },  
                        view_dimension: wgpu::TextureViewDimension::D2, 
                        multisampled: false 
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ],
            label: Some("Noise texture bind group layout"),
        });

        let noise_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &noise_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&noise_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&noise_texture_sampler),
                },
            ],
            label: Some("Noise texture bind group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into())
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &render_settings_bind_group_layout,
                &noise_texture_bind_group_layout,
            ],
            push_constant_ranges: &[]
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview: None
        });

        Self {
            surface,
            device,
            queue,
            config,
            size: PhysicalSize::new(width, height),

            render_pipeline,

            chunk_buffers,

            render_settings_uniform,
            render_settings_uniform_buffer,
            render_settings_uniform_bind_group,

            noise_texture_bind_group
        }
    }

    fn create_render_settings_uniform(camera: &Camera, device: &Device) -> (RenderSettings, wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
        let mut render_settings_uniform = RenderSettings::new();
        render_settings_uniform.update_view_proj(camera);

        let render_settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: bytemuck::cast_slice(&[render_settings_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let render_settings_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Camera uniform bind group layout"),
        });

        let render_settings_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_settings_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: render_settings_buffer.as_entire_binding(),
                },
            ],
            label: Some("Camera uniform bind group"),
        });

        (render_settings_uniform, render_settings_buffer, render_settings_bind_group, render_settings_bind_group_layout)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            self.surface.configure(&self.device, &self.config);

            console_log!("Resized canvas to {}x{}", new_size.width, new_size.height);
        }
    }

    pub fn render(&mut self, delay: f64, camera: &Camera) -> Result<(), wgpu::SurfaceError>{
        self.render_settings_uniform.update_view_proj(camera);
        self.queue.write_buffer(&self.render_settings_uniform_buffer, 0, bytemuck::cast_slice(&[self.render_settings_uniform]));

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(
                                wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0
                                }
                            ),
                            store: true
                        }
                    })
                ],
                depth_stencil_attachment: None
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(0, self.chunk_buffers.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.chunk_buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.set_bind_group(0, &self.render_settings_uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.noise_texture_bind_group, &[]);

            render_pass.draw_indexed(0..self.chunk_buffers.num_indices, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}