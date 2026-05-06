use crate::app::{DraftCage, PuzzleView};
use crate::cage_index::{cage_anchor, cage_at};

pub struct MenuContext {
    pub cell: (usize, usize),
    pub view: PuzzleView,
    pub drafts: Vec<DraftCage>,
}

#[derive(Clone, Debug)]
pub struct FlipTarget {
    pub anchor: (usize, usize),
    pub label: String,
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ContextMenuItems {
    pub set_operation: bool,
    pub make_singleton: bool,
    pub uncage: bool,
    pub delete_cage: bool,
    pub adjacent_targets: Vec<FlipTarget>,
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

    let adjacent_targets = committed_idx.map_or_else(Vec::new, |src_idx| {
        adjacent_committed_cages(&ctx.view, (r, c), src_idx)
            .into_iter()
            .map(|idx| {
                let anchor = cage_anchor(&ctx.view.cages[idx]);
                FlipTarget {
                    anchor,
                    label: cage_label(&ctx.view, idx),
                }
            })
            .collect()
    });

    ContextMenuItems {
        set_operation,
        make_singleton,
        uncage,
        delete_cage,
        adjacent_targets,
    }
}

fn adjacent_committed_cages(view: &PuzzleView, cell: (usize, usize), src_idx: usize) -> Vec<usize> {
    let (r, c) = cell;
    let n = view.n;
    let mut result: Vec<usize> = Vec::new();

    let neighbors: &[(isize, isize)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dr, dc) in neighbors {
        let Some(nr) = r.checked_add_signed(*dr) else {
            continue;
        };
        let Some(nc) = c.checked_add_signed(*dc) else {
            continue;
        };
        if nr >= n || nc >= n {
            continue;
        }
        if let Some(idx) = cage_at(view, nr, nc) {
            if idx != src_idx && !result.contains(&idx) {
                result.push(idx);
            }
        }
    }
    result
}

fn cage_label(view: &PuzzleView, idx: usize) -> String {
    view.cages.get(idx).map_or_else(String::new, |cage| {
        let anchor = cage_anchor(cage);
        format!("({},{})", anchor.0, anchor.1)
    })
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
        assert!(items.adjacent_targets.is_empty());
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
        assert!(items.adjacent_targets.is_empty());
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
        assert!(items.adjacent_targets.is_empty());
    }

    #[test]
    fn multi_cell_cage_adjacent_to_another_shows_full_menu() {
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
        assert!(!items.adjacent_targets.is_empty());
    }
}
