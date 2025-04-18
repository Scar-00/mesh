use glam::Vec4;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub fn new(c: impl ToColor) -> Self {
        c.to_color()
    }

    pub fn as_array(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

pub trait ToColor {
    fn to_color(&self) -> Color;
}

impl ToColor for Color {
    fn to_color(&self) -> Color {
        self.clone()
    }
}

impl ToColor for Vec4 {
    fn to_color(&self) -> Color {
        Color {
            r: (self.x * 255.0) as u8,
            g: (self.y * 255.0) as u8,
            b: (self.z * 255.0) as u8,
            a: (self.w * 255.0) as u8,
        }
    }
}

impl ToColor for [f32; 4] {
    fn to_color(&self) -> Color {
        Color {
            r: (self[0] * 255.0) as u8,
            g: (self[1] * 255.0) as u8,
            b: (self[2] * 255.0) as u8,
            a: (self[3] * 255.0) as u8,
        }
    }
}

impl ToColor for u32 {
    fn to_color(&self) -> Color {
        Color {
            r: (((*self) >> 0) & 0xFF) as u8,
            g: (((*self) >> 8) & 0xFF) as u8,
            b: (((*self) >> 16) & 0xFF) as u8,
            a: (((*self) >> 24) & 0xFF) as u8,
        }
    }
}

impl ToColor for i32 {
    fn to_color(&self) -> Color {
        Color {
            r: (((*self) >> 24) & 0xFF) as u8,
            g: (((*self) >> 16) & 0xFF) as u8,
            b: (((*self) >> 08) & 0xFF) as u8,
            a: (((*self) >> 00) & 0xFF) as u8,
        }
    }
}
