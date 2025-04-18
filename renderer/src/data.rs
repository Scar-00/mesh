use crate::vertex::Index;

pub struct DrawData<V> {
    pub(crate) vertecies: Vec<V>,
    pub(crate) indicies: Vec<Index>,
    pub(crate) offset: usize,
}

impl<V> DrawData<V> {
    pub fn new() -> Self {
        Self {
            vertecies: Vec::new(),
            indicies: Vec::new(),
            offset: 0,
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0;
        self.vertecies.clear();
        self.indicies.clear();
    }
}

impl DrawData<()> {
    pub const QUAD_POS: [[f32; 3]; 4] = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];

    pub const QUAD_UV_COORDS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
}
