use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i16,
    pub y: i16,
}

impl TilePos {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_pos_equality() {
        assert_eq!(TilePos::new(1, 2), TilePos { x: 1, y: 2 });
        assert_ne!(TilePos::new(1, 2), TilePos::new(2, 1));
    }

    #[test]
    fn tile_pos_default_is_origin() {
        assert_eq!(TilePos::default(), TilePos { x: 0, y: 0 });
    }
}
