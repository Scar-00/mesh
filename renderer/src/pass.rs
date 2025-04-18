use wgpu::{
    BindGroup, BindGroupLayout, BlendState, Buffer, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device,
    Face, FragmentState, FrontFace, IndexFormat, MultisampleState, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, StencilState, SurfaceConfiguration,
    TextureFormat,
};

use crate::{
    data::DrawData,
    vertex::{Index, IntoBuffer, VertexData},
};

#[derive(Debug)]
#[allow(unused)]
pub struct PassOptions<'a> {
    name: &'a str,
    device: &'a Device,
    config: &'a SurfaceConfiguration,
    bind_group_layouts: &'a [&'a BindGroupLayout],
    bind_group: BindGroup,
    shader_module: ShaderModuleDescriptor<'a>,
}

pub struct Pass<V: VertexData> {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    draw_data: DrawData<V>,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
}

impl<V: VertexData> Pass<V> {
    const DEFAULT_BUFFER_SIZE: u64 = 1024 * 4;

    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        bind_group_layouts: &[&BindGroupLayout],
        bind_group: BindGroup,
        name: impl AsRef<str>,
        shader_module: ShaderModuleDescriptor<'_>,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(name.as_ref()),
            size: Self::DEFAULT_BUFFER_SIZE,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(name.as_ref()),
            size: Self::DEFAULT_BUFFER_SIZE,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(shader_module);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{}-pipeline-layout", name.as_ref())),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{}-pipeline", name.as_ref())),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[V::descriptor()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            vertex_buffer,
            index_buffer,
            draw_data: DrawData::new(),
            pipeline,
            bind_group,
        }
    }

    pub fn add_data<const VN: usize, const VI: usize>(
        &mut self,
        vertecies: [V; VN],
        indicies: [Index; VI],
    ) {
        self.draw_data.vertecies.extend(vertecies);
        self.draw_data.indicies.reserve(VI);
        for index in indicies {
            self.draw_data
                .indicies
                .push(self.draw_data.offset as u32 + index);
        }
        self.draw_data.offset += VN;
    }

    /*fn invalidate_buffers(&mut self) {
        todo!();
    }*/

    fn finalize(&self, queue: &Queue) {
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            self.draw_data.vertecies.as_u8_slice(),
        );
        queue.write_buffer(&self.index_buffer, 0, self.draw_data.indicies.as_u8_slice());
        queue.submit([]);
    }

    pub fn submit(&mut self, queue: &Queue, render_pass: &mut RenderPass) {
        self.finalize(queue);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..(self.draw_data.indicies.len() as u32), 0, 0..1);
        self.draw_data.reset();
    }
}
