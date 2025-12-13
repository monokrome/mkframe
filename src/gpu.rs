//! GPU-accelerated rendering using wgpu with automatic fallback to software.

#[cfg(feature = "gpu")]
use bytemuck::{Pod, Zeroable};

/// Renderer backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    /// GPU-accelerated via wgpu (Vulkan, Metal, DX12)
    Gpu,
    /// Software rendering via tiny-skia
    Software,
}

/// A renderer that can use GPU or fallback to software
pub struct Renderer {
    backend: RendererBackend,
    #[cfg(feature = "gpu")]
    gpu: Option<GpuState>,
}

#[cfg(feature = "gpu")]
struct GpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    // Cached resources for 2D rendering
    rect_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BlitVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

impl Renderer {
    /// Create a new renderer, preferring GPU if available
    pub fn new() -> Self {
        #[cfg(feature = "gpu")]
        {
            match Self::try_create_gpu() {
                Ok(gpu) => {
                    log::info!(
                        "Using GPU renderer ({})",
                        gpu.device.limits().max_texture_dimension_2d
                    );
                    return Self {
                        backend: RendererBackend::Gpu,
                        gpu: Some(gpu),
                    };
                }
                Err(e) => {
                    log::warn!("GPU renderer unavailable, falling back to software: {}", e);
                }
            }
        }

        log::info!("Using software renderer");
        Self {
            backend: RendererBackend::Software,
            #[cfg(feature = "gpu")]
            gpu: None,
        }
    }

    /// Force software rendering (useful for testing)
    pub fn new_software() -> Self {
        Self {
            backend: RendererBackend::Software,
            #[cfg(feature = "gpu")]
            gpu: None,
        }
    }

    /// Get the current backend type
    pub fn backend(&self) -> RendererBackend {
        self.backend
    }

    /// Check if GPU rendering is active
    pub fn is_gpu(&self) -> bool {
        self.backend == RendererBackend::Gpu
    }

    #[cfg(feature = "gpu")]
    fn try_create_gpu() -> Result<GpuState, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or("No suitable GPU adapter found")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("mkframe"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| format!("Failed to create device: {}", e))?;

        // Create rect shader for solid color rectangles
        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect_shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });

        // Create blit shader for texture blitting
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit_shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit_pipeline_layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<BlitVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(GpuState {
            device,
            queue,
            rect_pipeline,
            blit_pipeline,
            sampler,
        })
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "gpu")]
const RECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[cfg(feature = "gpu")]
const BLIT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_sampler, in.tex_coord);
}
"#;

/// GPU-accelerated render target that can be read back to CPU
#[cfg(feature = "gpu")]
pub struct GpuRenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    readback_buffer: wgpu::Buffer,
}

#[cfg(feature = "gpu")]
impl GpuRenderTarget {
    pub fn new(renderer: &Renderer, width: u32, height: u32) -> Option<Self> {
        let gpu = renderer.gpu.as_ref()?;

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Buffer for reading back pixels (must be aligned to 256 bytes per row)
        let bytes_per_row = (width * 4 + 255) & !255;
        let buffer_size = (bytes_per_row * height) as u64;

        let readback_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Some(Self {
            texture,
            view,
            width,
            height,
            readback_buffer,
        })
    }

    /// Read pixels back to CPU buffer (BGRA format for Wayland)
    pub fn read_to_buffer(&self, renderer: &Renderer, output: &mut [u8]) {
        let Some(gpu) = renderer.gpu.as_ref() else {
            return;
        };

        let bytes_per_row = (self.width * 4 + 255) & !255;

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback_encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        gpu.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read
        let buffer_slice = self.readback_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::Maintain::Wait);

        {
            let data = buffer_slice.get_mapped_range();
            // Copy row by row (handle padding)
            let src_stride = bytes_per_row as usize;
            let dst_stride = (self.width * 4) as usize;
            for y in 0..self.height as usize {
                let src_offset = y * src_stride;
                let dst_offset = y * dst_stride;
                // Convert RGBA to BGRA for Wayland
                for x in 0..self.width as usize {
                    let si = src_offset + x * 4;
                    let di = dst_offset + x * 4;
                    output[di] = data[si + 2]; // B
                    output[di + 1] = data[si + 1]; // G
                    output[di + 2] = data[si]; // R
                    output[di + 3] = data[si + 3]; // A
                }
            }
        }

        self.readback_buffer.unmap();
    }
}
