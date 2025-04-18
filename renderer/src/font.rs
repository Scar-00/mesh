use std::{collections::HashMap, ffi::OsStr, sync::Arc};

use freetype::{face::LoadFlag, Library, RenderMode};
use glam::{IVec2, Vec2};
use wgpu::{
    AddressMode, Device, Extent3d, FilterMode, Origin3d, Queue, SamplerDescriptor,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

use crate::texture::Texture;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("freetype error: {0}")]
    Freetype(#[from] freetype::Error),
    #[error("failed to retrieve font name")]
    NoName,
}

pub struct FontContext {
    freetype: Library,
    font_list: HashMap<String, Arc<Font>>,
}

impl FontContext {
    const DEFAULT_FONT_SIZE: usize = 32;

    pub fn new() -> Result<FontContext, Error> {
        Ok(Self {
            freetype: Library::init()?,
            font_list: HashMap::new(),
        })
    }

    pub fn load_font(&mut self, path: impl AsRef<OsStr>) -> Result<Arc<Font>, Error> {
        let face = self.freetype.new_face(path, 0)?;
        face.set_pixel_sizes(0, Self::DEFAULT_FONT_SIZE as u32)?;
        let Some(name) = face.family_name() else {
            return Err(Error::NoName);
        };

        let glyphs_to_load = face.num_glyphs() as usize;

        let max_dim = (1 + (face.size_metrics().unwrap().height >> 5))
            * ((glyphs_to_load as f32).sqrt().ceil()) as i32;

        let mut tex_size = IVec2::new(1, 0);
        while tex_size.x < max_dim {
            tex_size.x <<= 1;
        }
        tex_size.y = tex_size.x;

        let mut pen = IVec2::new(0, 0);

        let mut bitmap = Vec::with_capacity(tex_size.x as usize * tex_size.y as usize);
        unsafe { bitmap.set_len(bitmap.capacity()) };

        let mut chars = Vec::with_capacity(glyphs_to_load);

        for i in 0..glyphs_to_load {
            if face.load_char(i, LoadFlag::RENDER).is_err() {
                continue;
            };

            let glyph = face.glyph();
            if glyph.render_glyph(RenderMode::Sdf).is_err() {
                continue;
            }

            let Some(face_size) = face.size_metrics() else {
                continue;
            };
            let glyph_bitmap = glyph.bitmap();
            if pen.x + glyph_bitmap.width() >= tex_size.x {
                pen.x = 0;
                pen.y += (face_size.height >> 5) + 1;
            }

            for row in 0..glyph_bitmap.rows() {
                for col in 0..glyph_bitmap.width() {
                    let x = pen.x + col;
                    let y = pen.y + row;
                    bitmap[(y * tex_size.x + x) as usize] =
                        glyph_bitmap.buffer()[(row * glyph_bitmap.pitch() + col) as usize];
                }
            }

            chars.push(Glyph {
                size: Vec2::new(glyph_bitmap.width() as f32, glyph_bitmap.rows() as f32),
                bearing: Vec2::new(glyph.bitmap_left() as f32, glyph.bitmap_top() as f32),
                uv_min: Vec2::new(
                    pen.x as f32 / tex_size.x as f32,
                    pen.y as f32 / tex_size.y as f32,
                ),
                uv_max: Vec2::new(
                    (pen.x as f32 + glyph_bitmap.width() as f32) / tex_size.y as f32,
                    (pen.y as f32 + glyph_bitmap.rows() as f32) / tex_size.y as f32,
                ),
                advance: glyph.advance().x as u32,
            });
            pen.x += glyph_bitmap.width() + 1;
        }

        self.font_list.insert(
            name.clone(),
            Arc::new(Font {
                name: name.clone(),
                chars,
                bitmap,
                size: tex_size,
                font_size: Self::DEFAULT_FONT_SIZE,
            }),
        );
        Ok(self.font_list.get(&name).unwrap().clone())
    }

    pub fn get(&self, font_name: impl AsRef<str>) -> Option<Arc<Font>> {
        self.font_list.get(font_name.as_ref()).cloned()
    }

    pub fn last(&self) -> Arc<Font> {
        self.font_list.values().last().cloned().unwrap()
    }
}

#[derive(Debug)]
pub struct Glyph {
    pub(crate) size: Vec2,
    pub(crate) bearing: Vec2,
    pub(crate) uv_min: Vec2,
    pub(crate) uv_max: Vec2,
    pub(crate) advance: u32,
}

#[derive(Debug)]
pub struct Font {
    pub(crate) name: String,
    pub(crate) chars: Vec<Glyph>,
    pub(crate) bitmap: Vec<u8>,
    pub(crate) size: IVec2,
    pub(crate) font_size: usize,
}

impl Font {
    pub fn create_texture(&self, device: &Device, queue: &Queue) -> Texture {
        let size = Extent3d {
            width: self.size.x as u32,
            height: self.size.y as u32,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&format!("font[{}]-texture", self.name)),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &self.bitmap,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.size.x as u32),
                rows_per_image: Some(self.size.y as u32),
            },
            size,
        );

        let view = texture.create_view(&Default::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some(&format!("font[{}]-sampler", self.name)),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Texture {
            texture,
            view,
            sampler,
        }
    }

    pub fn calc_size(&self, size: f32, text: impl AsRef<str>) -> Vec2 {
        let font_scale = size / self.font_size as f32;
        let mut out_size = Vec2::new(0.0, size);
        for char in text.as_ref().chars() {
            if char == '\n' {
                continue;
            }

            let char = &self.chars[char as usize];

            out_size.x += (char.advance >> 6) as f32 * font_scale;
        }
        if let Some(first) = text.as_ref().chars().next() {
            let char = &self.chars[first as usize];
            out_size.x -= char.bearing.x;
        }
        out_size
    }
}
