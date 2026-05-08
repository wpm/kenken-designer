use crate::app::{CageView, DraftCage, MoveState, OpKind, PuzzleView};
use crate::cage_colors::{assign_cage_colors, build_cell_cage_map};
use crate::cage_edit::effective_cages;
use crate::cage_index::{cage_anchor, cells_anchor};
use crate::operator_entry::{targets_for_op, ActiveCage, EntryMode, OperatorEntry};
use crate::theme::{ACCENT, BG, CAGE_PALETTE, INK, INK3, LINE, SERIF_FONT};
use leptos::prelude::*;

const MARGIN: f64 = 14.0;
const OUTER_STROKE: f64 = 2.6;
const THICK_STROKE: f64 = 2.2;
const THIN_STROKE: f64 = 0.5;
const OP_INSET: f64 = 4.0;
pub const UNCAGED_FILL: &str = "#fefcf7";
const ACTIVE_FILL_OPACITY: &str = "0.16";
const CURSOR_INSET: f64 = 1.5;
const CURSOR_STROKE: &str = "2.5";
const CURSOR_STROKE_ENTRY: &str = "3";
const MOVE_SOURCE_OPACITY: &str = "0.5";
const MOVE_TARGET_SELECTED_STROKE: &str = "2.0";
const MOVE_TARGET_STROKE: &str = "1.0";

#[derive(Clone, Copy)]
pub struct Layout {
    pub n: usize,
    pub cell: f64,
}

impl Layout {
    pub fn new(n: usize, size: u32) -> Self {
        let cell = MARGIN.mul_add(-2.0, f64::from(size)) / usize_to_f64(n).max(1.0);
        Self { n, cell }
    }

    fn cols(&self) -> usize {
        ceil_sqrt(self.n).max(1)
    }

    fn rows(&self) -> usize {
        self.n.div_ceil(self.cols()).max(1)
    }

    fn sub_w(&self) -> f64 {
        self.inner_extent() / usize_to_f64(self.cols())
    }

    fn sub_h(&self) -> f64 {
        self.inner_extent() / usize_to_f64(self.rows())
    }

    pub fn candidate_font(&self) -> f64 {
        (self.sub_w().min(self.sub_h()) * 0.38).max(6.0)
    }

    fn singleton_font(&self) -> f64 {
        self.cell * 0.5
    }

    fn op_font(&self) -> f64 {
        (self.cell * 0.16).max(10.0)
    }

    /// Buffer reserved on every inner edge of a cell so the top-left operator
    /// label has its own space and never overlaps candidate "fill" digits.
    pub fn digit_inset(&self) -> f64 {
        OP_INSET + self.op_font() + 2.0
    }

    /// Side length of the cell's inner area — the cell minus a `digit_inset`
    /// buffer on each edge — into which candidate digits are laid out.
    fn inner_extent(&self) -> f64 {
        2.0_f64.mul_add(-self.digit_inset(), self.cell).max(0.0)
    }

    /// Center of the full cell, used for the singleton (final-answer) digit.
    /// Spans the whole cell rather than the inner area so the answer is
    /// visually centered regardless of the operator buffer.
    const fn singleton_center(&self, cell_x: f64, cell_y: f64) -> (f64, f64) {
        (cell_x + self.cell / 2.0, cell_y + self.cell / 2.0)
    }

    pub const fn origin(&self, r: usize, c: usize) -> (f64, f64) {
        (
            usize_to_f64(c).mul_add(self.cell, MARGIN),
            usize_to_f64(r).mul_add(self.cell, MARGIN),
        )
    }

    pub fn sub_cell_center(&self, r: usize, c: usize, v: u8) -> (f64, f64) {
        let (cell_x, cell_y) = self.origin(r, c);
        let cols = self.cols();
        let sub_w = self.sub_w();
        let sub_h = self.sub_h();
        let inset = self.digit_inset();
        let idx = usize::from(v.saturating_sub(1));
        let sub_r = idx / cols;
        let sub_c = idx % cols;
        (
            usize_to_f64(sub_c).mul_add(sub_w, cell_x + inset) + sub_w / 2.0,
            usize_to_f64(sub_r).mul_add(sub_h, cell_y + inset) + sub_h / 2.0,
        )
    }
}

struct TextStyle {
    size: f64,
    fill: &'static str,
    opacity: &'static str,
    weight: &'static str,
}

impl TextStyle {
    fn singleton(layout: &Layout) -> Self {
        Self {
            size: layout.singleton_font(),
            fill: INK,
            opacity: "1.0",
            weight: "600",
        }
    }

    fn candidate(layout: &Layout) -> Self {
        Self {
            size: layout.candidate_font(),
            fill: INK3,
            opacity: "0.65",
            weight: "400",
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Row,
    Col,
}

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn Grid(
    view: PuzzleView,
    drafts: Vec<DraftCage>,
    size: u32,
    cursor: Signal<(usize, usize)>,
    active_cage: Signal<Option<usize>>,
    on_cell_click: Callback<(usize, usize)>,
    on_cell_right_click: Callback<(usize, usize, f64, f64)>,
    entry: Signal<Option<OperatorEntry>>,
    flash_diff: ReadSignal<crate::diff::PuzzleDiff>,
    move_mode: Signal<Option<MoveState>>,
) -> impl IntoView {
    let n = view.n;
    debug_assert!(n > 0, "Grid requires a puzzle with n > 0");

    let layout = Layout::new(n, size);
    let (effective, draft_idx) = effective_cages(&view, &drafts);
    let palette_idx = assign_cage_colors(n, &effective, CAGE_PALETTE.len());
    let cell_cage = build_cell_cage_map(n, &effective);

    let cells_view = render_cells(&layout, &cell_cage, &palette_idx);
    let texts_view = render_texts(&view, &layout, move_mode);
    let lines = render_gridlines(&layout, &cell_cage);
    let outer_size = layout.cell * usize_to_f64(n);
    let op_labels = render_op_labels(&effective, draft_idx, &layout, entry);
    let cages = view.cages.clone();
    let cages_for_dropdown = view.cages.clone();
    let drafts_for_dropdown = drafts;
    let view_for_move = view;

    let active_overlay = move || {
        active_cage
            .get()
            .and_then(|idx| cages.get(idx))
            .map(|cage| -> Vec<_> { active_cage_overlay_rects(cage, layout) })
    };

    let cursor_rect = move || cursor_rect_view(cursor.get(), layout, entry.get().is_some());

    let target_dropdown = move || {
        render_target_dropdown(
            &cages_for_dropdown,
            &drafts_for_dropdown,
            layout,
            entry.get(),
        )
    };

    let click_overlay = render_click_overlay(layout, on_cell_click, on_cell_right_click);

    // Move-mode overlays: source cell half opacity, target cages with dashed border.
    let move_overlay = move || {
        move_mode
            .get()
            .map(|state| render_move_overlays(&state, &view_for_move, layout))
    };

    view! {
        <svg
            class="grid-svg"
            viewBox=format!("0 0 {size} {size}")
            preserveAspectRatio="xMidYMid meet"
            xmlns="http://www.w3.org/2000/svg"
        >
            <rect x="0" y="0" width=size height=size fill=BG />
            {cells_view}
            {texts_view}
            {lines}
            <rect
                x=MARGIN
                y=MARGIN
                width=outer_size
                height=outer_size
                fill="none"
                stroke=INK
                stroke-width=OUTER_STROKE
            />
            {op_labels}
            {target_dropdown}
            {active_overlay}
            {move_overlay}
            {cursor_rect}
            {click_overlay}
            <crate::flash::FlashOverlay diff=flash_diff layout=layout />
        </svg>
    }
}

fn active_cage_overlay_rects(cage: &CageView, layout: Layout) -> Vec<impl IntoView> {
    let cell = layout.cell;
    cage.cells
        .iter()
        .map(|&(r, c)| {
            let (x, y) = layout.origin(r, c);
            view! {
                <rect
                    x=x
                    y=y
                    width=cell
                    height=cell
                    fill=ACCENT
                    fill-opacity=ACTIVE_FILL_OPACITY
                    pointer-events="none"
                />
            }
        })
        .collect()
}

fn cursor_rect_view(cursor: (usize, usize), layout: Layout, in_entry: bool) -> impl IntoView {
    let (x, y) = layout.origin(cursor.0, cursor.1);
    let side = 2.0_f64.mul_add(-CURSOR_INSET, layout.cell).max(0.0);
    let stroke_color = if in_entry { INK } else { ACCENT };
    let stroke_width = if in_entry {
        CURSOR_STROKE_ENTRY
    } else {
        CURSOR_STROKE
    };
    view! {
        <rect
            data-testid="cursor"
            x=x + CURSOR_INSET
            y=y + CURSOR_INSET
            width=side
            height=side
            fill="none"
            stroke=stroke_color
            stroke-width=stroke_width
            pointer-events="none"
        />
    }
}

fn render_click_overlay(
    layout: Layout,
    on_cell_click: Callback<(usize, usize)>,
    on_cell_right_click: Callback<(usize, usize, f64, f64)>,
) -> Vec<impl IntoView> {
    let n = layout.n;
    let cell = layout.cell;
    (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .map(|(r, c)| {
            let (x, y) = layout.origin(r, c);
            view! {
                <rect
                    x=x
                    y=y
                    width=cell
                    height=cell
                    fill="transparent"
                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                        if ev.button() == 0 {
                            on_cell_click.run((r, c));
                        }
                    }
                    on:contextmenu=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        on_cell_right_click.run((r, c, ev.client_x().into(), ev.client_y().into()));
                    }
                />
            }
        })
        .collect()
}

fn render_cells(
    layout: &Layout,
    cell_cage: &[Vec<Option<usize>>],
    palette_idx: &[usize],
) -> Vec<impl IntoView> {
    let n = layout.n;
    let cell = layout.cell;
    (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .map(|(r, c)| {
            let fill = cell_cage[r][c].map_or(UNCAGED_FILL, |i| {
                CAGE_PALETTE[palette_idx[i] % CAGE_PALETTE.len()]
            });
            let (x, y) = layout.origin(r, c);
            view! { <rect x=x y=y width=cell height=cell fill=fill /> }
        })
        .collect()
}

fn render_texts(
    view: &PuzzleView,
    layout: &Layout,
    move_mode: Signal<Option<MoveState>>,
) -> impl IntoView {
    let grid: Vec<Vec<Vec<u8>>> = view.cells.clone();
    let layout = *layout;

    move || {
        let source_cell = move_mode.get().map(|s| s.cell);
        let mut out = Vec::new();
        for (r, row) in grid.iter().enumerate() {
            for (c, candidates) in row.iter().enumerate() {
                // In move mode, skip candidates for the source cell
                if source_cell == Some((r, c)) {
                    continue;
                }
                let (cell_x, cell_y) = layout.origin(r, c);
                let singleton = candidates.len() == 1;
                let style = if singleton {
                    TextStyle::singleton(&layout)
                } else {
                    TextStyle::candidate(&layout)
                };
                for &v in candidates {
                    let (cx, cy) = if singleton {
                        layout.singleton_center(cell_x, cell_y)
                    } else {
                        layout.sub_cell_center(r, c, v)
                    };
                    out.push(view! {
                        <text
                            x=cx
                            y=cy
                            text-anchor="middle"
                            dominant-baseline="central"
                            font-family=SERIF_FONT
                            font-size=style.size
                            fill=style.fill
                            opacity=style.opacity
                            font-weight=style.weight
                        >
                            {v.to_string()}
                        </text>
                    });
                }
            }
        }
        out
    }
}

fn render_gridlines(layout: &Layout, cell_cage: &[Vec<Option<usize>>]) -> Vec<impl IntoView> {
    let mut lines = Vec::new();
    for axis in [Axis::Row, Axis::Col] {
        for (major, minor, a, b) in border_pairs(axis, cell_cage) {
            let (stroke, width) = stroke_for(is_thick_border(a, b));
            let (x1, y1, x2, y2) = endpoint_for(axis, layout, major, minor);
            lines.push(view! {
                <line x1=x1 y1=y1 x2=x2 y2=y2 stroke=stroke stroke-width=width stroke-linecap="round" />
            });
        }
    }
    lines
}

fn border_pairs(
    axis: Axis,
    cell_cage: &[Vec<Option<usize>>],
) -> Vec<(usize, usize, Option<usize>, Option<usize>)> {
    let n = cell_cage.len();
    let mut pairs = Vec::new();
    match axis {
        Axis::Row => {
            for (r, pair) in cell_cage.windows(2).enumerate() {
                for (c, (&top, &bot)) in pair[0].iter().zip(pair[1].iter()).enumerate() {
                    pairs.push((r, c, top, bot));
                }
            }
        }
        Axis::Col => {
            for c in 0..n.saturating_sub(1) {
                for (r, row) in cell_cage.iter().enumerate() {
                    pairs.push((c, r, row[c], row[c + 1]));
                }
            }
        }
    }
    pairs
}

fn endpoint_for(axis: Axis, layout: &Layout, major: usize, minor: usize) -> (f64, f64, f64, f64) {
    let cell = layout.cell;
    let cross = usize_to_f64(major + 1).mul_add(cell, MARGIN);
    let along_start = usize_to_f64(minor).mul_add(cell, MARGIN);
    let along_end = along_start + cell;
    match axis {
        Axis::Row => (along_start, cross, along_end, cross),
        Axis::Col => (cross, along_start, cross, along_end),
    }
}

const fn stroke_for(thick: bool) -> (&'static str, f64) {
    if thick {
        (INK, THICK_STROKE)
    } else {
        (LINE, THIN_STROKE)
    }
}

fn render_op_labels(
    cages: &[CageView],
    draft_idx: Option<usize>,
    layout: &Layout,
    entry: Signal<Option<OperatorEntry>>,
) -> Vec<impl IntoView> {
    let op_font = layout.op_font();
    cages
        .iter()
        .enumerate()
        .map(|(i, cage)| {
            let (r, c) = cage_anchor(cage);
            let (cell_x, cell_y) = layout.origin(r, c);
            let label_x = cell_x + OP_INSET;
            let label_y = cell_y + OP_INSET;
            let is_draft = draft_idx.is_some_and(|di| i >= di);
            let is_active_draft = draft_idx.is_some_and(|di| i == di);
            let cage_op = cage.op;
            let cage_target = cage.target;
            let cell_count = cage.cells.len();
            let label = move || {
                let entry_val = entry.get();
                let is_entry_cage = entry_val.as_ref().is_some_and(|e| match &e.cage {
                    ActiveCage::Committed(idx) => *idx == i,
                    ActiveCage::Draft => is_active_draft,
                });
                entry_val.filter(|_| is_entry_cage).map_or_else(
                    || {
                        if is_draft {
                            valid_op_glyphs(cell_count)
                        } else {
                            op_label(cage_op, cage_target)
                        }
                    },
                    |e| entry_label_text(&e),
                )
            };
            view! {
                <text
                    x=label_x
                    y=label_y
                    text-anchor="start"
                    dominant-baseline="hanging"
                    font-family=SERIF_FONT
                    font-size=op_font
                    font-weight="700"
                    fill=INK
                >
                    {label}
                </text>
            }
        })
        .collect()
}

/// Inline anchor label text for a cage that's currently the active entry.
///
/// `OpPicker`: valid operator glyphs (e.g. "+ − × ÷"). `TargetPicker`: the typed
/// digit buffer (or the selected target as a preview) followed by the operator
/// glyph and an entry caret.
fn entry_label_text(e: &OperatorEntry) -> String {
    match &e.mode {
        EntryMode::OpPicker => valid_op_glyphs_from_options(e),
        EntryMode::TargetPicker {
            op,
            selected,
            digits,
        } => {
            let glyph = op_glyph(*op);
            if !digits.is_empty() {
                return format!("{digits}{glyph}|");
            }
            let target = targets_for_op(&e.options, *op).get(*selected).copied();
            target.map_or_else(
                || format!("{glyph}|"),
                |t| {
                    let label = op_label(*op, t);
                    format!("{label}|")
                },
            )
        }
    }
}

/// Glyphs for valid operators on a cage of `cell_count` cells, joined by spaces.
/// Singletons fall back to `?` since their only valid op (Given) has no glyph.
fn valid_op_glyphs(cell_count: usize) -> String {
    let ops: &[OpKind] = match cell_count {
        2 => &[OpKind::Add, OpKind::Sub, OpKind::Mul, OpKind::Div],
        n if n >= 3 => &[OpKind::Add, OpKind::Mul],
        _ => return "?".to_string(),
    };
    join_op_glyphs(ops.iter().copied())
}

fn valid_op_glyphs_from_options(e: &OperatorEntry) -> String {
    let glyphs = join_op_glyphs(
        e.options
            .iter()
            .map(|o| o.op)
            .filter(|op| *op != OpKind::Given),
    );
    if glyphs.is_empty() {
        "?".to_string()
    } else {
        glyphs
    }
}

fn join_op_glyphs(ops: impl Iterator<Item = OpKind>) -> String {
    ops.map(op_glyph).collect::<Vec<_>>().join(" ")
}

const DROPDOWN_PAD: f64 = 4.0;
const DROPDOWN_ROW_GAP: f64 = 2.0;

/// Renders a small dropdown panel at the active cage's anchor showing the valid
/// targets for the chosen operator (step 2 of the operator-entry flow). Returns
/// `None` when there's no active `TargetPicker` entry.
fn render_target_dropdown(
    cages: &[CageView],
    drafts: &[DraftCage],
    layout: Layout,
    entry: Option<OperatorEntry>,
) -> Option<AnyView> {
    let entry = entry?;
    let EntryMode::TargetPicker { op, selected, .. } = entry.mode.clone() else {
        return None;
    };
    let anchor = match &entry.cage {
        ActiveCage::Committed(idx) => cages.get(*idx).map(cage_anchor)?,
        ActiveCage::Draft => drafts.first().map(|d| cells_anchor(&d.cells))?,
    };
    let targets: Vec<u32> = targets_for_op(&entry.options, op).to_vec();
    if targets.is_empty() {
        return None;
    }
    let font = layout.op_font();
    let row_height = font + DROPDOWN_ROW_GAP;
    let labels: Vec<String> = targets.iter().map(|&t| op_label(op, t)).collect();

    let (cell_x, cell_y) = layout.origin(anchor.0, anchor.1);
    // Position below the anchor label, just outside the operator buffer area so the
    // dropdown doesn't visually merge with the inline label.
    let panel_x = cell_x + OP_INSET;
    let panel_y = cell_y + OP_INSET + font + DROPDOWN_PAD;

    // Background rect width: longest label times an approximate em-width.
    let est_char_w = font * 0.62;
    let max_chars = labels.iter().map(|s| s.chars().count()).max().unwrap_or(1);
    let panel_w = est_char_w.mul_add(usize_to_f64(max_chars), 2.0 * DROPDOWN_PAD);
    let panel_h = row_height.mul_add(usize_to_f64(targets.len()), DROPDOWN_PAD);

    let rows: Vec<_> = labels
        .into_iter()
        .enumerate()
        .map(|(i, label)| {
            let y = row_height.mul_add(usize_to_f64(i), panel_y + DROPDOWN_PAD);
            let is_selected = i == selected;
            let weight = if is_selected { "700" } else { "400" };
            let fill = if is_selected { ACCENT } else { INK };
            view! {
                <text
                    x=panel_x + DROPDOWN_PAD
                    y=y
                    text-anchor="start"
                    dominant-baseline="hanging"
                    font-family=SERIF_FONT
                    font-size=font
                    font-weight=weight
                    fill=fill
                >
                    {label}
                </text>
            }
        })
        .collect();

    Some(
        view! {
            <g pointer-events="none">
                <rect
                    x=panel_x
                    y=panel_y
                    width=panel_w
                    height=panel_h
                    fill=BG
                    stroke=LINE
                    stroke-width=THIN_STROKE
                    rx="2"
                    ry="2"
                />
                {rows}
            </g>
        }
        .into_any(),
    )
}

/// Render overlays for move mode:
/// - Source cell: white overlay at half opacity (dims the cell)
/// - Currently-selected target cage cells: dashed accent border
/// - Other legal target cage cells: thin accent border
fn render_move_overlays(state: &MoveState, view: &PuzzleView, layout: Layout) -> Vec<AnyView> {
    let cell = layout.cell;
    let mut out: Vec<AnyView> = Vec::new();

    // Source cell overlay: half opacity white rectangle
    let (sx, sy) = layout.origin(state.cell.0, state.cell.1);
    out.push(
        view! {
            <rect
                x=sx
                y=sy
                width=cell
                height=cell
                fill="white"
                fill-opacity=MOVE_SOURCE_OPACITY
                pointer-events="none"
            />
        }
        .into_any(),
    );

    // Target cage overlays
    for (i, &target_anchor) in state.targets.iter().enumerate() {
        let is_selected = state.selected == Some(i);
        let stroke_width = if is_selected {
            MOVE_TARGET_SELECTED_STROKE
        } else {
            MOVE_TARGET_STROKE
        };
        let dash = if is_selected { "4,3" } else { "" };

        // Find target cage cells
        if let Some(cage) = view
            .cages
            .iter()
            .find(|cage| cage_anchor(cage) == target_anchor)
        {
            for &(r, c) in &cage.cells {
                let (x, y) = layout.origin(r, c);
                out.push(
                    view! {
                        <rect
                            x=x
                            y=y
                            width=cell
                            height=cell
                            fill="none"
                            stroke=ACCENT
                            stroke-width=stroke_width
                            stroke-dasharray=dash
                            pointer-events="none"
                        />
                    }
                    .into_any(),
                );
            }
        }
    }

    out
}

const fn is_thick_border(a: Option<usize>, b: Option<usize>) -> bool {
    matches!((a, b), (Some(x), Some(y)) if x != y)
}

const fn op_glyph(op: OpKind) -> &'static str {
    match op {
        OpKind::Add => "+",
        OpKind::Sub => "\u{2212}",
        OpKind::Mul => "\u{00d7}",
        OpKind::Div => "\u{00f7}",
        OpKind::Given => "",
    }
}

pub fn op_label(op: OpKind, target: u32) -> String {
    match op {
        OpKind::Add => format!("{target}+"),
        OpKind::Sub => format!("{target}\u{2212}"),
        OpKind::Mul => format!("{target}\u{00d7}"),
        OpKind::Div => format!("{target}\u{00f7}"),
        OpKind::Given => format!("{target}"),
    }
}

pub const fn ceil_sqrt(n: usize) -> usize {
    if n <= 1 {
        return n;
    }
    let mut x: usize = 1;
    while x.saturating_mul(x) < n {
        x += 1;
    }
    x
}

#[allow(clippy::cast_precision_loss)]
pub const fn usize_to_f64(x: usize) -> f64 {
    x as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpKind;

    #[test]
    fn op_label_add_is_number_then_plus() {
        assert_eq!(op_label(OpKind::Add, 15), "15+");
    }

    #[test]
    fn op_label_sub_is_number_then_minus() {
        assert_eq!(op_label(OpKind::Sub, 3), "3\u{2212}");
    }

    #[test]
    fn op_label_mul_is_number_then_times() {
        assert_eq!(op_label(OpKind::Mul, 24), "24\u{00d7}");
    }

    #[test]
    fn op_label_div_is_number_then_div() {
        assert_eq!(op_label(OpKind::Div, 2), "2\u{00f7}");
    }

    #[test]
    fn op_label_given_is_just_number() {
        assert_eq!(op_label(OpKind::Given, 7), "7");
    }

    use crate::app::GRID_SIZE;

    #[test]
    fn digit_inset_clears_operator_font() {
        for n in [4_usize, 6, 9] {
            let layout = Layout::new(n, GRID_SIZE);
            assert!(
                layout.digit_inset() >= OP_INSET + layout.op_font(),
                "n={n}: digit_inset={} should clear OP_INSET + op_font={}",
                layout.digit_inset(),
                OP_INSET + layout.op_font(),
            );
        }
    }

    #[test]
    fn top_left_candidate_sits_below_operator_buffer() {
        for n in [4_usize, 6, 9] {
            let layout = Layout::new(n, GRID_SIZE);
            let (_cell_x, cell_y) = layout.origin(0, 0);
            let inset = layout.digit_inset();
            let sub_h = layout.sub_h();
            let cy = cell_y + inset + sub_h / 2.0;
            let half_font = layout.candidate_font() / 2.0;
            assert!(
                cy - half_font >= cell_y + inset - 0.001,
                "n={n}: top-left candidate top={} must be at or below buffer bottom={}",
                cy - half_font,
                cell_y + inset,
            );
            assert!(
                cell_y + inset >= cell_y + OP_INSET + layout.op_font(),
                "n={n}: buffer must clear the operator label",
            );
        }
    }

    #[test]
    fn singleton_sits_at_geometric_center_of_cell() {
        // Pins the invariant that the singleton (final-answer) digit is
        // placed at the cell's geometric center — equidistant from all four
        // cell edges — and is therefore unaffected by `digit_inset`.
        let layout = Layout::new(6, GRID_SIZE);
        let (cell_x, cell_y) = layout.origin(2, 3);
        let (cx, cy) = layout.singleton_center(cell_x, cell_y);
        let left = cx - cell_x;
        let right = (cell_x + layout.cell) - cx;
        let top = cy - cell_y;
        let bottom = (cell_y + layout.cell) - cy;
        assert!(
            (left - right).abs() < 1e-9,
            "singleton not horizontally centered: left={left}, right={right}",
        );
        assert!(
            (top - bottom).abs() < 1e-9,
            "singleton not vertically centered: top={top}, bottom={bottom}",
        );
        assert!(
            left > layout.digit_inset(),
            "singleton's gap to cell edge ({left}) should exceed digit_inset \
             ({}) — i.e. singleton spans the full cell, not just the inner area",
            layout.digit_inset(),
        );
    }

    #[test]
    fn sub_dimensions_use_inner_area() {
        let layout = Layout::new(6, GRID_SIZE);
        let inner = 2.0_f64.mul_add(-layout.digit_inset(), layout.cell).max(0.0);
        let expected_sub_w = inner / usize_to_f64(layout.cols());
        let expected_sub_h = inner / usize_to_f64(layout.rows());
        assert!((layout.sub_w() - expected_sub_w).abs() < 1e-9);
        assert!((layout.sub_h() - expected_sub_h).abs() < 1e-9);
    }
}
