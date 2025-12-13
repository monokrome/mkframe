//! Split tree layout system for dividing a window into multiple panes.

use crate::widget::Rect;

/// Unique identifier for a leaf node in the split tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeafId(pub usize);

/// Direction of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Split horizontally (top/bottom).
    Horizontal,
    /// Split vertically (left/right).
    Vertical,
}

/// A node in the split tree.
#[derive(Debug)]
enum SplitNode<T> {
    /// A leaf node containing actual content.
    Leaf { id: LeafId, content: T },
    /// A split node containing two children.
    Split {
        direction: SplitDirection,
        /// Ratio of first child (0.0 to 1.0).
        ratio: f32,
        first: Box<SplitNode<T>>,
        second: Box<SplitNode<T>>,
    },
}

/// A tree of splits managing layout and focus.
pub struct SplitTree<T> {
    root: Option<SplitNode<T>>,
    focused: Option<LeafId>,
    next_id: usize,
}

impl<T> Default for SplitTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SplitTree<T> {
    /// Create an empty split tree.
    pub fn new() -> Self {
        Self {
            root: None,
            focused: None,
            next_id: 0,
        }
    }

    /// Create a split tree with a single leaf.
    pub fn with_root(content: T) -> Self {
        let mut tree = Self::new();
        tree.set_root(content);
        tree
    }

    fn next_leaf_id(&mut self) -> LeafId {
        let id = LeafId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Set the root content (replaces entire tree).
    pub fn set_root(&mut self, content: T) -> LeafId {
        let id = self.next_leaf_id();
        self.root = Some(SplitNode::Leaf { id, content });
        self.focused = Some(id);
        id
    }

    /// Add content by splitting the focused leaf vertically (left/right).
    /// New content goes to the right.
    pub fn split_vertical(&mut self, content: T) -> Option<LeafId> {
        self.split_focused(SplitDirection::Vertical, content)
    }

    /// Add content by splitting the focused leaf horizontally (top/bottom).
    /// New content goes to the bottom.
    pub fn split_horizontal(&mut self, content: T) -> Option<LeafId> {
        self.split_focused(SplitDirection::Horizontal, content)
    }

    /// Split the focused leaf in the given direction.
    fn split_focused(&mut self, direction: SplitDirection, content: T) -> Option<LeafId> {
        let focused_id = self.focused?;
        let new_id = self.next_leaf_id();

        self.root = self
            .root
            .take()
            .map(|node| Self::split_node(node, focused_id, direction, new_id, content));

        self.focused = Some(new_id);
        Some(new_id)
    }

    fn split_node(
        node: SplitNode<T>,
        target_id: LeafId,
        direction: SplitDirection,
        new_id: LeafId,
        content: T,
    ) -> SplitNode<T> {
        match node {
            SplitNode::Leaf {
                id,
                content: old_content,
            } if id == target_id => SplitNode::Split {
                direction,
                ratio: 0.5,
                first: Box::new(SplitNode::Leaf {
                    id,
                    content: old_content,
                }),
                second: Box::new(SplitNode::Leaf {
                    id: new_id,
                    content,
                }),
            },
            SplitNode::Leaf { .. } => node,
            SplitNode::Split {
                direction: d,
                ratio,
                first,
                second,
            } => {
                // Only recurse into the subtree that contains the target
                if Self::node_contains_leaf(&first, target_id) {
                    SplitNode::Split {
                        direction: d,
                        ratio,
                        first: Box::new(Self::split_node(
                            *first, target_id, direction, new_id, content,
                        )),
                        second,
                    }
                } else {
                    SplitNode::Split {
                        direction: d,
                        ratio,
                        first,
                        second: Box::new(Self::split_node(
                            *second, target_id, direction, new_id, content,
                        )),
                    }
                }
            }
        }
    }

    /// Get the currently focused leaf ID.
    pub fn focused(&self) -> Option<LeafId> {
        self.focused
    }

    /// Set focus to a specific leaf.
    pub fn set_focused(&mut self, id: LeafId) {
        if self.contains_leaf(id) {
            self.focused = Some(id);
        }
    }

    /// Check if a leaf ID exists in the tree.
    pub fn contains_leaf(&self, id: LeafId) -> bool {
        self.root
            .as_ref()
            .is_some_and(|n| Self::node_contains_leaf(n, id))
    }

    fn node_contains_leaf(node: &SplitNode<T>, id: LeafId) -> bool {
        match node {
            SplitNode::Leaf { id: leaf_id, .. } => *leaf_id == id,
            SplitNode::Split { first, second, .. } => {
                Self::node_contains_leaf(first, id) || Self::node_contains_leaf(second, id)
            }
        }
    }

    /// Get a reference to the focused content.
    pub fn focused_content(&self) -> Option<&T> {
        let focused_id = self.focused?;
        self.get(focused_id)
    }

    /// Get a mutable reference to the focused content.
    pub fn focused_content_mut(&mut self) -> Option<&mut T> {
        let focused_id = self.focused?;
        self.get_mut(focused_id)
    }

    /// Get a reference to content by ID.
    pub fn get(&self, id: LeafId) -> Option<&T> {
        self.root.as_ref().and_then(|n| Self::node_get(n, id))
    }

    fn node_get(node: &SplitNode<T>, id: LeafId) -> Option<&T> {
        match node {
            SplitNode::Leaf {
                id: leaf_id,
                content,
            } if *leaf_id == id => Some(content),
            SplitNode::Leaf { .. } => None,
            SplitNode::Split { first, second, .. } => {
                Self::node_get(first, id).or_else(|| Self::node_get(second, id))
            }
        }
    }

    /// Get a mutable reference to content by ID.
    pub fn get_mut(&mut self, id: LeafId) -> Option<&mut T> {
        self.root.as_mut().and_then(|n| Self::node_get_mut(n, id))
    }

    fn node_get_mut(node: &mut SplitNode<T>, id: LeafId) -> Option<&mut T> {
        match node {
            SplitNode::Leaf {
                id: leaf_id,
                content,
            } if *leaf_id == id => Some(content),
            SplitNode::Leaf { .. } => None,
            SplitNode::Split { first, second, .. } => {
                Self::node_get_mut(first, id).or_else(|| Self::node_get_mut(second, id))
            }
        }
    }

    /// Get the number of leaves in the tree.
    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, Self::node_len)
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    fn node_len(node: &SplitNode<T>) -> usize {
        match node {
            SplitNode::Leaf { .. } => 1,
            SplitNode::Split { first, second, .. } => {
                Self::node_len(first) + Self::node_len(second)
            }
        }
    }

    /// Iterate over all leaves with their computed regions.
    pub fn layout(&self, bounds: Rect) -> Vec<(LeafId, Rect)> {
        let mut result = Vec::new();
        if let Some(ref node) = self.root {
            Self::layout_node(node, bounds, &mut result);
        }
        result
    }

    fn layout_node(node: &SplitNode<T>, bounds: Rect, result: &mut Vec<(LeafId, Rect)>) {
        match node {
            SplitNode::Leaf { id, .. } => {
                result.push((*id, bounds));
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_bounds, second_bounds) = Self::split_bounds(bounds, *direction, *ratio);
                Self::layout_node(first, first_bounds, result);
                Self::layout_node(second, second_bounds, result);
            }
        }
    }

    /// Find the leaf at the given position within the given bounds.
    /// Returns the LeafId and its Rect if found.
    pub fn find_at_position(&self, bounds: Rect, x: f64, y: f64) -> Option<(LeafId, Rect)> {
        let layout = self.layout(bounds);
        for (id, rect) in layout {
            let x_in = x >= rect.x as f64 && x < (rect.x + rect.width as i32) as f64;
            let y_in = y >= rect.y as f64 && y < (rect.y + rect.height as i32) as f64;
            if x_in && y_in {
                return Some((id, rect));
            }
        }
        None
    }

    fn split_bounds(bounds: Rect, direction: SplitDirection, ratio: f32) -> (Rect, Rect) {
        match direction {
            SplitDirection::Vertical => {
                let first_width = ((bounds.width as f32) * ratio) as u32;
                let second_width = bounds.width.saturating_sub(first_width);
                (
                    Rect::new(bounds.x, bounds.y, first_width, bounds.height),
                    Rect::new(
                        bounds.x + first_width as i32,
                        bounds.y,
                        second_width,
                        bounds.height,
                    ),
                )
            }
            SplitDirection::Horizontal => {
                let first_height = ((bounds.height as f32) * ratio) as u32;
                let second_height = bounds.height.saturating_sub(first_height);
                (
                    Rect::new(bounds.x, bounds.y, bounds.width, first_height),
                    Rect::new(
                        bounds.x,
                        bounds.y + first_height as i32,
                        bounds.width,
                        second_height,
                    ),
                )
            }
        }
    }

    /// Render all leaves using a callback.
    pub fn render<F>(&self, bounds: Rect, mut render_fn: F)
    where
        F: FnMut(LeafId, Rect, &T, bool),
    {
        let focused = self.focused;
        if let Some(ref node) = self.root {
            Self::render_node(node, bounds, focused, &mut render_fn);
        }
    }

    fn render_node<F>(node: &SplitNode<T>, bounds: Rect, focused: Option<LeafId>, render_fn: &mut F)
    where
        F: FnMut(LeafId, Rect, &T, bool),
    {
        match node {
            SplitNode::Leaf { id, content } => {
                let is_focused = focused == Some(*id);
                render_fn(*id, bounds, content, is_focused);
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_bounds, second_bounds) = Self::split_bounds(bounds, *direction, *ratio);
                Self::render_node(first, first_bounds, focused, render_fn);
                Self::render_node(second, second_bounds, focused, render_fn);
            }
        }
    }

    /// Move focus in a direction relative to current focus.
    pub fn focus_direction(&mut self, direction: SplitDirection, forward: bool) -> bool {
        let Some(focused_id) = self.focused else {
            return false;
        };
        if self.root.is_none() {
            return false;
        };

        // Get layout to find positions
        let bounds = Rect::new(0, 0, 1000, 1000); // Arbitrary for relative positioning
        let layout = self.layout(bounds);

        // Find focused leaf's position
        let Some((_, focused_rect)) = layout.iter().find(|(id, _)| *id == focused_id) else {
            return false;
        };

        // Find best candidate in the given direction
        let focused_center_x = focused_rect.x + focused_rect.width as i32 / 2;
        let focused_center_y = focused_rect.y + focused_rect.height as i32 / 2;

        let mut best: Option<(LeafId, i32)> = None;

        for (id, rect) in &layout {
            if *id == focused_id {
                continue;
            }

            let center_x = rect.x + rect.width as i32 / 2;
            let center_y = rect.y + rect.height as i32 / 2;

            let is_valid = match (direction, forward) {
                (SplitDirection::Horizontal, true) => center_y > focused_center_y, // Down
                (SplitDirection::Horizontal, false) => center_y < focused_center_y, // Up
                (SplitDirection::Vertical, true) => center_x > focused_center_x,   // Right
                (SplitDirection::Vertical, false) => center_x < focused_center_x,  // Left
            };

            if !is_valid {
                continue;
            }

            let distance = match direction {
                SplitDirection::Horizontal => (center_y - focused_center_y).abs(),
                SplitDirection::Vertical => (center_x - focused_center_x).abs(),
            };

            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((*id, distance));
            }
        }

        if let Some((id, _)) = best {
            self.focused = Some(id);
            true
        } else {
            false
        }
    }

    /// Focus the leaf to the left.
    pub fn focus_left(&mut self) -> bool {
        self.focus_direction(SplitDirection::Vertical, false)
    }

    /// Focus the leaf to the right.
    pub fn focus_right(&mut self) -> bool {
        self.focus_direction(SplitDirection::Vertical, true)
    }

    /// Focus the leaf above.
    pub fn focus_up(&mut self) -> bool {
        self.focus_direction(SplitDirection::Horizontal, false)
    }

    /// Focus the leaf below.
    pub fn focus_down(&mut self) -> bool {
        self.focus_direction(SplitDirection::Horizontal, true)
    }

    /// Close the focused leaf, returning its content.
    /// Focus moves to a sibling if possible.
    pub fn close_focused(&mut self) -> Option<T> {
        let focused_id = self.focused?;
        let (new_root, removed, new_focus) = Self::remove_leaf(self.root.take()?, focused_id)?;
        self.root = new_root;
        self.focused = new_focus;
        Some(removed)
    }

    fn remove_leaf(
        node: SplitNode<T>,
        target: LeafId,
    ) -> Option<(Option<SplitNode<T>>, T, Option<LeafId>)> {
        match node {
            SplitNode::Leaf { id, content } if id == target => Some((None, content, None)),
            SplitNode::Leaf { .. } => None,
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                // Check which subtree contains the target
                let first_contains = Self::node_contains_leaf(&first, target);
                let second_contains = Self::node_contains_leaf(&second, target);

                if first_contains {
                    let (new_first, removed, _) = Self::remove_leaf(*first, target)?;
                    let new_focus = Self::first_leaf_id(&second);
                    match new_first {
                        Some(f) => Some((
                            Some(SplitNode::Split {
                                direction,
                                ratio,
                                first: Box::new(f),
                                second,
                            }),
                            removed,
                            new_focus,
                        )),
                        None => Some((Some(*second), removed, new_focus)),
                    }
                } else if second_contains {
                    let (new_second, removed, _) = Self::remove_leaf(*second, target)?;
                    let new_focus = Self::first_leaf_id(&first);
                    match new_second {
                        Some(s) => Some((
                            Some(SplitNode::Split {
                                direction,
                                ratio,
                                first,
                                second: Box::new(s),
                            }),
                            removed,
                            new_focus,
                        )),
                        None => Some((Some(*first), removed, new_focus)),
                    }
                } else {
                    None
                }
            }
        }
    }

    fn first_leaf_id(node: &SplitNode<T>) -> Option<LeafId> {
        match node {
            SplitNode::Leaf { id, .. } => Some(*id),
            SplitNode::Split { first, .. } => Self::first_leaf_id(first),
        }
    }

    /// Get all leaf IDs in order.
    pub fn leaf_ids(&self) -> Vec<LeafId> {
        let mut ids = Vec::new();
        if let Some(ref node) = self.root {
            Self::collect_leaf_ids(node, &mut ids);
        }
        ids
    }

    fn collect_leaf_ids(node: &SplitNode<T>, ids: &mut Vec<LeafId>) {
        match node {
            SplitNode::Leaf { id, .. } => ids.push(*id),
            SplitNode::Split { first, second, .. } => {
                Self::collect_leaf_ids(first, ids);
                Self::collect_leaf_ids(second, ids);
            }
        }
    }

    /// Cycle focus to the next leaf.
    pub fn focus_next(&mut self) -> bool {
        let ids = self.leaf_ids();
        if ids.is_empty() {
            return false;
        }

        let current_idx = self
            .focused
            .and_then(|f| ids.iter().position(|id| *id == f))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % ids.len();
        self.focused = Some(ids[next_idx]);
        true
    }

    /// Cycle focus to the previous leaf.
    pub fn focus_prev(&mut self) -> bool {
        let ids = self.leaf_ids();
        if ids.is_empty() {
            return false;
        }

        let current_idx = self
            .focused
            .and_then(|f| ids.iter().position(|id| *id == f))
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            ids.len() - 1
        } else {
            current_idx - 1
        };
        self.focused = Some(ids[prev_idx]);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tree_is_empty() {
        let tree: SplitTree<i32> = SplitTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert!(tree.focused().is_none());
    }

    #[test]
    fn test_with_root() {
        let tree = SplitTree::with_root(42);
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);
        assert!(tree.focused().is_some());
        assert_eq!(tree.focused_content(), Some(&42));
    }

    #[test]
    fn test_set_root() {
        let mut tree = SplitTree::new();
        let id = tree.set_root("hello");
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.focused(), Some(id));
        assert_eq!(tree.get(id), Some(&"hello"));
    }

    #[test]
    fn test_split_vertical() {
        let mut tree = SplitTree::with_root(1);
        let first_id = tree.focused().unwrap();

        let second_id = tree.split_vertical(2).unwrap();

        assert_eq!(tree.len(), 2);
        assert_eq!(tree.get(first_id), Some(&1));
        assert_eq!(tree.get(second_id), Some(&2));
        assert_eq!(tree.focused(), Some(second_id)); // Focus moves to new leaf
    }

    #[test]
    fn test_split_horizontal() {
        let mut tree = SplitTree::with_root("top");
        let first_id = tree.focused().unwrap();

        let second_id = tree.split_horizontal("bottom").unwrap();

        assert_eq!(tree.len(), 2);
        assert_eq!(tree.get(first_id), Some(&"top"));
        assert_eq!(tree.get(second_id), Some(&"bottom"));
    }

    #[test]
    fn test_multiple_splits() {
        let mut tree = SplitTree::with_root(1);
        tree.split_vertical(2);
        tree.split_horizontal(3);
        tree.split_vertical(4);

        assert_eq!(tree.len(), 4);
    }

    #[test]
    fn test_get_mut() {
        let mut tree = SplitTree::with_root(10);
        let id = tree.focused().unwrap();

        if let Some(content) = tree.get_mut(id) {
            *content = 20;
        }

        assert_eq!(tree.get(id), Some(&20));
    }

    #[test]
    fn test_focused_content_mut() {
        let mut tree = SplitTree::with_root(100);

        if let Some(content) = tree.focused_content_mut() {
            *content = 200;
        }

        assert_eq!(tree.focused_content(), Some(&200));
    }

    #[test]
    fn test_contains_leaf() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        let id2 = tree.split_vertical(2).unwrap();

        assert!(tree.contains_leaf(id1));
        assert!(tree.contains_leaf(id2));
        assert!(!tree.contains_leaf(LeafId(999)));
    }

    #[test]
    fn test_set_focused() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        let id2 = tree.split_vertical(2).unwrap();

        assert_eq!(tree.focused(), Some(id2));

        tree.set_focused(id1);
        assert_eq!(tree.focused(), Some(id1));

        // Setting invalid ID should not change focus
        tree.set_focused(LeafId(999));
        assert_eq!(tree.focused(), Some(id1));
    }

    #[test]
    fn test_close_focused() {
        let mut tree = SplitTree::with_root(1);
        tree.split_vertical(2);

        assert_eq!(tree.len(), 2);

        let removed = tree.close_focused();
        assert_eq!(removed, Some(2));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.focused_content(), Some(&1));
    }

    #[test]
    fn test_close_last_leaf() {
        let mut tree = SplitTree::with_root(42);
        let removed = tree.close_focused();
        assert_eq!(removed, Some(42));
        assert!(tree.is_empty());
        assert!(tree.focused().is_none());
    }

    #[test]
    fn test_layout_single_leaf() {
        let tree = SplitTree::with_root("content");
        let bounds = Rect::new(0, 0, 100, 100);
        let layout = tree.layout(bounds);

        assert_eq!(layout.len(), 1);
        let (_, rect) = layout[0];
        assert_eq!(rect, bounds);
    }

    #[test]
    fn test_layout_vertical_split() {
        let mut tree = SplitTree::with_root(1);
        tree.split_vertical(2);

        let bounds = Rect::new(0, 0, 100, 100);
        let layout = tree.layout(bounds);

        assert_eq!(layout.len(), 2);

        // First should be left half
        let (_, rect1) = layout[0];
        assert_eq!(rect1.x, 0);
        assert_eq!(rect1.width, 50);

        // Second should be right half
        let (_, rect2) = layout[1];
        assert_eq!(rect2.x, 50);
        assert_eq!(rect2.width, 50);
    }

    #[test]
    fn test_layout_horizontal_split() {
        let mut tree = SplitTree::with_root(1);
        tree.split_horizontal(2);

        let bounds = Rect::new(0, 0, 100, 100);
        let layout = tree.layout(bounds);

        assert_eq!(layout.len(), 2);

        // First should be top half
        let (_, rect1) = layout[0];
        assert_eq!(rect1.y, 0);
        assert_eq!(rect1.height, 50);

        // Second should be bottom half
        let (_, rect2) = layout[1];
        assert_eq!(rect2.y, 50);
        assert_eq!(rect2.height, 50);
    }

    #[test]
    fn test_leaf_ids() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        let id2 = tree.split_vertical(2).unwrap();
        let id3 = tree.split_horizontal(3).unwrap();

        let ids = tree.leaf_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_focus_next() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        let id2 = tree.split_vertical(2).unwrap();
        let id3 = tree.split_vertical(3).unwrap();

        // Currently at id3
        assert_eq!(tree.focused(), Some(id3));

        // Next should wrap to id1
        tree.focus_next();
        assert_eq!(tree.focused(), Some(id1));

        // Next should be id2
        tree.focus_next();
        assert_eq!(tree.focused(), Some(id2));
    }

    #[test]
    fn test_focus_prev() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        let _id2 = tree.split_vertical(2).unwrap();
        let id3 = tree.split_vertical(3).unwrap();

        tree.set_focused(id1);

        // Prev should wrap to id3
        tree.focus_prev();
        assert_eq!(tree.focused(), Some(id3));
    }

    #[test]
    fn test_focus_left_right() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        tree.set_focused(id1);
        let id2 = tree.split_vertical(2).unwrap();

        // Currently at id2 (right)
        tree.focus_left();
        assert_eq!(tree.focused(), Some(id1));

        tree.focus_right();
        assert_eq!(tree.focused(), Some(id2));
    }

    #[test]
    fn test_focus_up_down() {
        let mut tree = SplitTree::with_root(1);
        let id1 = tree.focused().unwrap();
        tree.set_focused(id1);
        let id2 = tree.split_horizontal(2).unwrap();

        // Currently at id2 (bottom)
        tree.focus_up();
        assert_eq!(tree.focused(), Some(id1));

        tree.focus_down();
        assert_eq!(tree.focused(), Some(id2));
    }

    #[test]
    fn test_render() {
        let mut tree = SplitTree::with_root("a");
        tree.split_vertical("b");

        let bounds = Rect::new(0, 0, 100, 100);
        let mut rendered = Vec::new();

        tree.render(bounds, |id, rect, content, is_focused| {
            rendered.push((id, rect, *content, is_focused));
        });

        assert_eq!(rendered.len(), 2);

        // One should be focused
        let focused_count = rendered.iter().filter(|(_, _, _, f)| *f).count();
        assert_eq!(focused_count, 1);
    }

    #[test]
    fn test_default() {
        let tree: SplitTree<i32> = SplitTree::default();
        assert!(tree.is_empty());
    }
}
