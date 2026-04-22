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

/// Row-major walkable grid. Out-of-bounds is treated as non-walkable so the
/// map edge acts as an implicit wall — callers don't need to pad the input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WalkGrid {
    width: i16,
    height: i16,
    cells: Vec<bool>,
}

impl WalkGrid {
    /// All-walkable grid of size `width × height`. `width`/`height` are clamped
    /// to `>= 0`.
    pub fn new_all_walkable(width: i16, height: i16) -> Self {
        let w = width.max(0);
        let h = height.max(0);
        let n = (w as usize) * (h as usize);
        Self {
            width: w,
            height: h,
            cells: vec![true; n],
        }
    }

    /// Build from row-major cells. Panics if `cells.len() != width * height`
    /// (this is a programmer error, not external input).
    pub fn from_row_major(width: i16, height: i16, cells: Vec<bool>) -> Self {
        let w = width.max(0);
        let h = height.max(0);
        let expected = (w as usize) * (h as usize);
        assert_eq!(
            cells.len(),
            expected,
            "WalkGrid::from_row_major: cells.len()={} but width*height={}",
            cells.len(),
            expected
        );
        Self {
            width: w,
            height: h,
            cells,
        }
    }

    pub fn width(&self) -> i16 {
        self.width
    }

    pub fn height(&self) -> i16 {
        self.height
    }

    /// Out-of-bounds tiles are non-walkable. This is the ONE authoritative
    /// walkability check — callers must not reimplement bounds logic.
    pub fn is_walkable(&self, tile: TilePos) -> bool {
        if tile.x < 0 || tile.y < 0 || tile.x >= self.width || tile.y >= self.height {
            return false;
        }
        let idx = (tile.y as usize) * (self.width as usize) + (tile.x as usize);
        self.cells[idx]
    }

    pub fn set(&mut self, tile: TilePos, walkable: bool) {
        if tile.x < 0 || tile.y < 0 || tile.x >= self.width || tile.y >= self.height {
            return;
        }
        let idx = (tile.y as usize) * (self.width as usize) + (tile.x as usize);
        self.cells[idx] = walkable;
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

    #[test]
    fn walk_grid_new_all_walkable_has_expected_size() {
        let g = WalkGrid::new_all_walkable(5, 3);
        assert_eq!(g.width(), 5);
        assert_eq!(g.height(), 3);
        for y in 0..3 {
            for x in 0..5 {
                assert!(g.is_walkable(TilePos::new(x, y)));
            }
        }
    }

    #[test]
    fn walk_grid_out_of_bounds_is_not_walkable() {
        let g = WalkGrid::new_all_walkable(3, 2);
        assert!(!g.is_walkable(TilePos::new(-1, 0)));
        assert!(!g.is_walkable(TilePos::new(0, -1)));
        assert!(!g.is_walkable(TilePos::new(3, 0)));
        assert!(!g.is_walkable(TilePos::new(0, 2)));
        assert!(!g.is_walkable(TilePos::new(100, 100)));
    }

    #[test]
    fn walk_grid_set_marks_tile_non_walkable() {
        let mut g = WalkGrid::new_all_walkable(4, 4);
        g.set(TilePos::new(2, 1), false);
        assert!(!g.is_walkable(TilePos::new(2, 1)));
        assert!(g.is_walkable(TilePos::new(1, 1)));
        assert!(g.is_walkable(TilePos::new(3, 1)));
    }

    #[test]
    fn walk_grid_from_row_major_preserves_order() {
        // 3x2:
        //   y=0: [T, F, T]
        //   y=1: [F, T, F]
        let cells = vec![true, false, true, false, true, false];
        let g = WalkGrid::from_row_major(3, 2, cells);
        assert!(g.is_walkable(TilePos::new(0, 0)));
        assert!(!g.is_walkable(TilePos::new(1, 0)));
        assert!(g.is_walkable(TilePos::new(2, 0)));
        assert!(!g.is_walkable(TilePos::new(0, 1)));
        assert!(g.is_walkable(TilePos::new(1, 1)));
        assert!(!g.is_walkable(TilePos::new(2, 1)));
    }

    #[test]
    fn walk_grid_set_out_of_bounds_is_silent_noop() {
        let mut g = WalkGrid::new_all_walkable(2, 2);
        g.set(TilePos::new(-1, 0), false);
        g.set(TilePos::new(5, 5), false);
        // Nothing changed; in-bounds tiles still walkable.
        for y in 0..2 {
            for x in 0..2 {
                assert!(g.is_walkable(TilePos::new(x, y)));
            }
        }
    }
}
