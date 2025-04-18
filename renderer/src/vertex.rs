use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

pub trait IntoBuffer {
    fn as_u8_slice(&self) -> &[u8];
}

pub trait VertexData: Sized {
    const VERTEX_ATTRIBUTES: &[VertexAttribute];

    fn descriptor() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}

impl<T> IntoBuffer for [T] {
    fn as_u8_slice(&self) -> &[u8] {
        let base = self.as_ptr() as *const u8;
        let size = std::mem::size_of::<T>();
        let slice = unsafe { std::slice::from_raw_parts(base, size * self.len()) };
        slice
    }
}

impl<T> IntoBuffer for Vec<T> {
    fn as_u8_slice(&self) -> &[u8] {
        let base = self.as_ptr() as *const u8;
        let size = std::mem::size_of::<T>();
        let slice = unsafe { std::slice::from_raw_parts(base, size * self.len()) };
        slice
    }
}

impl IntoBuffer for glam::Mat4 {
    fn as_u8_slice(&self) -> &[u8] {
        let slice = self.as_ref();
        let base = slice.as_ptr() as *const u8;
        let size = std::mem::size_of::<f32>();
        let slice = unsafe { std::slice::from_raw_parts(base, slice.len() * size) };
        slice
    }
}

pub type Index = u32;
