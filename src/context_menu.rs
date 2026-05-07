use crate::app::{DraftCage, PuzzleView};
use crate::cage_edit::legal_move_targets;
use crate::cage_index::cage_at;

/// Display label for the keyboard shortcut bound to each right-click menu item.
/// These mirror the bindings in [`crate::app::dispatch_key`] and the Enter/Splinter
/// handlers in `install_keydown_handler`.
pub mod shortcut {
    pub const SET_OPERATION: &str = "Enter";
    pub const MAKE_SINGLETON: &str = "Space";
    pub const UNCAGE: &str = "Esc";
    pub const DELETE_CAGE: &str = "Del";
    pub const MOVE_CELL: &str = "M";
}

pub struct MenuContext {
    pub cell: (usize, usize),
    pub view: PuzzleView,
    pub drafts: Vec<DraftCage>,
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ContextMenuItems {
    pub set_operation: bool,
    pub make_singleton: bool,
    pub uncage: bool,
    pub delete_cage: bool,
    pub can_move: bool,
}

#[must_use]
pub fn menu_items_for(ctx: &MenuContext) -> ContextMenuItems {
    let (r, c) = ctx.cell;
    let committed_idx = cage_at(&ctx.view, r, c);

    let set_operation = committed_idx.is_some();
    let uncage = committed_idx.is_some();
    let delete_cage = committed_idx.is_some();

    let is_committed_singleton = committed_idx.is_some_and(|idx| {
        ctx.view
            .cages
            .get(idx)
            .is_some_and(|cage| cage.cells.len() == 1)
    });
    let is_singleton_draft = ctx
        .drafts
        .iter()
        .any(|d| d.cells.len() == 1 && d.cells.contains(&(r, c)));
    let make_singleton = !is_committed_singleton && !is_singleton_draft;

    let can_move = !legal_move_targets(&ctx.view, (r, c)).is_empty();

    ContextMenuItems {
        set_operation,
        make_singleton,
        uncage,
        delete_cage,
        can_move,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{CageView, OpKind};

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Add,
            target: 3,
        }
    }

    fn view(n: usize, cages: Vec<CageView>) -> PuzzleView {
        PuzzleView {
            n,
            cells: vec![vec![Vec::new(); n]; n],
            cages,
            diff: crate::diff::PuzzleDiff::default(),
        }
    }

    #[test]
    fn uncovered_cell_with_no_neighbors_caged_shows_only_make_singleton() {
        let v = view(3, vec![]);
        let ctx = MenuContext {
            cell: (1, 1),
            view: v,
            drafts: vec![],
        };
        let items = menu_items_for(&ctx);
        assert!(!items.set_operation);
        assert!(items.make_singleton);
        assert!(!items.uncage);
        assert!(!items.delete_cage);
        assert!(!items.can_move);
    }

    #[test]
    fn uncovered_cell_adjacent_to_committed_cage_shows_make_singleton() {
        let v = view(3, vec![cage(&[(0, 0)])]);
        let ctx = MenuContext {
            cell: (1, 0),
            view: v,
            drafts: vec![],
        };
        let items = menu_items_for(&ctx);
        assert!(!items.set_operation);
        assert!(items.make_singleton);
        assert!(!items.uncage);
        assert!(!items.delete_cage);
        assert!(!items.can_move);
    }

    #[test]
    fn singleton_cage_shows_set_operation_uncage_delete_not_make_singleton() {
        let v = view(3, vec![cage(&[(1, 1)])]);
        let ctx = MenuContext {
            cell: (1, 1),
            view: v,
            drafts: vec![],
        };
        let items = menu_items_for(&ctx);
        assert!(items.set_operation);
        assert!(!items.make_singleton);
        assert!(items.uncage);
        assert!(items.delete_cage);
        // Singleton cage with no adjacent cages: can_move is false
        assert!(!items.can_move);
    }

    #[test]
    fn multi_cell_cage_adjacent_to_another_shows_can_move() {
        let v = view(3, vec![cage(&[(0, 0), (0, 1)]), cage(&[(1, 0), (1, 1)])]);
        let ctx = MenuContext {
            cell: (0, 0),
            view: v,
            drafts: vec![],
        };
        let items = menu_items_for(&ctx);
        assert!(items.set_operation);
        assert!(items.make_singleton);
        assert!(items.uncage);
        assert!(items.delete_cage);
        assert!(items.can_move);
    }
}
