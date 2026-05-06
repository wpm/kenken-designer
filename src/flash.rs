use crate::diff::{flash_entries, FlashEntry, PuzzleDiff};
use crate::grid::Layout;
use leptos::prelude::*;

#[component]
pub fn FlashOverlay(diff: ReadSignal<PuzzleDiff>, layout: Layout) -> impl IntoView {
    let entries: RwSignal<Vec<FlashEntry>> = RwSignal::new(vec![]);
    let generation: RwSignal<u64> = RwSignal::new(0);
    let font_size = layout.candidate_font();

    Effect::new(move |_| {
        let current_diff = diff.get();
        if current_diff.is_empty() {
            return;
        }

        let gen = generation.get_untracked() + 1;
        generation.set(gen);

        entries.set(flash_entries(&current_diff, layout));

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
                    .map(|entry| render_flash_entry(entry, font_size))
                    .collect::<Vec<_>>()
            }}
        </g>
    }
}

fn render_flash_entry(entry: FlashEntry, font_size: f64) -> impl IntoView {
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
            font-size=font_size
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
