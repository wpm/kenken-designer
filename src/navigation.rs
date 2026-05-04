use crate::app::CageView;
use crate::cage_index::cage_anchor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Tab,
    ShiftTab,
}

impl NavKey {
    #[must_use]
    pub const fn from_key(key: &str, shift: bool) -> Option<Self> {
        match (key.as_bytes(), shift) {
            (b"ArrowUp", _) => Some(Self::ArrowUp),
            (b"ArrowDown", _) => Some(Self::ArrowDown),
            (b"ArrowLeft", _) => Some(Self::ArrowLeft),
            (b"ArrowRight", _) => Some(Self::ArrowRight),
            (b"Tab", false) => Some(Self::Tab),
            (b"Tab", true) => Some(Self::ShiftTab),
            _ => None,
        }
    }
}

#[must_use]
pub fn next_state(
    cursor: (usize, usize),
    active_cage: Option<usize>,
    n: usize,
    cages: &[CageView],
    key: NavKey,
) -> ((usize, usize), Option<usize>) {
    match key {
        NavKey::ArrowUp | NavKey::ArrowDown | NavKey::ArrowLeft | NavKey::ArrowRight => {
            let cursor = move_cursor(cursor, n, key);
            let active = cages.iter().position(|c| c.cells.contains(&cursor));
            (cursor, active)
        }
        NavKey::Tab | NavKey::ShiftTab => {
            if cages.is_empty() {
                return (cursor, active_cage);
            }
            let next_idx = cycle_cage_idx(active_cage, cages.len(), matches!(key, NavKey::Tab));
            (cage_anchor(&cages[next_idx]), Some(next_idx))
        }
    }
}

const fn cycle_cage_idx(active: Option<usize>, len: usize, forward: bool) -> usize {
    match (active, forward) {
        (None, true) => 0,
        (None, false) => len - 1,
        (Some(i), true) => (i + 1) % len,
        (Some(i), false) => (i + len - 1) % len,
    }
}

const fn move_cursor((r, c): (usize, usize), n: usize, key: NavKey) -> (usize, usize) {
    let n_max = n.saturating_sub(1);
    match key {
        NavKey::ArrowUp => (r.saturating_sub(1), c),
        NavKey::ArrowDown => {
            let nr = if r + 1 < n { r + 1 } else { n_max };
            (nr, c)
        }
        NavKey::ArrowLeft => (r, c.saturating_sub(1)),
        NavKey::ArrowRight => {
            let nc = if c + 1 < n { c + 1 } else { n_max };
            (r, nc)
        }
        _ => (r, c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpKind;

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Given,
            target: 0,
        }
    }

    fn cages_basic() -> Vec<CageView> {
        vec![
            cage(&[(0, 0), (0, 1)]),
            cage(&[(1, 0), (1, 1)]),
            cage(&[(2, 2)]),
        ]
    }

    #[test]
    fn from_key_recognises_arrows_regardless_of_shift() {
        assert_eq!(NavKey::from_key("ArrowUp", false), Some(NavKey::ArrowUp));
        assert_eq!(NavKey::from_key("ArrowUp", true), Some(NavKey::ArrowUp));
        assert_eq!(
            NavKey::from_key("ArrowDown", false),
            Some(NavKey::ArrowDown)
        );
        assert_eq!(
            NavKey::from_key("ArrowLeft", false),
            Some(NavKey::ArrowLeft)
        );
        assert_eq!(
            NavKey::from_key("ArrowRight", false),
            Some(NavKey::ArrowRight)
        );
    }

    #[test]
    fn from_key_distinguishes_tab_and_shift_tab() {
        assert_eq!(NavKey::from_key("Tab", false), Some(NavKey::Tab));
        assert_eq!(NavKey::from_key("Tab", true), Some(NavKey::ShiftTab));
    }

    #[test]
    fn from_key_returns_none_for_unrelated_keys() {
        assert_eq!(NavKey::from_key("a", false), None);
        assert_eq!(NavKey::from_key("Enter", false), None);
        assert_eq!(NavKey::from_key("", false), None);
    }

    #[test]
    fn arrow_up_clamps_at_top_edge() {
        let cages = cages_basic();
        let (cursor, _) = next_state((0, 1), None, 3, &cages, NavKey::ArrowUp);
        assert_eq!(cursor, (0, 1));
    }

    #[test]
    fn arrow_down_clamps_at_bottom_edge() {
        let cages = cages_basic();
        let (cursor, _) = next_state((2, 2), None, 3, &cages, NavKey::ArrowDown);
        assert_eq!(cursor, (2, 2));
    }

    #[test]
    fn arrow_left_clamps_at_left_edge() {
        let cages = cages_basic();
        let (cursor, _) = next_state((1, 0), None, 3, &cages, NavKey::ArrowLeft);
        assert_eq!(cursor, (1, 0));
    }

    #[test]
    fn arrow_right_clamps_at_right_edge() {
        let cages = cages_basic();
        let (cursor, _) = next_state((1, 2), None, 3, &cages, NavKey::ArrowRight);
        assert_eq!(cursor, (1, 2));
    }

    #[test]
    fn arrow_moves_one_cell_within_grid() {
        let cages = cages_basic();
        assert_eq!(
            next_state((1, 1), None, 3, &cages, NavKey::ArrowUp).0,
            (0, 1)
        );
        assert_eq!(
            next_state((1, 1), None, 3, &cages, NavKey::ArrowDown).0,
            (2, 1)
        );
        assert_eq!(
            next_state((1, 1), None, 3, &cages, NavKey::ArrowLeft).0,
            (1, 0)
        );
        assert_eq!(
            next_state((1, 1), None, 3, &cages, NavKey::ArrowRight).0,
            (1, 2)
        );
    }

    #[test]
    fn arrow_updates_active_cage_to_target_cell_cage() {
        let cages = cages_basic();
        let (cursor, active) = next_state((0, 1), None, 3, &cages, NavKey::ArrowDown);
        assert_eq!(cursor, (1, 1));
        assert_eq!(active, Some(1));
    }

    #[test]
    fn arrow_clears_active_cage_when_target_cell_uncaged() {
        let cages = vec![cage(&[(0, 0)])];
        let (cursor, active) = next_state((0, 0), Some(0), 3, &cages, NavKey::ArrowRight);
        assert_eq!(cursor, (0, 1));
        assert_eq!(active, None);
    }

    #[test]
    fn tab_cycles_to_next_cage_and_jumps_to_anchor() {
        let cages = cages_basic();
        let (cursor, active) = next_state((0, 0), Some(0), 3, &cages, NavKey::Tab);
        assert_eq!(active, Some(1));
        assert_eq!(cursor, (1, 0));
    }

    #[test]
    fn tab_wraps_around_to_first_cage() {
        let cages = cages_basic();
        let (cursor, active) = next_state((2, 2), Some(2), 3, &cages, NavKey::Tab);
        assert_eq!(active, Some(0));
        assert_eq!(cursor, (0, 0));
    }

    #[test]
    fn shift_tab_cycles_backwards() {
        let cages = cages_basic();
        let (cursor, active) = next_state((1, 0), Some(1), 3, &cages, NavKey::ShiftTab);
        assert_eq!(active, Some(0));
        assert_eq!(cursor, (0, 0));
    }

    #[test]
    fn shift_tab_wraps_to_last_cage() {
        let cages = cages_basic();
        let (cursor, active) = next_state((0, 0), Some(0), 3, &cages, NavKey::ShiftTab);
        assert_eq!(active, Some(2));
        assert_eq!(cursor, (2, 2));
    }

    #[test]
    fn tab_from_no_active_cage_starts_at_zero() {
        let cages = cages_basic();
        let (cursor, active) = next_state((1, 1), None, 3, &cages, NavKey::Tab);
        assert_eq!(active, Some(0));
        assert_eq!(cursor, (0, 0));
    }

    #[test]
    fn shift_tab_from_no_active_cage_starts_at_last() {
        let cages = cages_basic();
        let (cursor, active) = next_state((1, 1), None, 3, &cages, NavKey::ShiftTab);
        assert_eq!(active, Some(2));
        assert_eq!(cursor, (2, 2));
    }

    #[test]
    fn tab_with_no_cages_is_a_no_op() {
        let (cursor, active) = next_state((1, 1), None, 3, &[], NavKey::Tab);
        assert_eq!(cursor, (1, 1));
        assert_eq!(active, None);
    }

    #[test]
    fn shift_tab_with_no_cages_is_a_no_op() {
        let (cursor, active) = next_state((1, 1), Some(0), 3, &[], NavKey::ShiftTab);
        assert_eq!(cursor, (1, 1));
        assert_eq!(active, Some(0));
    }
}
