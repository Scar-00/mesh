use wgpu::{Sampler, TextureView};

#[allow(dead_code)]
pub struct Texture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: TextureView,
    pub(crate) sampler: Sampler,
}

impl Texture {
    pub fn new(texture: wgpu::Texture, view: TextureView, sampler: Sampler) -> Self {
        Self {
            texture,
            view,
            sampler,
        }
    }
}
