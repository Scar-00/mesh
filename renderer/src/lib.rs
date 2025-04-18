pub mod color;
pub mod data;
pub mod font;
pub mod pass;
pub mod texture;
pub mod vertex;
use color::ToColor;
use data::DrawData;
use font::FontContext;
use glam::{Mat4, Quat, UVec2, Vec2, Vec3};
use pass::Pass;
use std::sync::Arc;
use tao::{dpi::PhysicalSize, window::Window};
use texture::Texture;
use util::Shared;
use vertex::*;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, Backends, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferUsages,
    Color, CommandEncoderDescriptor, CompareFunction, CompositeAlphaMode, CreateSurfaceError,
    Device, DeviceDescriptor, Extent3d, Features, FilterMode, Instance, InstanceDescriptor, Limits,
    LoadOp, Operations, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RequestAdapterOptions,
    RequestDeviceError, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp, Surface,
    SurfaceConfiguration, SurfaceError, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct GeometryVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

impl VertexData for GeometryVertex {
    const VERTEX_ATTRIBUTES: &[wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TextVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl VertexData for TextVertex {
    const VERTEX_ATTRIBUTES: &[wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4];
}

struct ViewPorj {
    view: Mat4,
    proj: Mat4,
}

struct DebugCamera {
    min: UVec2,
    max: UVec2,
    //zoom: f32,
    //aspect: f32,
    pos: Vec3,
    view_proj: ViewPorj,
}

impl DebugCamera {
    pub fn new(view_port_size: UVec2) -> Self {
        let mut this = Self {
            pos: Vec3::ZERO,
            //aspect: (view_port_size.x / view_port_size.y) as f32,
            view_proj: ViewPorj {
                view: Mat4::IDENTITY,
                proj: Mat4::IDENTITY,
            },
            min: UVec2::ZERO,
            max: view_port_size,
            //zoom: 1.0,
        };
        this.update();
        return this;
    }

    pub fn on_resize(&mut self, new_size: UVec2) {
        self.max = new_size;
        self.update();
    }

    pub fn update(&mut self) {
        let transform = Mat4::IDENTITY.transform_point3(self.pos);
        self.view_proj.view = Mat4::inverse(&Mat4::from_translation(transform));
        self.view_proj.proj = Mat4::orthographic_rh(
            self.min.x as f32,
            self.max.x as f32,
            self.min.y as f32,
            self.max.y as f32,
            -100.0,
            100.0,
        );
    }

    pub fn view_proj(&self) -> Mat4 {
        self.view_proj.view * self.view_proj.proj
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] CreateSurfaceError),
    #[error("surface error: {0}")]
    Surface(#[from] SurfaceError),
    #[error("no adapter found")]
    NoAdapter,
    #[error("failed to request device: {0}")]
    RequestDevice(#[from] RequestDeviceError),
    #[error("font error: {0}")]
    Font(#[from] font::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[allow(dead_code)]
pub struct Renderer<'a> {
    window: Arc<Window>,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: UVec2,
    debug_camera: DebugCamera,
    camera_buffer: Buffer,
    depth_texture: Texture,
    geom_pass: Pass<GeometryVertex>,
    text_pass: Pass<TextVertex>,
    font_context: Shared<FontContext>,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: Arc<Window>, font_context: Shared<FontContext>) -> Result<Self> {
        let size = UVec2::new(window.inner_size().width, window.inner_size().height);
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(Error::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.x,
            height: size.y,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let debug_camera = DebugCamera::new(size);

        let view_proj = debug_camera.view_proj();
        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera-buffer"),
            contents: view_proj.as_u8_slice(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("camera-bind-group-layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let depth_texture = Self::create_depth_texture(&device, &config);

        let geom_pass = Pass::new(
            &device,
            &config,
            &[&camera_bind_group_layout],
            camera_bind_group,
            "geom-pass",
            wgpu::include_wgsl!("../../res/test.wgsl"),
        );

        let font = font_context.get_mut().load_font("./res/consola.ttf")?;
        let text_texture = font.create_texture(&device, &queue);

        let text_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("text-bind-group-layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let text_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &text_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&text_texture.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&text_texture.sampler),
                },
            ],
        });

        let text_pass = Pass::new(
            &device,
            &config,
            &[&text_bind_group_layout],
            text_bind_group,
            "text-pass",
            wgpu::include_wgsl!("../../res/text_test.wgsl"),
        );

        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            debug_camera,
            camera_buffer,
            depth_texture,
            geom_pass,
            text_pass,
            font_context,
        })
    }

    fn create_depth_texture(device: &Device, config: &SurfaceConfiguration) -> Texture {
        let size = Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let descriptor = TextureDescriptor {
            label: Some("depth-texture-descriptor"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Texture::new(texture, view, sampler)
    }

    pub fn on_resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = UVec2::new(new_size.width, new_size.height);
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.debug_camera.on_resize(self.size);
            self.depth_texture = Self::create_depth_texture(&self.device, &self.config);
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                self.debug_camera.view_proj().as_u8_slice(),
            );
            self.queue.submit([]);
        }
    }

    pub fn render(&mut self) -> Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut geom_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        {
            let mut geom_render_pass = geom_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.geom_pass.submit(&self.queue, &mut geom_render_pass);
        }

        let mut text_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        {
            let mut text_render_pass = text_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("text-render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.text_pass.submit(&self.queue, &mut text_render_pass);
        }

        self.queue
            .submit([geom_encoder.finish(), text_encoder.finish()]);
        output.present();

        Ok(())
    }

    pub fn add_quad_v3(&mut self, pos: Vec3, size: Vec3, color: &impl ToColor) {
        let transform = Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, pos);

        let vertecies = [
            GeometryVertex {
                pos: transform
                    .transform_point3(DrawData::QUAD_POS[0].into())
                    .to_array(),
                color: color.to_color().as_array(),
            },
            GeometryVertex {
                pos: transform
                    .transform_point3(DrawData::QUAD_POS[1].into())
                    .to_array(),
                color: color.to_color().as_array(),
            },
            GeometryVertex {
                pos: transform
                    .transform_point3(DrawData::QUAD_POS[2].into())
                    .to_array(),
                color: color.to_color().as_array(),
            },
            GeometryVertex {
                pos: transform
                    .transform_point3(DrawData::QUAD_POS[3].into())
                    .to_array(),
                color: color.to_color().as_array(),
            },
        ];

        self.geom_pass.add_data(vertecies, [0, 1, 2, 2, 3, 0]);
    }

    pub fn add_quad(&mut self, pos: Vec2, size: Vec2, color: &impl ToColor) {
        self.add_quad_v3(
            Vec3::new(pos.x, pos.y, 0.0),
            Vec3::new(size.x, size.y, 0.0),
            color,
        )
    }

    pub fn add_debug(&mut self, pos: Vec2, size: Vec2, color: &impl ToColor) {
        let thickness = 20.0;
        self.add_quad_v3(
            Vec3::new(pos.x, pos.x, 100.0),
            Vec3::new(thickness, size.y, 0.0),
            color,
        );
        self.add_quad_v3(
            Vec3::new(pos.y, pos.y, 100.0),
            Vec3::new(thickness, size.y, 0.0),
            color,
        );
        self.add_quad_v3(
            Vec3::new(pos.x, pos.x, 0.0),
            Vec3::new(size.x, thickness, 0.0),
            color,
        );
        self.add_quad_v3(
            Vec3::new(pos.y, pos.y, 0.0),
            Vec3::new(size.x, thickness, 0.0),
            color,
        );
    }

    pub fn add_text(
        &mut self,
        mut pos: Vec3,
        size: f32,
        text: impl AsRef<str>,
        color: &impl ToColor,
    ) {
        let font = self.font_context.get().get("Consolas").unwrap();
        let font_size = size / font.font_size as f32;
        let mut z_index_adder = 0.0;
        for char in text.as_ref().chars() {
            if char == '\n' {
                continue;
            }

            let char = &font.chars[char as usize];
            let e_pos = Vec3::new(
                pos.x + char.bearing.x,
                pos.y - (char.size.y - char.bearing.y),
                pos.z + z_index_adder,
            );
            z_index_adder += 0.001;

            let uv_coords = [
                [char.uv_min.x, char.uv_max.y],
                [char.uv_max.x, char.uv_max.y],
                [char.uv_max.x, char.uv_min.y],
                [char.uv_min.x, char.uv_min.y],
            ];

            let size = Vec3::new(char.size.x * font_size, char.size.y * font_size, 1.0);
            let transform = Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, e_pos);

            let color = color.to_color();

            let vertecies = [
                TextVertex {
                    pos: transform
                        .transform_point3(DrawData::QUAD_POS[0].into())
                        .to_array(),
                    uv: uv_coords[0],
                    color: color.as_array(),
                },
                TextVertex {
                    pos: transform
                        .transform_point3(DrawData::QUAD_POS[1].into())
                        .to_array(),
                    uv: uv_coords[1],
                    color: color.as_array(),
                },
                TextVertex {
                    pos: transform
                        .transform_point3(DrawData::QUAD_POS[2].into())
                        .to_array(),
                    uv: uv_coords[2],
                    color: color.as_array(),
                },
                TextVertex {
                    pos: transform
                        .transform_point3(DrawData::QUAD_POS[3].into())
                        .to_array(),
                    uv: uv_coords[3],
                    color: color.as_array(),
                },
            ];

            self.text_pass.add_data(vertecies, [0, 1, 2, 2, 3, 0]);
            pos.x += (char.advance >> 6) as f32 * font_size;
        }
    }
}
