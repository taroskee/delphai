#![allow(clippy::result_large_err)]
// godot 0.2 macros generate `Result<_, CallError>` where CallError is >160B;
// the lint can't see through the macro so we silence it crate-wide.

use delphai_core::pathfinding::TilePos;
use delphai_core::resource::Resource;
use delphai_core::world::{MapBounds, World};
use godot::prelude::*;

struct DelphaiExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DelphaiExtension {}

/// Pure helpers — keep the index/bounds logic out of `#[func]` methods so it
/// can be exercised under `cargo test` without the godot runtime.
fn citizen_name_at(world: &World, i: i64) -> Option<&str> {
    let idx = usize::try_from(i).ok()?;
    world.citizens.get(idx).map(|c| c.name.as_str())
}

fn citizen_world_pos_at(world: &World, i: i64, alpha: f32) -> Option<(f32, f32)> {
    let idx = usize::try_from(i).ok()?;
    if idx >= world.citizen_moves.len() {
        return None;
    }
    Some(world.get_citizen_world_pos(idx, alpha))
}

fn citizen_fed_at(world: &World, i: i64) -> Option<f32> {
    let idx = usize::try_from(i).ok()?;
    world.citizen_vitals.get(idx).map(|v| v.fed)
}

fn resource_tile_at(world: &World, i: i64) -> Option<TilePos> {
    let idx = usize::try_from(i).ok()?;
    world.resources.get(idx).map(|r| r.tile_pos)
}

fn resource_amount_at(world: &World, i: i64) -> Option<f32> {
    let idx = usize::try_from(i).ok()?;
    world.resources.get(idx).map(|r| r.amount)
}

/// Decode a flat `[x0, y0, x1, y1, ...]` i32 sequence into Berry resources.
/// Odd-length inputs drop the trailing coordinate. Out-of-range (i16) values
/// are clamped so a bad Godot-side constant can't panic the extension.
fn berry_resources_from_i32_pairs(tiles: &[i32]) -> Vec<Resource> {
    tiles
        .chunks_exact(2)
        .map(|pair| {
            let x = pair[0].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let y = pair[1].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            Resource::new_berry(TilePos::new(x, y))
        })
        .collect()
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct WorldNode {
    _base: Base<Node>,
    world: World,
}

#[godot_api]
impl INode for WorldNode {
    fn init(base: Base<Node>) -> Self {
        Self {
            _base: base,
            world: World::new(),
        }
    }
}

#[godot_api]
impl WorldNode {
    /// Spawn the minimal village for Sprint N4 smoke test and enable a
    /// deterministic random walk so the citizen keeps moving after reaching
    /// any target. `bounds` clamps target tiles to `0..width × 0..height`.
    #[func]
    fn initialize(&mut self, width: i32, height: i32, seed: i64) {
        self.world.spawn_citizen("Alice", TilePos::new(0, 0));
        self.world.enable_random_walk(
            seed as u64,
            MapBounds {
                width: width.clamp(1, i16::MAX as i32) as i16,
                height: height.clamp(1, i16::MAX as i32) as i16,
            },
        );
    }

    #[func]
    fn tick(&mut self) {
        self.world.tick();
    }

    /// Install a walkable grid from a row-major byte buffer: `0` means blocked,
    /// any non-zero byte means walkable. `cells.len()` must equal
    /// `width * height` (silently ignored otherwise — Godot side is trusted).
    #[func]
    fn set_walkable_map(&mut self, width: i32, height: i32, cells: PackedByteArray) {
        let w = width.clamp(0, i16::MAX as i32) as i16;
        let h = height.clamp(0, i16::MAX as i32) as i16;
        let expected = (w as usize) * (h as usize);
        if cells.len() != expected {
            godot_warn!(
                "set_walkable_map: got {} cells but expected {}x{}={}; ignoring",
                cells.len(),
                w,
                h,
                expected
            );
            return;
        }
        let bools: Vec<bool> = (0..cells.len()).map(|i| cells.get(i).unwrap_or(0) != 0).collect();
        self.world.set_walkable_map(w, h, bools);
    }

    #[func]
    fn get_citizen_count(&self) -> i64 {
        self.world.citizens.len() as i64
    }

    #[func]
    fn get_citizen_name(&self, i: i64) -> GString {
        GString::from(citizen_name_at(&self.world, i).unwrap_or(""))
    }

    #[func]
    fn get_citizen_world_pos(&self, i: i64, alpha: f32) -> Vector2 {
        match citizen_world_pos_at(&self.world, i, alpha) {
            Some((x, y)) => Vector2::new(x, y),
            None => Vector2::ZERO,
        }
    }

    /// Seed berry resources from a flat `[x0, y0, x1, y1, ...]` Int32 array.
    /// Replaces any previously-set resources.
    #[func]
    fn set_berry_tiles(&mut self, tiles: PackedInt32Array) {
        let raw: Vec<i32> = (0..tiles.len()).map(|i| tiles.get(i).unwrap_or(0)).collect();
        self.world.set_resources(berry_resources_from_i32_pairs(&raw));
    }

    #[func]
    fn get_resource_count(&self) -> i64 {
        self.world.resources.len() as i64
    }

    /// Tile-space position of resource `i`. Godot side scales by `TILE_SIZE`
    /// (same convention as `get_citizen_world_pos`).
    #[func]
    fn get_resource_world_pos(&self, i: i64) -> Vector2 {
        match resource_tile_at(&self.world, i) {
            Some(t) => Vector2::new(t.x as f32, t.y as f32),
            None => Vector2::ZERO,
        }
    }

    /// Remaining berry amount for resource `i`. Useful for driving visual
    /// regeneration/depletion feedback (scale, color).
    #[func]
    fn get_resource_amount(&self, i: i64) -> f32 {
        resource_amount_at(&self.world, i).unwrap_or(0.0)
    }

    /// Fullness of citizen `i` in `[0.0, 1.0]`. Returns `0.0` for out-of-range
    /// indices (mirrors `get_citizen_world_pos`'s silent fallback).
    #[func]
    fn get_citizen_fed(&self, i: i64) -> f32 {
        citizen_fed_at(&self.world, i).unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_world_with_one_citizen_moving() -> World {
        let mut w = World::new();
        let idx = w.spawn_citizen("Alice", TilePos::new(0, 0));
        w.set_move_target(idx, TilePos::new(10, 0));
        w
    }

    #[test]
    fn name_at_returns_name_for_valid_index() {
        let w = make_world_with_one_citizen_moving();
        assert_eq!(citizen_name_at(&w, 0), Some("Alice"));
    }

    #[test]
    fn name_at_returns_none_for_out_of_range_index() {
        let w = make_world_with_one_citizen_moving();
        assert_eq!(citizen_name_at(&w, 1), None);
        assert_eq!(citizen_name_at(&w, 99), None);
    }

    #[test]
    fn name_at_returns_none_for_negative_index() {
        let w = make_world_with_one_citizen_moving();
        assert_eq!(citizen_name_at(&w, -1), None);
    }

    #[test]
    fn world_pos_at_returns_tuple_for_valid_index() {
        let mut w = make_world_with_one_citizen_moving();
        w.tick(); // prev=(0,0), curr=(1,0)
        let pos = citizen_world_pos_at(&w, 0, 0.5);
        assert!(pos.is_some());
        let (x, y) = pos.unwrap();
        assert!((x - 0.5).abs() < 1e-6);
        assert!(y.abs() < 1e-6);
    }

    #[test]
    fn world_pos_at_returns_none_for_out_of_range_index() {
        let w = make_world_with_one_citizen_moving();
        assert!(citizen_world_pos_at(&w, 1, 0.0).is_none());
        assert!(citizen_world_pos_at(&w, -1, 0.0).is_none());
    }

    #[test]
    fn citizen_fed_at_returns_default_for_new_citizen() {
        let w = make_world_with_one_citizen_moving();
        assert_eq!(citizen_fed_at(&w, 0), Some(1.0));
    }

    #[test]
    fn citizen_fed_at_returns_none_for_bad_index() {
        let w = make_world_with_one_citizen_moving();
        assert_eq!(citizen_fed_at(&w, 5), None);
        assert_eq!(citizen_fed_at(&w, -1), None);
    }

    #[test]
    fn berry_resources_from_i32_pairs_parses_coordinate_pairs() {
        let out = berry_resources_from_i32_pairs(&[3, 4, 10, 12]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].tile_pos, TilePos::new(3, 4));
        assert_eq!(out[1].tile_pos, TilePos::new(10, 12));
    }

    #[test]
    fn berry_resources_from_i32_pairs_drops_unpaired_tail() {
        let out = berry_resources_from_i32_pairs(&[1, 2, 9]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tile_pos, TilePos::new(1, 2));
    }

    #[test]
    fn berry_resources_from_i32_pairs_clamps_out_of_range_to_i16_bounds() {
        let out = berry_resources_from_i32_pairs(&[i32::MAX, i32::MIN]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tile_pos, TilePos::new(i16::MAX, i16::MIN));
    }

    #[test]
    fn resource_tile_at_returns_seeded_position() {
        let mut w = World::new();
        w.set_resources(vec![Resource::new_berry(TilePos::new(7, 9))]);
        assert_eq!(resource_tile_at(&w, 0), Some(TilePos::new(7, 9)));
        assert_eq!(resource_tile_at(&w, 1), None);
        assert_eq!(resource_tile_at(&w, -1), None);
    }

    #[test]
    fn resource_amount_at_returns_none_for_bad_index() {
        let w = World::new();
        assert_eq!(resource_amount_at(&w, 0), None);
    }

    #[test]
    fn world_pos_at_alpha_boundaries_match_prev_and_current() {
        let mut w = make_world_with_one_citizen_moving();
        w.tick(); // prev=(0,0), curr=(1,0)
        let (x0, _) = citizen_world_pos_at(&w, 0, 0.0).unwrap();
        let (x1, _) = citizen_world_pos_at(&w, 0, 1.0).unwrap();
        assert!((x0 - 0.0).abs() < 1e-6);
        assert!((x1 - 1.0).abs() < 1e-6);
    }
}
