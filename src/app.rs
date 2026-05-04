use crate::grid::Grid;
use leptos::ev::Event;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::future::Future;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

const PUZZLE_UPDATED_EVENT: &str = "puzzle-updated";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
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

fn listen_for_puzzle_updates(set_puzzle: WriteSignal<Option<PuzzleView>>) {
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
            if let Ok(view) = serde_wasm_bindgen::from_value::<PuzzleView>(payload) {
                set_puzzle.set(Some(view));
            }
        }
    });
    let _ = listen(PUZZLE_UPDATED_EVENT, &cb);
    cb.forget();
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
    listen_for_puzzle_updates(set_puzzle);

    let on_size_change = move |ev: Event| {
        let Ok(n) = event_target_value(&ev).parse::<usize>() else {
            return;
        };
        refresh(Box::pin(call("new_puzzle", NewPuzzleArgs { n })));
    };

    let current_n = move || {
        puzzle
            .get()
            .map_or_else(|| "4".to_string(), |v| v.n.to_string())
    };

    view! {
        <main class="app-main">
            {move || puzzle.get().map(|view| view! { <Grid view=view size=560 /> })}
            <div class="size-control">
                <label>
                    "Size: "
                    <select on:change=on_size_change prop:value=current_n>
                        <option value="2">"2"</option>
                        <option value="3">"3"</option>
                        <option value="4">"4"</option>
                        <option value="5">"5"</option>
                        <option value="6">"6"</option>
                        <option value="7">"7"</option>
                        <option value="8">"8"</option>
                        <option value="9">"9"</option>
                    </select>
                </label>
            </div>
        </main>
    }
}
