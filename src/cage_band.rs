use crate::app::{PuzzleView, GRID_SIZE};
use crate::cage_colors::{assign_cage_colors, build_cell_cage_map};
use crate::cage_index::cage_anchor;
use crate::grid::{ceil_sqrt, op_label, usize_to_f64, UNCAGED_FILL};
use crate::theme::{ACCENT, BG, CAGE_PALETTE, INK, INK3, LINE, SERIF_FONT};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// ─── Data types returned by the backend ─────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RankedTuple {
    pub tuple: Vec<u32>,
    pub view: PuzzleView,
    pub total_reduction: usize,
    pub newly_singleton: usize,
}

// ─── Tauri call helpers ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct RankArgs {
    anchor: (usize, usize),
}

#[derive(Serialize)]
struct ApplyNarrowingArgs {
    anchor: (usize, usize),
    tuple: Vec<u32>,
}

#[allow(clippy::future_not_send)] // WASM single-threaded runtime; Send is meaningless here
async fn call_rank_active_cage(anchor: (usize, usize)) -> Option<Vec<RankedTuple>> {
    let args = serde_wasm_bindgen::to_value(&RankArgs { anchor }).ok()?;
    let result = invoke("rank_active_cage", args).await;
    serde_wasm_bindgen::from_value(result).ok()
}

#[allow(clippy::future_not_send)] // WASM single-threaded runtime; Send is meaningless here
async fn call_apply_narrowing(anchor: (usize, usize), tuple: Vec<u32>) -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&ApplyNarrowingArgs { anchor, tuple }).ok()?;
    let result = invoke("apply_narrowing", args).await;
    serde_wasm_bindgen::from_value(result).ok()
}

// ─── Layout constants ────────────────────────────────────────────────────────

/// Side length of each thumbnail (square).
const THUMB_SIZE: u32 = 112;
/// Gap between thumbnails.
const THUMB_GAP: u32 = 8;
/// Height of the up/down advance buttons.
const BTN_HEIGHT: u32 = 28;

/// Total height each thumbnail occupies including gap.
const THUMB_STEP: u32 = THUMB_SIZE + THUMB_GAP;

/// How many thumbnails fit stacked in the band.
/// Derived from `GRID_SIZE` minus 2×8px padding, 2×4px gap, `2×BTN_HEIGHT`.
#[allow(clippy::cast_possible_truncation)] // GRID_SIZE and THUMB_STEP are small; quotient fits in usize
const VISIBLE_COUNT: usize = ((GRID_SIZE - 2 * BTN_HEIGHT) / THUMB_STEP) as usize;

// ─── Thumbnail SVG ───────────────────────────────────────────────────────────

const THUMB_MARGIN: f64 = 6.0;
const THUMB_OUTER_STROKE: f64 = 1.5;
const THUMB_THICK_STROKE: f64 = 1.2;
const THUMB_THIN_STROKE: f64 = 0.3;
const THUMB_ACTIVE_OPACITY: &str = "0.22";

/// Draw a single thumbnail SVG for one `RankedTuple`, highlighting `active_cells`.
#[component]
fn Thumbnail(
    rt: RankedTuple,
    active_cells: Vec<(usize, usize)>,
    selected: Signal<bool>,
    on_click: Callback<()>,
) -> impl IntoView {
    let n = rt.view.n;
    if n == 0 {
        return view! { <svg /> }.into_any();
    }
    let size = THUMB_SIZE;
    let cell_px = THUMB_MARGIN.mul_add(-2.0, f64::from(size)) / usize_to_f64(n).max(1.0);

    let cell_cage_map = build_cell_cage_map(n, &rt.view.cages);
    let palette_idx = assign_cage_colors(n, &rt.view.cages, CAGE_PALETTE.len());

    let cell_fills: Vec<_> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .map(|(r, c)| {
            let fill = cell_cage_map[r][c].map_or(UNCAGED_FILL, |i| {
                CAGE_PALETTE[palette_idx[i] % CAGE_PALETTE.len()]
            });
            let x = usize_to_f64(c).mul_add(cell_px, THUMB_MARGIN);
            let y = usize_to_f64(r).mul_add(cell_px, THUMB_MARGIN);
            view! { <rect x=x y=y width=cell_px height=cell_px fill=fill /> }
        })
        .collect();

    let highlights: Vec<_> = active_cells
        .iter()
        .map(|&(r, c)| {
            let x = usize_to_f64(c).mul_add(cell_px, THUMB_MARGIN);
            let y = usize_to_f64(r).mul_add(cell_px, THUMB_MARGIN);
            view! {
                <rect
                    x=x y=y
                    width=cell_px height=cell_px
                    fill=ACCENT
                    fill-opacity=THUMB_ACTIVE_OPACITY
                    pointer-events="none"
                />
            }
        })
        .collect();

    let cols_per_cell = ceil_sqrt(n).max(1);
    let rows_per_cell = n.div_ceil(cols_per_cell).max(1);
    let sub_w = cell_px / usize_to_f64(cols_per_cell);
    let sub_h = cell_px / usize_to_f64(rows_per_cell);
    let cand_font = (sub_w.min(sub_h) * 0.38).max(4.0);
    let singleton_font = cell_px * 0.5;

    let texts: Vec<_> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .flat_map(|(r, c)| {
            let cell_data = rt.view.cells[r][c].clone();
            let cx0 = usize_to_f64(c).mul_add(cell_px, THUMB_MARGIN);
            let cy0 = usize_to_f64(r).mul_add(cell_px, THUMB_MARGIN);
            let singleton = cell_data.len() == 1;
            cell_data
                .into_iter()
                .map(move |v| {
                    let (tx, ty, fs, fill, opacity, weight) = if singleton {
                        (
                            cx0 + cell_px / 2.0,
                            cy0 + cell_px / 2.0,
                            singleton_font,
                            INK,
                            "1.0",
                            "600",
                        )
                    } else {
                        let idx = usize::from(v).saturating_sub(1);
                        let sr = idx / cols_per_cell;
                        let sc = idx % cols_per_cell;
                        (
                            usize_to_f64(sc).mul_add(sub_w, cx0) + sub_w / 2.0,
                            usize_to_f64(sr).mul_add(sub_h, cy0) + sub_h / 2.0,
                            cand_font,
                            INK3,
                            "0.65",
                            "400",
                        )
                    };
                    view! {
                        <text
                            x=tx y=ty
                            text-anchor="middle"
                            dominant-baseline="central"
                            font-family=SERIF_FONT
                            font-size=fs
                            fill=fill
                            opacity=opacity
                            font-weight=weight
                        >
                            {v.to_string()}
                        </text>
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let mut lines: Vec<_> = Vec::new();
    for r in 1..n {
        let y = usize_to_f64(r).mul_add(cell_px, THUMB_MARGIN);
        for (c, (above_row, below_row)) in cell_cage_map[r - 1]
            .iter()
            .zip(cell_cage_map[r].iter())
            .enumerate()
        {
            let thick = matches!((*above_row, *below_row), (Some(a), Some(b)) if a != b);
            let (stroke, sw) = if thick {
                (INK, THUMB_THICK_STROKE)
            } else {
                (LINE, THUMB_THIN_STROKE)
            };
            let cx0 = usize_to_f64(c).mul_add(cell_px, THUMB_MARGIN);
            let cx1 = cx0 + cell_px;
            lines.push(view! {
                <line x1=cx0 y1=y x2=cx1 y2=y stroke=stroke stroke-width=sw stroke-linecap="round" />
            });
        }
    }
    for c in 1..n {
        let x = usize_to_f64(c).mul_add(cell_px, THUMB_MARGIN);
        for (r, row) in cell_cage_map.iter().enumerate() {
            let left = row[c - 1];
            let right = row[c];
            let thick = matches!((left, right), (Some(a), Some(b)) if a != b);
            let (stroke, sw) = if thick {
                (INK, THUMB_THICK_STROKE)
            } else {
                (LINE, THUMB_THIN_STROKE)
            };
            let cy0 = usize_to_f64(r).mul_add(cell_px, THUMB_MARGIN);
            let cy1 = cy0 + cell_px;
            lines.push(view! {
                <line x1=x y1=cy0 x2=x y2=cy1 stroke=stroke stroke-width=sw stroke-linecap="round" />
            });
        }
    }

    let op_font = (cell_px * 0.16).max(6.0);
    let op_labels: Vec<_> = rt
        .view
        .cages
        .iter()
        .map(|cage| {
            let (ar, ac) = cage_anchor(cage);
            let lx = usize_to_f64(ac).mul_add(cell_px, THUMB_MARGIN) + 2.0;
            let ly = usize_to_f64(ar).mul_add(cell_px, THUMB_MARGIN) + 2.0;
            let label = op_label(cage.op, cage.target);
            view! {
                <text
                    x=lx y=ly
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
        .collect();

    let outer_size = usize_to_f64(n) * cell_px;

    let ring_stroke = move || {
        if selected.get() {
            ACCENT
        } else {
            "transparent"
        }
    };

    view! {
        <svg
            viewBox=format!("0 0 {size} {size}")
            width=size
            height=size
            xmlns="http://www.w3.org/2000/svg"
            style="cursor:pointer;flex-shrink:0;"
            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                if ev.button() == 0 {
                    on_click.run(());
                }
            }
        >
            <rect x="0" y="0" width=size height=size fill=BG />
            {cell_fills}
            {highlights}
            {texts}
            {lines}
            <rect
                x=THUMB_MARGIN
                y=THUMB_MARGIN
                width=outer_size
                height=outer_size
                fill="none"
                stroke=INK
                stroke-width=THUMB_OUTER_STROKE
            />
            {op_labels}
            <rect
                x="1" y="1"
                width=size - 2
                height=size - 2
                fill="none"
                stroke=ring_stroke
                stroke-width="2.5"
                rx="3"
                pointer-events="none"
            />
        </svg>
    }
    .into_any()
}

// ─── CageBand component ──────────────────────────────────────────────────────

/// Vertical strip on the right side of the grid showing narrowing previews for the active cage.
///
/// When `active_cage_anchor` is `None`, collapses to a 4px placeholder bar.
/// When a cage is active, loads ranked tuples from the backend and renders
/// thumbnail previews with up/down scrolling and Enter-to-commit.
#[component]
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn CageBand(
    /// The anchor cell of the currently-active cage, or None.
    active_cage_anchor: Signal<Option<(usize, usize)>>,
    /// Cells of the currently-active cage (for highlighting in thumbnails).
    active_cage_cells: Signal<Vec<(usize, usize)>>,
    /// Callback invoked with the post-narrow PuzzleView when the user commits a tuple.
    on_commit: Callback<PuzzleView>,
) -> impl IntoView {
    let ranked = RwSignal::new(Vec::<RankedTuple>::new());
    let selected_idx = RwSignal::new(None::<usize>);
    let scroll_offset = RwSignal::new(0_usize);

    // Reload ranked tuples whenever the active cage changes.
    Effect::new(move |_| {
        let anchor = active_cage_anchor.get();
        ranked.set(vec![]);
        selected_idx.set(None);
        scroll_offset.set(0);
        if let Some(anchor) = anchor {
            spawn_local(async move {
                if let Some(tuples) = call_rank_active_cage(anchor).await {
                    ranked.set(tuples);
                }
            });
        }
    });

    let is_active = move || active_cage_anchor.get().is_some();

    let can_scroll_up = move || scroll_offset.get() > 0;
    let can_scroll_down = move || {
        let total = ranked.with(Vec::len);
        scroll_offset.get() + VISIBLE_COUNT < total
    };

    let scroll_up = move || {
        let off = scroll_offset.get();
        if off > 0 {
            scroll_offset.set(off - 1);
        }
    };

    let scroll_down = move || {
        if can_scroll_down() {
            scroll_offset.set(scroll_offset.get() + 1);
        }
    };

    let on_up = move |_| scroll_up();
    let on_down = move |_| scroll_down();

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                scroll_up();
            }
            "ArrowDown" => {
                ev.prevent_default();
                scroll_down();
            }
            "Escape" => {
                ev.prevent_default();
                selected_idx.set(None);
            }
            "Enter" => {
                ev.prevent_default();
                if let Some(idx) = selected_idx.get() {
                    let anchor = active_cage_anchor.get_untracked();
                    let tuple = ranked.with(|rs| rs.get(idx).map(|rt| rt.tuple.clone()));
                    if let (Some(anchor), Some(tuple)) = (anchor, tuple) {
                        spawn_local(async move {
                            if let Some(view) = call_apply_narrowing(anchor, tuple).await {
                                on_commit.run(view);
                                selected_idx.set(None);
                            }
                        });
                    }
                }
            }
            _ => {}
        }
    };

    view! {
        <div
            class="cage-band"
            class:cage-band--active=is_active
            class:cage-band--collapsed=move || !is_active()
            tabindex="0"
            on:keydown=on_keydown
        >
            {move || {
                if !is_active() {
                    return view! { <div class="cage-band__placeholder" /> }.into_any();
                }
                let rs = ranked.get();
                let cells = active_cage_cells.get();
                let off = scroll_offset.get();
                let visible_items: Vec<_> = rs
                    .into_iter()
                    .enumerate()
                    .skip(off)
                    .take(VISIBLE_COUNT)
                    .collect();
                view! {
                    <button
                        class="cage-band__arrow"
                        disabled=move || !can_scroll_up()
                        on:click=on_up
                        aria-label="Scroll up"
                    >
                        "▲"
                    </button>
                    <div class="cage-band__strip">
                        <For
                            each=move || visible_items.clone()
                            key=|(i, _)| *i
                            children=move |(i, rt)| {
                                let cells_clone = cells.clone();
                                let is_selected = move || selected_idx.get() == Some(i);
                                view! {
                                    <Thumbnail
                                        rt=rt
                                        active_cells=cells_clone
                                        selected=Signal::derive(is_selected)
                                        on_click=Callback::new(move |()| {
                                            let prev = selected_idx.get_untracked();
                                            selected_idx.set(if prev == Some(i) { None } else { Some(i) });
                                        })
                                    />
                                }
                            }
                        />
                    </div>
                    <button
                        class="cage-band__arrow"
                        disabled=move || !can_scroll_down()
                        on:click=on_down
                        aria-label="Scroll down"
                    >
                        "▼"
                    </button>
                }.into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(THUMB_STEP > THUMB_SIZE);
    const _: () = assert!(VISIBLE_COUNT > 0);
}
