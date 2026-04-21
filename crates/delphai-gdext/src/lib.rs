#![allow(clippy::result_large_err)]
// godot 0.2 macros generate `Result<_, CallError>` where CallError is >160B;
// the lint can't see through the macro so we silence it crate-wide.

use delphai_core::pathfinding::TilePos;
use delphai_core::world::World;
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
    /// Spawn the minimal village for Sprint N4 smoke test: one citizen walking
    /// along x. Called from GDScript `_ready`.
    #[func]
    fn initialize(&mut self) {
        let idx = self.world.spawn_citizen("Alice", TilePos::new(0, 0));
        self.world.set_move_target(idx, TilePos::new(10, 0));
    }

    #[func]
    fn tick(&mut self) {
        self.world.tick();
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
    fn world_pos_at_alpha_boundaries_match_prev_and_current() {
        let mut w = make_world_with_one_citizen_moving();
        w.tick(); // prev=(0,0), curr=(1,0)
        let (x0, _) = citizen_world_pos_at(&w, 0, 0.0).unwrap();
        let (x1, _) = citizen_world_pos_at(&w, 0, 1.0).unwrap();
        assert!((x0 - 0.0).abs() < 1e-6);
        assert!((x1 - 1.0).abs() < 1e-6);
    }
}
