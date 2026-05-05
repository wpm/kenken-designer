use crate::app::{CageView, DraftCage, OpKind, PuzzleView};
use crate::cage_colors::{assign_cage_colors, build_cell_cage_map};
use crate::cage_edit::effective_cages;
use crate::cage_index::cage_anchor;
use crate::operator_entry::{ActiveCage, OperatorEntry};
use crate::theme::{ACCENT, BG, CAGE_PALETTE, INK, INK3, LINE, SERIF_FONT};
use leptos::prelude::*;

const MARGIN: f64 = 14.0;
const OUTER_STROKE: f64 = 2.6;
const THICK_STROKE: f64 = 2.2;
const THIN_STROKE: f64 = 0.5;
const OP_HALO_STROKE: f64 = 2.5;
const OP_INSET: f64 = 4.0;
const UNCAGED_FILL: &str = "#fefcf7";
const ACTIVE_FILL_OPACITY: &str = "0.16";
const CURSOR_INSET: f64 = 1.5;
const CURSOR_STROKE: &str = "2.5";
const CURSOR_STROKE_ENTRY: &str = "3";

#[derive(Clone, Copy)]
struct Layout {
    n: usize,
    cell: f64,
}

impl Layout {
    fn new(n: usize, size: u32) -> Self {
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
        self.cell / usize_to_f64(self.cols())
    }

    fn sub_h(&self) -> f64 {
        self.cell / usize_to_f64(self.rows())
    }

    fn candidate_font(&self) -> f64 {
        (self.sub_w().min(self.sub_h()) * 0.38).max(6.0)
    }

    fn singleton_font(&self) -> f64 {
        self.cell * 0.5
    }

    fn op_font(&self) -> f64 {
        (self.cell * 0.16).max(10.0)
    }

    const fn origin(&self, r: usize, c: usize) -> (f64, f64) {
        (
            usize_to_f64(c).mul_add(self.cell, MARGIN),
            usize_to_f64(r).mul_add(self.cell, MARGIN),
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
    draft: Option<DraftCage>,
    size: u32,
    cursor: Signal<(usize, usize)>,
    active_cage: Signal<Option<usize>>,
    on_cell_click: Callback<(usize, usize)>,
    entry: Signal<Option<OperatorEntry>>,
) -> impl IntoView {
    let n = view.n;
    debug_assert!(n > 0, "Grid requires a puzzle with n > 0");

    let layout = Layout::new(n, size);
    let (effective, draft_idx) = effective_cages(&view, draft.as_ref());
    let palette_idx = assign_cage_colors(n, &effective, CAGE_PALETTE.len());
    let cell_cage = build_cell_cage_map(n, &effective);

    let cells_view = render_cells(&layout, &cell_cage, &palette_idx);
    let texts_view = render_texts(&view, &layout);
    let lines = render_gridlines(&layout, &cell_cage);
    let outer_size = layout.cell * usize_to_f64(n);
    let op_labels = render_op_labels(&effective, draft_idx, &layout, entry);
    let cages = view.cages;

    let active_overlay = move || {
        active_cage
            .get()
            .and_then(|idx| cages.get(idx))
            .map(|cage| -> Vec<_> { active_cage_overlay_rects(cage, layout) })
    };

    let cursor_rect = move || cursor_rect_view(cursor.get(), layout, entry.get().is_some());

    let click_overlay = render_click_overlay(layout, on_cell_click);

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
            {active_overlay}
            {cursor_rect}
            {click_overlay}
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

fn render_texts(view: &PuzzleView, layout: &Layout) -> Vec<impl IntoView> {
    let n = layout.n;
    let cols = layout.cols();
    let sub_w = layout.sub_w();
    let sub_h = layout.sub_h();
    let mut out = Vec::new();
    for r in 0..n {
        for c in 0..n {
            let cell_data = &view.cells[r][c];
            let (cell_x, cell_y) = layout.origin(r, c);
            let singleton = cell_data.len() == 1;
            let style = if singleton {
                TextStyle::singleton(layout)
            } else {
                TextStyle::candidate(layout)
            };
            for &v in cell_data {
                let (cx, cy) = if singleton {
                    (cell_x + layout.cell / 2.0, cell_y + layout.cell / 2.0)
                } else {
                    let idx = usize::from(v.saturating_sub(1));
                    let sub_r = idx / cols;
                    let sub_c = idx % cols;
                    (
                        usize_to_f64(sub_c).mul_add(sub_w, cell_x) + sub_w / 2.0,
                        usize_to_f64(sub_r).mul_add(sub_h, cell_y) + sub_h / 2.0,
                    )
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
            let is_draft = Some(i) == draft_idx;
            let cage_op = cage.op;
            let cage_target = cage.target;
            let label = move || {
                let entry_val = entry.get();
                let is_entry_cage = entry_val.as_ref().is_some_and(|e| match &e.cage {
                    ActiveCage::Committed(idx) => *idx == i,
                    ActiveCage::Draft => is_draft,
                });
                if let Some(e) = entry_val.filter(|_| is_entry_cage) {
                    let glyph = e.op.map_or("", op_glyph);
                    format!("{}{}|", glyph, e.digits)
                } else if is_draft {
                    "?".to_string()
                } else {
                    op_label(cage_op, cage_target)
                }
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
                    stroke="white"
                    stroke-width=OP_HALO_STROKE
                    stroke-linejoin="round"
                    paint-order="stroke"
                >
                    {label}
                </text>
            }
        })
        .collect()
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

fn op_label(op: OpKind, target: u32) -> String {
    match op {
        OpKind::Add => format!("+{target}"),
        OpKind::Sub => format!("{target}\u{2212}"),
        OpKind::Mul => format!("{target}\u{00d7}"),
        OpKind::Div => format!("{target}\u{00f7}"),
        OpKind::Given => format!("{target}"),
    }
}

const fn ceil_sqrt(n: usize) -> usize {
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
const fn usize_to_f64(x: usize) -> f64 {
    x as f64
}
