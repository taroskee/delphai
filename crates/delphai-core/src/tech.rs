//! Technology tree for the first civilization era.
//!
//! Research points accumulate whenever a citizen gathers food.
//! When enough points are collected, the next unlocked tech fires its effect.

pub type TechNodeId = u32;

#[derive(Debug, Clone)]
pub struct TechNode {
    pub id: TechNodeId,
    pub name: &'static str,
    pub required_points: u32,
    pub unlocked: bool,
}

impl TechNode {
    const fn new(id: TechNodeId, name: &'static str, required_points: u32) -> Self {
        Self { id, name, required_points, unlocked: false }
    }
}

#[derive(Debug, Clone)]
pub struct TechTree {
    pub nodes: Vec<TechNode>,
    pub research_points: u32,
}

impl TechTree {
    pub fn new() -> Self {
        Self {
            nodes: vec![
                TechNode::new(0, "stone_tools", 50),
                TechNode::new(1, "agriculture", 200),
                TechNode::new(2, "bronze_tools", 500),
            ],
            research_points: 0,
        }
    }

    /// Add `delta` research points and unlock the next node if the threshold is met.
    /// Returns the id of any newly unlocked tech.
    pub fn add_points(&mut self, delta: u32) -> Option<TechNodeId> {
        self.research_points = self.research_points.saturating_add(delta);
        self.try_unlock()
    }

    /// Unlock all locked nodes whose threshold is now reached (in order).
    /// Returns the id of the last newly-unlocked tech, or None if nothing changed.
    fn try_unlock(&mut self) -> Option<TechNodeId> {
        let mut last = None;
        for node in &mut self.nodes {
            if !node.unlocked && self.research_points >= node.required_points {
                node.unlocked = true;
                last = Some(node.id);
            }
        }
        last
    }

    pub fn is_unlocked(&self, id: TechNodeId) -> bool {
        self.nodes.iter().any(|n| n.id == id && n.unlocked)
    }

    /// Name of the next tech not yet unlocked, or `None` if everything is researched.
    pub fn next_tech_name(&self) -> Option<&'static str> {
        self.nodes.iter().find(|n| !n.unlocked).map(|n| n.name)
    }

    /// Required points for the next unresearched node.
    pub fn next_required_points(&self) -> Option<u32> {
        self.nodes.iter().find(|n| !n.unlocked).map(|n| n.required_points)
    }
}

impl Default for TechTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_stone_tools_locked() {
        let tree = TechTree::new();
        assert!(!tree.is_unlocked(0));
        assert_eq!(tree.next_tech_name(), Some("stone_tools"));
    }

    #[test]
    fn add_points_below_threshold_does_not_unlock() {
        let mut tree = TechTree::new();
        let result = tree.add_points(49);
        assert!(result.is_none());
        assert!(!tree.is_unlocked(0));
    }

    #[test]
    fn add_points_at_threshold_unlocks_stone_tools() {
        let mut tree = TechTree::new();
        let result = tree.add_points(50);
        assert_eq!(result, Some(0));
        assert!(tree.is_unlocked(0));
    }

    #[test]
    fn unlocked_twice_returns_none_second_time() {
        let mut tree = TechTree::new();
        tree.add_points(50);
        let second = tree.add_points(10);
        assert!(second.is_none(), "already-unlocked tech must not fire again");
    }

    #[test]
    fn next_tech_name_is_none_when_all_unlocked() {
        let mut tree = TechTree::new();
        tree.add_points(500); // enough to unlock all three techs
        assert!(tree.next_tech_name().is_none());
    }

    #[test]
    fn second_tech_agriculture_unlocks_at_200() {
        let mut tree = TechTree::new();
        tree.add_points(199);
        assert!(tree.next_tech_name() == Some("agriculture"));
        tree.add_points(1);
        assert!(tree.is_unlocked(1));
    }

    #[test]
    fn bronze_tools_unlocks_at_500() {
        let mut tree = TechTree::new();
        tree.add_points(499);
        assert!(!tree.is_unlocked(2));
        tree.add_points(1);
        assert!(tree.is_unlocked(2));
    }
}
