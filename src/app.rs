use leptos::ev::Event;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::theme;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize)]
struct NewPuzzleArgs {
    n: usize,
}

#[derive(Serialize)]
struct NoArgs {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PuzzleView {
    pub n: usize,
    pub cells: Vec<Vec<Vec<u8>>>,
    pub cages: Vec<CageView>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CageView {
    pub cells: Vec<(usize, usize)>,
    pub op: OpKind,
    pub target: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum OpKind {
    Add,
    Sub,
    Mul,
    Div,
    Given,
}

async fn invoke_view(cmd: &str, args: JsValue) -> Option<PuzzleView> {
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

async fn call_get_state() -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&NoArgs {}).ok()?;
    invoke_view("get_state", args).await
}

async fn call_new_puzzle(n: usize) -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&NewPuzzleArgs { n }).ok()?;
    invoke_view("new_puzzle", args).await
}

async fn call_undo() -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&NoArgs {}).ok()?;
    invoke_view("undo", args).await
}

async fn call_redo() -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&NoArgs {}).ok()?;
    invoke_view("redo", args).await
}

#[component]
pub fn App() -> impl IntoView {
    let (puzzle, set_puzzle) = signal::<Option<PuzzleView>>(None);

    spawn_local(async move {
        if let Some(view) = call_get_state().await {
            set_puzzle.set(Some(view));
        }
    });

    let on_size_change = move |ev: Event| {
        let value = event_target_value(&ev);
        let Ok(n) = value.parse::<usize>() else {
            return;
        };
        spawn_local(async move {
            if let Some(view) = call_new_puzzle(n).await {
                set_puzzle.set(Some(view));
            }
        });
    };

    let on_undo = move |_| {
        spawn_local(async move {
            if let Some(view) = call_undo().await {
                set_puzzle.set(Some(view));
            }
        });
    };

    let on_redo = move |_| {
        spawn_local(async move {
            if let Some(view) = call_redo().await {
                set_puzzle.set(Some(view));
            }
        });
    };

    let summary = move || match puzzle.get() {
        Some(v) => format!("{}×{} with {} cages", v.n, v.n, v.cages.len()),
        None => "loading…".to_string(),
    };

    let current_n =
        move || puzzle.get().map(|v| v.n.to_string()).unwrap_or_else(|| "4".to_string());

    let title_style = format!(
        "font-family: {serif}; color: {ink}; margin: 0;",
        serif = theme::SERIF_FONT,
        ink = theme::INK,
    );
    let header_style = format!(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 1rem 1.5rem; border-bottom: 1px solid {line}; background: {bg};",
        line = theme::LINE,
        bg = theme::BG,
    );
    let controls_style = format!("display: flex; gap: 0.75rem; align-items: center;");
    let main_style = format!(
        "padding: 2rem 1.5rem; font-family: {sans}; color: {ink2};",
        sans = theme::SANS_FONT,
        ink2 = theme::INK2,
    );
    let summary_style = format!(
        "font-family: {mono}; color: {ink}; font-size: 1.1rem;",
        mono = theme::MONO_FONT,
        ink = theme::INK,
    );

    view! {
        <div>
            <header style=header_style>
                <h1 style=title_style>"KenKen Designer"</h1>
                <div style=controls_style>
                    <label>
                        "Size: "
                        <select on:change=on_size_change prop:value=current_n>
                            <option value="3">"3"</option>
                            <option value="4">"4"</option>
                            <option value="5">"5"</option>
                            <option value="6">"6"</option>
                            <option value="7">"7"</option>
                            <option value="8">"8"</option>
                            <option value="9">"9"</option>
                        </select>
                    </label>
                    <button on:click=on_undo>"Undo"</button>
                    <button on:click=on_redo>"Redo"</button>
                </div>
            </header>
            <main style=main_style>
                <div style=summary_style>{summary}</div>
            </main>
        </div>
    }
}
