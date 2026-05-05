// FlashOverlay is public API not yet wired into the main binary entry-point;
// suppress dead_code lint for this module.
#![allow(dead_code)]

use crate::diff::{flash_entries, FlashEntry, PuzzleDiff};
use leptos::prelude::*;

/// Overlay that renders a `PuzzleDiff` as a transient flash animation.
///
/// Positions itself as a `<g>` inside the grid SVG with `pointer-events: none`.
/// Removed digits flash with opacity 1→0 and a strikethrough tint.
/// Added digits highlight briefly with a glow.
/// Duration is controlled by the CSS variable `--flash-duration` (default 300ms).
/// When `prefers-reduced-motion: reduce` is set, the duration collapses to 0ms.
///
/// Reentrancy: each new diff increments a generation counter; stale `set_timeout`
/// callbacks check the generation before clearing the state.
#[component]
pub fn FlashOverlay(
    /// The current diff to animate. Each write triggers a new flash.
    diff: ReadSignal<PuzzleDiff>,
    /// SVG cell size in SVG units (same as passed to `Layout::new`).
    cell_size: f64,
    /// SVG margin in SVG units.
    margin: f64,
    /// Puzzle dimension n.
    n: usize,
) -> impl IntoView {
    let entries: RwSignal<Vec<FlashEntry>> = RwSignal::new(vec![]);
    let generation: RwSignal<u64> = RwSignal::new(0);

    Effect::new(move |_| {
        let current_diff = diff.get();
        if current_diff.is_empty() {
            return;
        }

        let gen = generation.get_untracked() + 1;
        generation.set(gen);

        entries.set(flash_entries(&current_diff, cell_size, margin, n));

        let duration_ms = read_flash_duration_ms().unwrap_or(300.0_f64);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let duration_millis = duration_ms.ceil() as u64;

        leptos::task::spawn_local(async move {
            gloo_timers::future::sleep(std::time::Duration::from_millis(duration_millis)).await;
            if generation.get_untracked() == gen && !entries.get_untracked().is_empty() {
                entries.set(vec![]);
            }
        });
    });

    view! {
        <g class="flash-overlay" pointer-events="none">
            {move || {
                entries
                    .get()
                    .into_iter()
                    .map(render_flash_entry)
                    .collect::<Vec<_>>()
            }}
        </g>
    }
}

fn render_flash_entry(entry: FlashEntry) -> impl IntoView {
    let class = if entry.removed {
        "flash-removed"
    } else {
        "flash-added"
    };
    view! {
        <text
            x=entry.x
            y=entry.y
            text-anchor="middle"
            dominant-baseline="central"
            class=class
        >
            {entry.value.to_string()}
        </text>
    }
}

fn read_flash_duration_ms() -> Option<f64> {
    let window = web_sys::window()?;
    let doc = window.document()?;
    let root = doc.document_element()?;
    let style = window.get_computed_style(&root).ok()??;
    let raw = style.get_property_value("--flash-duration").ok()?;
    let trimmed = raw.trim();
    trimmed.strip_suffix("ms").map_or_else(
        || {
            trimmed
                .strip_suffix('s')
                .and_then(|s_str| s_str.trim().parse::<f64>().ok())
                .map(|secs| secs * 1000.0)
        },
        |ms_str| ms_str.trim().parse::<f64>().ok(),
    )
}
