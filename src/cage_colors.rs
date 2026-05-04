use crate::app::PuzzleView;
use std::collections::BTreeSet;

#[must_use]
pub fn build_cell_cage_map(view: &PuzzleView) -> Vec<Vec<Option<usize>>> {
    let n = view.n;
    let mut map: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n];
    for (i, cage) in view.cages.iter().enumerate() {
        for &(r, c) in &cage.cells {
            if r < n && c < n {
                map[r][c] = Some(i);
            }
        }
    }
    map
}

#[must_use]
pub fn assign_cage_colors(view: &PuzzleView, palette_size: usize) -> Vec<usize> {
    let cage_count = view.cages.len();
    if cage_count == 0 {
        return Vec::new();
    }
    let n = view.n;
    let modulus = palette_size.max(1);

    let cell_to_cage = build_cell_cage_map(view);

    let mut adjacency: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); cage_count];
    for (i, cage) in view.cages.iter().enumerate() {
        for &(r, c) in &cage.cells {
            for (nr, nc) in cell_neighbors(r, c, n) {
                if let Some(other) = cell_to_cage[nr][nc] {
                    if other != i {
                        adjacency[i].insert(other);
                    }
                }
            }
        }
    }

    let mut order: Vec<usize> = (0..cage_count).collect();
    order.sort_by(|a, b| {
        adjacency[*b]
            .len()
            .cmp(&adjacency[*a].len())
            .then_with(|| a.cmp(b))
    });

    let mut colors: Vec<Option<usize>> = vec![None; cage_count];
    for &i in &order {
        let mut used = BTreeSet::new();
        for &neighbor in &adjacency[i] {
            if let Some(c) = colors[neighbor] {
                used.insert(c);
            }
        }
        let mut color = 0_usize;
        while used.contains(&color) {
            color += 1;
        }
        colors[i] = Some(color % modulus);
    }

    colors.into_iter().map(Option::unwrap_or_default).collect()
}

fn cell_neighbors(r: usize, c: usize, n: usize) -> impl Iterator<Item = (usize, usize)> {
    let mut out: [(usize, usize); 4] = [(0, 0); 4];
    let mut count = 0;
    if r > 0 {
        out[count] = (r - 1, c);
        count += 1;
    }
    if r + 1 < n {
        out[count] = (r + 1, c);
        count += 1;
    }
    if c > 0 {
        out[count] = (r, c - 1);
        count += 1;
    }
    if c + 1 < n {
        out[count] = (r, c + 1);
        count += 1;
    }
    out.into_iter().take(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{CageView, OpKind};
    use std::collections::BTreeSet;

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Given,
            target: 0,
        }
    }

    fn view_with(n: usize, cages: Vec<CageView>) -> PuzzleView {
        PuzzleView {
            n,
            cells: vec![vec![Vec::new(); n]; n],
            cages,
        }
    }

    #[test]
    fn single_cage_gets_color_zero() {
        let view = view_with(2, vec![cage(&[(0, 0), (0, 1), (1, 0), (1, 1)])]);
        assert_eq!(assign_cage_colors(&view, 8), vec![0]);
    }

    #[test]
    fn non_adjacent_cages_share_color_zero() {
        let view = view_with(3, vec![cage(&[(0, 0)]), cage(&[(2, 2)])]);
        assert_eq!(assign_cage_colors(&view, 8), vec![0, 0]);
    }

    #[test]
    fn adjacent_cages_get_distinct_colors() {
        let view = view_with(2, vec![cage(&[(0, 0)]), cage(&[(0, 1)])]);
        let colors = assign_cage_colors(&view, 8);
        assert_eq!(colors.len(), 2);
        assert_ne!(colors[0], colors[1]);
    }

    #[test]
    fn chain_of_five_uses_at_most_two_colors() {
        let view = view_with(
            5,
            vec![
                cage(&[(0, 0)]),
                cage(&[(0, 1)]),
                cage(&[(0, 2)]),
                cage(&[(0, 3)]),
                cage(&[(0, 4)]),
            ],
        );
        let colors = assign_cage_colors(&view, 8);
        let unique: BTreeSet<_> = colors.iter().copied().collect();
        assert!(
            unique.len() <= 2,
            "chain used {} colors: {:?}",
            unique.len(),
            colors
        );
        for w in colors.windows(2) {
            assert_ne!(w[0], w[1]);
        }
    }

    #[test]
    fn no_cages_returns_empty() {
        let view = view_with(4, vec![]);
        assert!(assign_cage_colors(&view, 8).is_empty());
    }
}
