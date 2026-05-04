use crate::cage_index::cage_at;
use crate::grid::Grid;
use crate::navigation::{next_state, NavKey};
use leptos::ev::{Event, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
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

fn is_text_input_focused() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(doc) = window.document() else {
        return false;
    };
    let Some(element) = doc.active_element() else {
        return false;
    };
    is_text_input_tag(&element.tag_name())
}

const fn is_text_input_tag(tag: &str) -> bool {
    tag.as_bytes().eq_ignore_ascii_case(b"INPUT")
        || tag.as_bytes().eq_ignore_ascii_case(b"TEXTAREA")
        || tag.as_bytes().eq_ignore_ascii_case(b"SELECT")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Undo,
    Redo,
    Navigate(NavKey),
    Ignore,
}

fn dispatch_key(key: &str, shift: bool, modifier: bool, in_text_input: bool) -> KeyAction {
    if in_text_input {
        return KeyAction::Ignore;
    }
    if modifier && key.eq_ignore_ascii_case("z") {
        return if shift {
            KeyAction::Redo
        } else {
            KeyAction::Undo
        };
    }
    NavKey::from_key(key, shift).map_or(KeyAction::Ignore, KeyAction::Navigate)
}

#[component]
pub fn App() -> impl IntoView {
    let (puzzle, set_puzzle) = signal::<Option<PuzzleView>>(None);
    let cursor = RwSignal::new((0_usize, 0_usize));
    let active_cage = RwSignal::new(None::<usize>);

    refresh_from(set_puzzle, Box::pin(call("get_state", NoArgs {})));
    listen_for_puzzle_updates(set_puzzle);

    let on_size_change = move |ev: Event| {
        let Ok(n) = event_target_value(&ev).parse::<usize>() else {
            return;
        };
        cursor.set((0, 0));
        active_cage.set(None);
        refresh_from(
            set_puzzle,
            Box::pin(call("new_puzzle", NewPuzzleArgs { n })),
        );
    };

    let current_n = move || {
        puzzle
            .get()
            .map_or_else(|| "4".to_string(), |v| v.n.to_string())
    };

    let on_cell_click = Callback::new(move |(r, c): (usize, usize)| {
        if cursor.get_untracked() != (r, c) {
            cursor.set((r, c));
        }
        let next_active = puzzle.with_untracked(|opt| opt.as_ref().and_then(|v| cage_at(v, r, c)));
        if active_cage.get_untracked() != next_active {
            active_cage.set(next_active);
        }
    });

    install_keydown_handler(puzzle, set_puzzle, cursor, active_cage);

    view! {
        <main class="app-main">
            {move || puzzle.get().map(|view| view! {
                <Grid
                    view=view
                    size=560
                    cursor=cursor.into()
                    active_cage=active_cage.into()
                    on_cell_click=on_cell_click
                />
            })}
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

fn install_keydown_handler(
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    cursor: RwSignal<(usize, usize)>,
    active_cage: RwSignal<Option<usize>>,
) {
    window_event_listener(leptos::ev::keydown, move |ev: KeyboardEvent| {
        let modifier = ev.meta_key() || ev.ctrl_key();
        let action = dispatch_key(&ev.key(), ev.shift_key(), modifier, is_text_input_focused());
        match action {
            KeyAction::Ignore => {}
            KeyAction::Undo => {
                ev.prevent_default();
                refresh_from(set_puzzle, Box::pin(call("undo", NoArgs {})));
            }
            KeyAction::Redo => {
                ev.prevent_default();
                refresh_from(set_puzzle, Box::pin(call("redo", NoArgs {})));
            }
            KeyAction::Navigate(nav_key) => {
                ev.prevent_default();
                let next = puzzle.with_untracked(|opt| {
                    opt.as_ref().map(|v| {
                        next_state(
                            cursor.get_untracked(),
                            active_cage.get_untracked(),
                            v.n,
                            &v.cages,
                            nav_key,
                        )
                    })
                });
                if let Some((next_cursor, next_active)) = next {
                    if cursor.get_untracked() != next_cursor {
                        cursor.set(next_cursor);
                    }
                    if active_cage.get_untracked() != next_active {
                        active_cage.set(next_active);
                    }
                }
            }
        }
    });
}

fn refresh_from(
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    fut: Pin<Box<dyn Future<Output = Option<PuzzleView>>>>,
) {
    spawn_local(async move {
        if let Some(view) = fut.await {
            set_puzzle.set(Some(view));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_text_input_tag_matches_form_controls() {
        assert!(is_text_input_tag("INPUT"));
        assert!(is_text_input_tag("input"));
        assert!(is_text_input_tag("Textarea"));
        assert!(is_text_input_tag("SELECT"));
    }

    #[test]
    fn is_text_input_tag_rejects_other_elements() {
        assert!(!is_text_input_tag("DIV"));
        assert!(!is_text_input_tag("BUTTON"));
        assert!(!is_text_input_tag("svg"));
        assert!(!is_text_input_tag(""));
    }

    #[test]
    fn dispatch_key_returns_undo_for_modifier_z() {
        assert_eq!(dispatch_key("z", false, true, false), KeyAction::Undo);
        assert_eq!(dispatch_key("Z", false, true, false), KeyAction::Undo);
    }

    #[test]
    fn dispatch_key_returns_redo_for_modifier_shift_z() {
        assert_eq!(dispatch_key("z", true, true, false), KeyAction::Redo);
        assert_eq!(dispatch_key("Z", true, true, false), KeyAction::Redo);
    }

    #[test]
    fn dispatch_key_ignores_z_without_modifier() {
        assert_eq!(dispatch_key("z", false, false, false), KeyAction::Ignore);
        assert_eq!(dispatch_key("z", true, false, false), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_returns_navigate_for_arrow_keys() {
        assert_eq!(
            dispatch_key("ArrowUp", false, false, false),
            KeyAction::Navigate(NavKey::ArrowUp)
        );
        assert_eq!(
            dispatch_key("ArrowDown", false, false, false),
            KeyAction::Navigate(NavKey::ArrowDown)
        );
        assert_eq!(
            dispatch_key("ArrowLeft", false, false, false),
            KeyAction::Navigate(NavKey::ArrowLeft)
        );
        assert_eq!(
            dispatch_key("ArrowRight", false, false, false),
            KeyAction::Navigate(NavKey::ArrowRight)
        );
    }

    #[test]
    fn dispatch_key_returns_navigate_for_tab_and_shift_tab() {
        assert_eq!(
            dispatch_key("Tab", false, false, false),
            KeyAction::Navigate(NavKey::Tab)
        );
        assert_eq!(
            dispatch_key("Tab", true, false, false),
            KeyAction::Navigate(NavKey::ShiftTab)
        );
    }

    #[test]
    fn dispatch_key_ignores_when_text_input_focused() {
        assert_eq!(dispatch_key("z", false, true, true), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("ArrowUp", false, false, true),
            KeyAction::Ignore
        );
        assert_eq!(dispatch_key("Tab", false, false, true), KeyAction::Ignore);
    }

    #[test]
    fn dispatch_key_ignores_unrelated_keys() {
        assert_eq!(dispatch_key("a", false, false, false), KeyAction::Ignore);
        assert_eq!(
            dispatch_key("Enter", false, false, false),
            KeyAction::Ignore
        );
        assert_eq!(dispatch_key("", false, false, false), KeyAction::Ignore);
    }
}
