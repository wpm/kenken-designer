use crate::app::PuzzleView;
use crate::cage_colors::{assign_cage_colors, build_cell_cage_map};
use crate::cage_index::cage_anchor;
use crate::grid::{ceil_sqrt, op_label, usize_to_f64, UNCAGED_FILL};
use crate::theme::{ACCENT, BG, CAGE_PALETTE, INK, INK3, LINE, SERIF_FONT};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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

/// Side length of each thumbnail (square). Sized to match the strip width
/// (192px) minus the strip's horizontal padding so thumbnails fill the band.
const THUMB_SIZE: u32 = 168;
/// Gap between thumbnails.
const THUMB_GAP: u32 = 8;
const THUMB_STEP: u32 = THUMB_SIZE + THUMB_GAP;

/// Conservative fallback for visible-thumbnail count when the strip element
/// has not yet been measured (e.g. the first render). `fits_in_strip` will
/// replace this once the DOM is laid out.
const DEFAULT_VISIBLE_COUNT: usize = 1;

/// CSS class prefix used to identify thumbnails by index for focus targeting.
const THUMB_IDX_CLASS_PREFIX: &str = "cage-band__thumb--idx-";

/// Nominal scroll animation duration. The actual wait honours
/// `prefers-reduced-motion` by reading `--scroll-anim-duration` from the
/// document root at runtime; if that resolves to 0ms this returns 0.
const SCROLL_ANIM_MS_NOMINAL: u32 = 200;

/// Read the effective `--scroll-anim-duration` from the document root (in ms).
/// Falls back to `SCROLL_ANIM_MS_NOMINAL` if the CSS variable is unavailable.
fn scroll_anim_ms() -> u32 {
    let raw = web_sys::window()
        .and_then(|win| {
            let el = win.document()?.document_element()?;
            win.get_computed_style(&el).ok().flatten().map(|s| {
                s.get_property_value("--scroll-anim-duration")
                    .unwrap_or_default()
            })
        })
        .unwrap_or_default();
    // CSS value is e.g. " 200ms" or " 0ms"
    raw.trim()
        .trim_end_matches("ms")
        .parse::<u32>()
        .unwrap_or(SCROLL_ANIM_MS_NOMINAL)
}

thread_local! {
    static FOCUSED_THUMB: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

// ─── Pure helpers (testable) ────────────────────────────────────────────────

/// Number of thumbnails (each occupying `step_px` including its gap) that
/// fit in `strip_height_px` pixels of available height. Always returns at
/// least 1 so the UI can render before the strip has been measured.
const fn fits_in_strip(strip_height_px: i32, step_px: u32) -> usize {
    if strip_height_px <= 0 || step_px == 0 {
        return 1;
    }
    #[allow(clippy::cast_sign_loss)]
    let h = strip_height_px as usize;
    let s = step_px as usize;
    let n = h / s;
    if n == 0 {
        1
    } else {
        n
    }
}

/// Largest valid scroll offset so that `offset + visible <= total`. Returns
/// 0 when everything fits (`visible >= total`).
const fn max_scroll_offset(visible: usize, total: usize) -> usize {
    total.saturating_sub(visible)
}

/// Whether the strip can be scrolled up from the given offset.
const fn can_scroll_up_at(offset: usize) -> bool {
    offset > 0
}

/// Whether the strip can be scrolled down to reveal further thumbnails.
const fn can_scroll_down_at(offset: usize, visible: usize, total: usize) -> bool {
    offset + visible < total
}

/// Whether `idx` falls within the visible window `[offset, offset + visible)`.
const fn is_in_visible_window(idx: usize, offset: usize, visible: usize) -> bool {
    visible > 0 && idx >= offset && idx < offset + visible
}

/// Compute the new (`focused`, `scroll_offset`) after pressing `ArrowUp` on
/// the thumbnail at `focused`. Returns `None` if focus is already at the top.
const fn arrow_up_target(focused: usize, scroll: usize) -> Option<(usize, usize)> {
    if focused == 0 {
        return None;
    }
    let new_focused = focused - 1;
    let new_scroll = if new_focused < scroll {
        new_focused
    } else {
        scroll
    };
    Some((new_focused, new_scroll))
}

/// "N Tuple"/"N Tuples" caption with English pluralization.
pub fn tuple_count_label(count: usize) -> String {
    if count == 1 {
        "1 Tuple".to_string()
    } else {
        format!("{count} Tuples")
    }
}

/// Compute the new (`focused`, `scroll_offset`) after pressing `ArrowDown`
/// on the thumbnail at `focused`. Returns `None` if focus is already at the
/// last thumbnail (or the list is empty).
const fn arrow_down_target(
    focused: usize,
    scroll: usize,
    visible: usize,
    total: usize,
) -> Option<(usize, usize)> {
    if total == 0 || focused + 1 >= total {
        return None;
    }
    let new_focused = focused + 1;
    let new_scroll = if visible > 0 && new_focused >= scroll + visible {
        new_focused + 1 - visible
    } else {
        scroll
    };
    Some((new_focused, new_scroll))
}

// ─── Thumbnail SVG ───────────────────────────────────────────────────────────

const THUMB_MARGIN: f64 = 6.0;
const THUMB_OUTER_STROKE: f64 = 1.5;
const THUMB_THICK_STROKE: f64 = 1.2;
const THUMB_THIN_STROKE: f64 = 0.3;
const THUMB_ACTIVE_OPACITY: &str = "0.22";

/// Draw a single thumbnail SVG for one `RankedTuple`, highlighting `active_cells`.
#[component]
fn Thumbnail(rt: RankedTuple, active_cells: Vec<(usize, usize)>) -> impl IntoView {
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

    view! {
        <svg
            viewBox=format!("0 0 {size} {size}")
            width=size
            height=size
            xmlns="http://www.w3.org/2000/svg"
            style="display:block;flex-shrink:0;pointer-events:none;"
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
        </svg>
    }
    .into_any()
}

// ─── DOM helpers ─────────────────────────────────────────────────────────────

/// Index of the focused thumbnail, if any. Updated by each thumb's on:focus /
/// on:blur handlers. Used by both the band's local keydown handler and the
/// global window handler in `app.rs` to detect when keys belong to the band.
pub fn focused_thumb_idx() -> Option<usize> {
    FOCUSED_THUMB.with(std::cell::Cell::get)
}

fn set_focused_thumb(idx: Option<usize>) {
    FOCUSED_THUMB.with(|c| c.set(idx));
}

fn request_animation_frame_once(f: impl FnOnce() + 'static) {
    let cb = Closure::once(f);
    if let Some(win) = web_sys::window() {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

// Guards against a stale rAF re-focusing a thumb after Escape clears selected_idx.
fn focus_thumb(idx: usize, still_valid: impl Fn() -> bool + 'static) {
    request_animation_frame_once(move || {
        if !still_valid() {
            return;
        }
        let Some(win) = web_sys::window() else { return };
        let Some(doc) = win.document() else { return };
        let selector = format!(".{THUMB_IDX_CLASS_PREFIX}{idx}");
        if let Ok(Some(el)) = doc.query_selector(&selector) {
            if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                let _ = html_el.focus();
            }
        }
    });
}

fn blur_active() {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Some(doc) = win.document() else {
        return;
    };
    if let Some(active) = doc.active_element() {
        if let Ok(html_el) = active.dyn_into::<web_sys::HtmlElement>() {
            let _ = html_el.blur();
        }
    }
}

// ─── CageBand component ──────────────────────────────────────────────────────

/// Vertical strip on the right side of the grid showing narrowing previews for the active cage.
///
/// Always renders an outlined column reserving the same horizontal slot in
/// the layout: when `active_cage_anchor` is `None` (or no tuples are ranked
/// yet) the strip shows a `"0 Tuples"` caption; when a cage is active it
/// loads ranked tuples from the backend and renders thumbnail previews with
/// up/down scrolling and Enter-to-commit.
#[component]
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn CageBand(
    /// The anchor cell of the currently-active cage, or None.
    active_cage_anchor: Signal<Option<(usize, usize)>>,
    /// Cells of the currently-active cage (for highlighting in thumbnails).
    active_cage_cells: Signal<Vec<(usize, usize)>>,
    /// Callback invoked with the post-narrow PuzzleView when the user commits a tuple.
    on_commit: Callback<PuzzleView>,
    /// Written with the current ranked-tuple count so the caller can display it outside the band.
    tuple_count: RwSignal<usize>,
) -> impl IntoView {
    let ranked = RwSignal::new(Vec::<RankedTuple>::new());
    let ranked_len = Memo::new(move |_| ranked.with(Vec::len));
    let selected_idx = RwSignal::new(None::<usize>);
    let scroll_offset = RwSignal::new(0_usize);
    let visible_count = RwSignal::new(DEFAULT_VISIBLE_COUNT);
    let strip_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Animation state ────────────────────────────────────────────────────────
    // render_offset lags scroll_offset during animation (holds the render
    // window start, including the one-item buffer in the direction of travel).
    let render_offset = RwSignal::new(0_usize);
    let anim_translate_px = RwSignal::new(0_i32);
    let strip_no_transition = RwSignal::new(false);
    let is_animating = RwSignal::new(false);
    // Incremented on anchor change to cancel any in-flight animation callback.
    let anim_gen = RwSignal::new(0_u32);
    // One extra item is rendered while animating to fill the buffer slot.
    let render_extra = Signal::derive(move || usize::from(is_animating.get()));

    Effect::new(move |_| {
        let anchor = active_cage_anchor.get();
        batch(move || {
            ranked.set(vec![]);
            selected_idx.set(None);
            scroll_offset.set(0);
            render_offset.set(0);
            anim_translate_px.set(0);
            strip_no_transition.set(false);
            is_animating.set(false);
            anim_gen.update(|g| *g = g.wrapping_add(1));
        });
        if let Some(anchor) = anchor {
            spawn_local(async move {
                if let Some(tuples) = call_rank_active_cage(anchor).await {
                    ranked.set(tuples);
                }
            });
        }
    });

    let measure_strip = move || {
        if let Some(el) = strip_ref.get_untracked() {
            let count = fits_in_strip(el.client_height(), THUMB_STEP);
            if count != visible_count.get_untracked() {
                visible_count.set(count);
            }
        }
    };
    Effect::new(move |_| {
        // Subscribe to ranked length so the strip is measured once it mounts.
        let _ = ranked.with(Vec::len);
        measure_strip();
    });
    window_event_listener(leptos::ev::resize, move |_| measure_strip());

    Effect::new(move |_| {
        let total = ranked.with(Vec::len);
        let vis = visible_count.get();
        let off = scroll_offset.get_untracked();
        let max_off = max_scroll_offset(vis, total);
        if off > max_off {
            scroll_offset.set(max_off);
            render_offset.set(max_off);
        }
    });

    Effect::new(move |_| {
        tuple_count.set(ranked_len.get());
    });

    let can_scroll_up = move || can_scroll_up_at(scroll_offset.get());
    let can_scroll_down = move || {
        let total = ranked.with(Vec::len);
        can_scroll_down_at(scroll_offset.get(), visible_count.get(), total)
    };

    let animate_scroll = move |direction: i32| {
        if is_animating.get_untracked() {
            return;
        }
        let off = scroll_offset.get_untracked();
        let total = ranked.with_untracked(Vec::len);
        let vis = visible_count.get_untracked();

        let new_off = if direction < 0 {
            if !can_scroll_up_at(off) {
                return;
            }
            off - 1
        } else {
            if !can_scroll_down_at(off, vis, total) {
                return;
            }
            off + 1
        };

        is_animating.set(true);
        let gen = anim_gen.get_untracked();
        #[allow(clippy::cast_possible_wrap)] // THUMB_STEP is a small layout constant
        let neg_step = -(THUMB_STEP as i32);

        if direction < 0 {
            // Scrolling up: render from new_off (one item earlier) and start the
            // strip translated up by one step. Two rAFs are needed: the first
            // lets the browser commit the snapped -THUMB_STEP position, the
            // second starts the CSS transition sliding back to 0.
            batch(move || {
                render_offset.set(new_off);
                strip_no_transition.set(true);
                anim_translate_px.set(neg_step);
            });
            request_animation_frame_once(move || {
                if anim_gen.get_untracked() != gen {
                    return;
                }
                request_animation_frame_once(move || {
                    if anim_gen.get_untracked() != gen {
                        return;
                    }
                    strip_no_transition.set(false);
                    anim_translate_px.set(0);
                });
            });
        } else {
            // Scrolling down: render from current offset (one extra item appended
            // below via render_extra) and animate the strip sliding up by one step.
            render_offset.set(off);
            anim_translate_px.set(neg_step);
        }

        let anim_ms = scroll_anim_ms();
        // After the CSS transition completes, commit the new offset and snap back.
        spawn_local(async move {
            TimeoutFuture::new(anim_ms).await;
            if anim_gen.get_untracked() != gen {
                return; // anchor changed mid-animation; stale callback
            }
            batch(move || {
                scroll_offset.set(new_off);
                render_offset.set(new_off);
                strip_no_transition.set(true);
                anim_translate_px.set(0);
                // Clamp selected_idx into the now-committed visible window so it
                // can't silently point at an off-screen thumbnail after fast input.
                if let Some(sel) = selected_idx.get_untracked() {
                    let vis = visible_count.get_untracked();
                    let total = ranked.with_untracked(Vec::len);
                    // Re-clamp new_off: visible_count may have changed during the
                    // animation timeout (e.g. a window resize), even though an
                    // anchor change is guarded by anim_gen above.
                    let off = new_off.min(max_scroll_offset(vis, total));
                    if !is_in_visible_window(sel, off, vis) {
                        selected_idx.set(None);
                    }
                }
            });
            // Re-enable transition after the no-transition snap has painted.
            request_animation_frame_once(move || {
                strip_no_transition.set(false);
                is_animating.set(false);
            });
        });
    };

    let on_up = move |_| animate_scroll(-1);
    let on_down = move |_| animate_scroll(1);

    let commit_selected = move || {
        let Some(idx) = selected_idx.get_untracked() else {
            return;
        };
        let anchor = active_cage_anchor.get_untracked();
        let tuple = ranked.with_untracked(|rs| rs.get(idx).map(|rt| rt.tuple.clone()));
        if let (Some(anchor), Some(tuple)) = (anchor, tuple) {
            spawn_local(async move {
                if let Some(view) = call_apply_narrowing(anchor, tuple).await {
                    on_commit.run(view);
                    selected_idx.set(None);
                }
            });
        }
    };

    let apply_arrow = move |target: Option<(usize, usize)>| {
        let Some((new_i, new_scroll)) = target else {
            return;
        };
        let old_scroll = scroll_offset.get_untracked();
        if new_scroll != old_scroll {
            let dir = if new_scroll > old_scroll {
                1_i32
            } else {
                -1_i32
            };
            animate_scroll(dir);
        }
        selected_idx.set(Some(new_i));
        // Set eagerly so the global dispatcher sees the new index on the very
        // next keydown, before the rAF-deferred DOM .focus() call fires.
        set_focused_thumb(Some(new_i));
        focus_thumb(new_i, move || selected_idx.get_untracked() == Some(new_i));
    };

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| match ev.key().as_str() {
        "ArrowUp" => {
            let Some(i) = focused_thumb_idx() else { return };
            ev.prevent_default();
            ev.stop_propagation();
            apply_arrow(arrow_up_target(i, scroll_offset.get_untracked()));
        }
        "ArrowDown" => {
            let Some(i) = focused_thumb_idx() else { return };
            ev.prevent_default();
            ev.stop_propagation();
            let total = ranked.with_untracked(Vec::len);
            let vis = visible_count.get_untracked();
            apply_arrow(arrow_down_target(
                i,
                scroll_offset.get_untracked(),
                vis,
                total,
            ));
        }
        "Escape" => {
            if focused_thumb_idx().is_none() {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            // Clear eagerly — don't rely on the blur event to update FOCUSED_THUMB,
            // since a stale Some(_) would cause the global dispatcher to keep
            // swallowing arrow keys even after focus has returned to the grid.
            set_focused_thumb(None);
            selected_idx.set(None);
            blur_active();
        }
        // Plain Enter commits the selected tuple. Shift+Enter is reserved for
        // the global "splinter cursor cell" shortcut, so let it bubble.
        "Enter" if !ev.shift_key() => {
            if focused_thumb_idx().is_none() {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            commit_selected();
        }
        _ => {}
    };

    view! {
        <div class="cage-band" on:keydown=on_keydown>
            <button
                class="cage-band__arrow"
                disabled=move || !can_scroll_up()
                on:click=on_up
                aria-label="Scroll up"
            >
                "▲"
            </button>
            <div class="cage-band__strip" node_ref=strip_ref>
                <div
                    class="cage-band__strip-inner"
                    class:cage-band__strip--no-transition=move || strip_no_transition.get()
                    style=move || format!("transform:translateY({}px)", anim_translate_px.get())
                >
                    {move || ranked.get().is_empty().then(|| view! { <div class="cage-band__empty"></div> })}
                    <For
                        each=move || {
                            let rs = ranked.get();
                            let off = render_offset.get();
                            let vis = visible_count.get();
                            let extra = render_extra.get();
                            rs.into_iter()
                                .enumerate()
                                .skip(off)
                                .take(vis + extra)
                                .collect::<Vec<_>>()
                        }
                        key=|(i, _)| *i
                        children=move |(i, rt)| {
                            let cells_clone = active_cage_cells.get_untracked();
                            let is_selected = move || selected_idx.get() == Some(i);
                            view! {
                                <div
                                    class=format!("cage-band__thumb {THUMB_IDX_CLASS_PREFIX}{i}")
                                    class:cage-band__thumb--selected=is_selected
                                    tabindex="0"
                                    on:focus=move |_| {
                                        set_focused_thumb(Some(i));
                                        if selected_idx.get_untracked() != Some(i) {
                                            selected_idx.set(Some(i));
                                        }
                                    }
                                    on:blur=move |_| {
                                        set_focused_thumb(None);
                                    }
                                >
                                    <Thumbnail
                                        rt=rt
                                        active_cells=cells_clone
                                    />
                                </div>
                            }
                        }
                    />
                </div>
            </div>
            <button
                class="cage-band__arrow"
                disabled=move || !can_scroll_down()
                on:click=on_down
                aria-label="Scroll down"
            >
                "▼"
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = assert!(THUMB_STEP > THUMB_SIZE);

    #[test]
    fn fits_in_strip_returns_one_for_zero_or_negative_height() {
        assert_eq!(fits_in_strip(0, 100), 1);
        assert_eq!(fits_in_strip(-5, 100), 1);
    }

    #[test]
    fn fits_in_strip_returns_one_when_height_is_below_one_step() {
        assert_eq!(fits_in_strip(1, 100), 1);
        assert_eq!(fits_in_strip(99, 100), 1);
    }

    #[test]
    fn fits_in_strip_returns_floor_of_height_over_step() {
        assert_eq!(fits_in_strip(100, 100), 1);
        assert_eq!(fits_in_strip(199, 100), 1);
        assert_eq!(fits_in_strip(200, 100), 2);
        assert_eq!(fits_in_strip(550, 100), 5);
    }

    #[test]
    fn fits_in_strip_handles_zero_step_gracefully() {
        assert_eq!(fits_in_strip(500, 0), 1);
    }

    #[test]
    fn max_scroll_offset_is_total_minus_visible() {
        assert_eq!(max_scroll_offset(3, 10), 7);
        assert_eq!(max_scroll_offset(10, 10), 0);
        assert_eq!(max_scroll_offset(20, 10), 0);
        assert_eq!(max_scroll_offset(0, 0), 0);
    }

    #[test]
    fn can_scroll_up_at_returns_true_only_when_offset_positive() {
        assert!(!can_scroll_up_at(0));
        assert!(can_scroll_up_at(1));
        assert!(can_scroll_up_at(42));
    }

    #[test]
    fn can_scroll_down_at_returns_false_when_everything_is_in_view() {
        // 5 total, 5 visible from offset 0 — nothing more below.
        assert!(!can_scroll_down_at(0, 5, 5));
        // 5 total, 3 visible, offset 2 — last fits exactly.
        assert!(!can_scroll_down_at(2, 3, 5));
    }

    #[test]
    fn can_scroll_down_at_returns_true_when_more_below() {
        assert!(can_scroll_down_at(0, 3, 10));
        assert!(can_scroll_down_at(6, 3, 10));
        assert!(!can_scroll_down_at(7, 3, 10));
    }

    #[test]
    fn can_scroll_down_at_handles_empty_list() {
        assert!(!can_scroll_down_at(0, 1, 0));
    }

    #[test]
    fn arrow_up_target_at_zero_returns_none() {
        assert_eq!(arrow_up_target(0, 0), None);
    }

    #[test]
    fn arrow_up_target_decrements_focus_within_visible() {
        // Focused at 3, scroll at 0 — moving up to 2 doesn't require scroll.
        assert_eq!(arrow_up_target(3, 0), Some((2, 0)));
        assert_eq!(arrow_up_target(7, 5), Some((6, 5)));
    }

    #[test]
    fn arrow_up_target_scrolls_when_target_above_view() {
        // Focused at 5, scroll at 5 — moving up needs scroll up by 1.
        assert_eq!(arrow_up_target(5, 5), Some((4, 4)));
        // Focused at 10, scroll at 8 — target 9 still in view.
        assert_eq!(arrow_up_target(10, 8), Some((9, 8)));
    }

    #[test]
    fn arrow_down_target_returns_none_when_at_last() {
        assert_eq!(arrow_down_target(9, 0, 3, 10), None);
    }

    #[test]
    fn arrow_down_target_returns_none_for_empty_list() {
        assert_eq!(arrow_down_target(0, 0, 3, 0), None);
    }

    #[test]
    fn arrow_down_target_increments_focus_within_visible() {
        // Focus 0, visible [0..3) — moving to 1 stays in view.
        assert_eq!(arrow_down_target(0, 0, 3, 10), Some((1, 0)));
        // Focus 1, visible [0..3) — moving to 2 stays in view.
        assert_eq!(arrow_down_target(1, 0, 3, 10), Some((2, 0)));
    }

    #[test]
    fn arrow_down_target_scrolls_when_target_below_view() {
        // Focus 2 at end of visible [0..3); pressing down needs to scroll.
        // visible window must end at new_focused (3), so scroll = 3 + 1 - 3 = 1.
        assert_eq!(arrow_down_target(2, 0, 3, 10), Some((3, 1)));
        // Focus 5 with scroll 3, visible 3 — moving to 6 needs scroll to 4.
        assert_eq!(arrow_down_target(5, 3, 3, 10), Some((6, 4)));
    }

    #[test]
    fn arrow_down_target_does_not_scroll_past_end() {
        // Focus 8 with scroll 6, visible 3; new_focused 9 is last; scroll stays.
        assert_eq!(arrow_down_target(8, 6, 3, 10), Some((9, 7)));
    }

    #[test]
    fn is_in_visible_window_returns_false_for_zero_visible() {
        assert!(!is_in_visible_window(0, 0, 0));
        assert!(!is_in_visible_window(5, 3, 0));
    }

    #[test]
    fn is_in_visible_window_returns_true_for_idx_in_range() {
        assert!(is_in_visible_window(0, 0, 3));
        assert!(is_in_visible_window(2, 0, 3));
        assert!(is_in_visible_window(5, 3, 4));
        assert!(is_in_visible_window(6, 3, 4));
    }

    #[test]
    fn is_in_visible_window_returns_false_for_idx_outside_range() {
        assert!(!is_in_visible_window(3, 0, 3)); // one past end
        assert!(!is_in_visible_window(2, 3, 4)); // before offset
        assert!(!is_in_visible_window(7, 3, 4)); // past end
    }

    #[test]
    fn tuple_count_label_singular_for_one() {
        assert_eq!(tuple_count_label(1), "1 Tuple");
    }

    #[test]
    fn tuple_count_label_plural_for_zero() {
        assert_eq!(tuple_count_label(0), "0 Tuples");
    }

    #[test]
    fn tuple_count_label_plural_for_many() {
        assert_eq!(tuple_count_label(2), "2 Tuples");
        assert_eq!(tuple_count_label(7), "7 Tuples");
        assert_eq!(tuple_count_label(100), "100 Tuples");
    }
}
