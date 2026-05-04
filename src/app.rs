use crate::grid::Grid;
use leptos::ev::Event;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::future::Future;
use wasm_bindgen::prelude::*;

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

#[allow(clippy::future_not_send)] // WASM single-threaded runtime; Send is meaningless here
async fn call<A: Serialize>(cmd: &str, args: A) -> Option<PuzzleView> {
    let args = serde_wasm_bindgen::to_value(&args).ok()?;
    let value = invoke(cmd, args).await;
    serde_wasm_bindgen::from_value(value).ok()
}

#[component]
pub fn App() -> impl IntoView {
    let (puzzle, set_puzzle) = signal::<Option<PuzzleView>>(None);

    let refresh = move |fut: std::pin::Pin<Box<dyn Future<Output = Option<PuzzleView>>>>| {
        spawn_local(async move {
            if let Some(view) = fut.await {
                set_puzzle.set(Some(view));
            }
        });
    };

    refresh(Box::pin(call("get_state", NoArgs {})));

    let on_size_change = move |ev: Event| {
        let Ok(n) = event_target_value(&ev).parse::<usize>() else {
            return;
        };
        refresh(Box::pin(call("new_puzzle", NewPuzzleArgs { n })));
    };

    let on_undo = move |_| refresh(Box::pin(call("undo", NoArgs {})));
    let on_redo = move |_| refresh(Box::pin(call("redo", NoArgs {})));

    let summary = move || match puzzle.get() {
        Some(v) => format!("{}×{} with {} cages", v.n, v.n, v.cages.len()),
        None => "loading…".to_string(),
    };

    let current_n = move || {
        puzzle
            .get()
            .map_or_else(|| "4".to_string(), |v| v.n.to_string())
    };

    view! {
        <header class="app-header">
            <h1 class="app-title">"KenKen Designer"</h1>
            <div class="app-controls">
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
        <main class="app-main">
            <div class="app-summary">{summary}</div>
            {move || puzzle.get().map(|view| view! { <Grid view=view size=560 /> })}
        </main>
    }
}
