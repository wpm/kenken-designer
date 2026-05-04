use crate::app::CageView;
use std::collections::BTreeSet;

#[must_use]
pub fn build_cell_cage_map(n: usize, cages: &[CageView]) -> Vec<Vec<Option<usize>>> {
    let mut map: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n];
    for (i, cage) in cages.iter().enumerate() {
        for &(r, c) in &cage.cells {
            if r < n && c < n {
                map[r][c] = Some(i);
            }
        }
    }
    map
}

#[must_use]
pub fn assign_cage_colors(n: usize, cages: &[CageView], palette_size: usize) -> Vec<usize> {
    let cage_count = cages.len();
    if cage_count == 0 {
        return Vec::new();
    }
    let modulus = palette_size.max(1);

    let cell_to_cage = build_cell_cage_map(n, cages);

    let mut adjacency: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); cage_count];
    for (i, cage) in cages.iter().enumerate() {
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
        let mut used: u64 = 0;
        for &neighbor in &adjacency[i] {
            if let Some(c) = colors[neighbor] {
                if c < u64::BITS as usize {
                    used |= 1_u64 << c;
                }
            }
        }
        let mut color = 0_usize;
        while color < u64::BITS as usize && used & (1_u64 << color) != 0 {
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
    use crate::app::OpKind;
    use std::collections::BTreeSet;

    fn cage(cells: &[(usize, usize)]) -> CageView {
        CageView {
            cells: cells.to_vec(),
            op: OpKind::Given,
            target: 0,
        }
    }

    #[test]
    fn single_cage_gets_color_zero() {
        let cages = vec![cage(&[(0, 0), (0, 1), (1, 0), (1, 1)])];
        assert_eq!(assign_cage_colors(2, &cages, 8), vec![0]);
    }

    #[test]
    fn non_adjacent_cages_share_color_zero() {
        let cages = vec![cage(&[(0, 0)]), cage(&[(2, 2)])];
        assert_eq!(assign_cage_colors(3, &cages, 8), vec![0, 0]);
    }

    #[test]
    fn adjacent_cages_get_distinct_colors() {
        let cages = vec![cage(&[(0, 0)]), cage(&[(0, 1)])];
        let colors = assign_cage_colors(2, &cages, 8);
        assert_eq!(colors.len(), 2);
        assert_ne!(colors[0], colors[1]);
    }

    #[test]
    fn chain_of_five_uses_at_most_two_colors() {
        let cages = vec![
            cage(&[(0, 0)]),
            cage(&[(0, 1)]),
            cage(&[(0, 2)]),
            cage(&[(0, 3)]),
            cage(&[(0, 4)]),
        ];
        let colors = assign_cage_colors(5, &cages, 8);
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
        let cages: Vec<CageView> = vec![];
        assert!(assign_cage_colors(4, &cages, 8).is_empty());
    }
}
