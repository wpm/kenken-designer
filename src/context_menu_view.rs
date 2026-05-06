use crate::app::{
    apply_edit, sync_active_cage, ContextMenuState, DraftCage, MoveState, PuzzleView,
};
use crate::cage_edit::{delete_at, escape_at, splinter_at};
use crate::cage_index::cage_anchor;
use crate::operator_entry::{ActiveCage, OperatorEntry};
use crate::theme::{BG, INK, LINE, SANS_FONT};
use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;

fn item_style() -> String {
    format!("padding:5px 14px;cursor:pointer;white-space:nowrap;color:{INK};user-select:none;")
}

#[allow(clippy::needless_pass_by_value)]
fn item_enter(ev: leptos::ev::MouseEvent) {
    let t = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok());
    if let Some(el) = t {
        let _ = el.style().set_property("background", LINE);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn item_leave(ev: leptos::ev::MouseEvent) {
    let t = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok());
    if let Some(el) = t {
        let _ = el.style().remove_property("background");
    }
}

#[component]
#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
pub fn ContextMenu(
    state: ContextMenuState,
    puzzle: ReadSignal<Option<PuzzleView>>,
    set_puzzle: WriteSignal<Option<PuzzleView>>,
    set_flash_diff: WriteSignal<crate::diff::PuzzleDiff>,
    drafts: RwSignal<Vec<DraftCage>>,
    active_cage: RwSignal<Option<usize>>,
    cursor: RwSignal<(usize, usize)>,
    entry: RwSignal<Option<OperatorEntry>>,
    move_mode: RwSignal<Option<MoveState>>,
    on_close: Callback<()>,
) -> impl IntoView {
    let (r, c) = state.cell;
    let x = state.x;
    let y = state.y;
    let items = state.items;

    let close = move || on_close.run(());

    let menu_style = format!(
        "position:fixed;left:{x}px;top:{y}px;background:{BG};border:0.5px solid {LINE};\
         border-radius:4px;box-shadow:0 2px 8px rgba(0,0,0,0.15);z-index:1000;\
         font-family:{SANS_FONT};font-size:12.5px;min-width:160px;padding:3px 0;"
    );

    view! {
        <div style=menu_style>
            {items.set_operation.then(|| view! {
                <div
                    style=item_style()
                    on:mouseenter=item_enter
                    on:mouseleave=item_leave
                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        let active = puzzle.with_untracked(|opt| {
                            opt.as_ref().and_then(|v| {
                                crate::cage_index::cage_at(v, r, c).map(|idx| {
                                    let anchor = cage_anchor(&v.cages[idx]);
                                    let cage_op = v.cages[idx].op;
                                    let cage_target = v.cages[idx].target;
                                    (idx, anchor, cage_op, cage_target)
                                })
                            })
                        });
                        if let Some((idx, anchor, cage_op, cage_target)) = active {
                            cursor.set(anchor);
                            active_cage.set(Some(idx));
                            entry.set(Some(OperatorEntry {
                                cage: ActiveCage::Committed(idx),
                                op: Some(cage_op),
                                digits: if cage_target > 0 {
                                    cage_target.to_string()
                                } else {
                                    String::new()
                                },
                            }));
                        }
                        close();
                    }
                >
                    "Set operation\u{2026}"
                </div>
            })}
            {items.make_singleton.then(|| view! {
                <div
                    style=item_style()
                    on:mouseenter=item_enter
                    on:mouseleave=item_leave
                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        let action = puzzle.with_untracked(|opt| {
                            opt.as_ref().map(|v| {
                                drafts.with_untracked(|ds| splinter_at((r, c), v, ds.first()))
                            })
                        });
                        if let Some(action) = action {
                            apply_edit(set_puzzle, set_flash_diff, drafts, active_cage, action);
                            sync_active_cage(puzzle, cursor, active_cage);
                        }
                        close();
                    }
                >
                    "Make singleton"
                </div>
            })}
            {items.uncage.then(|| view! {
                <div
                    style=item_style()
                    on:mouseenter=item_enter
                    on:mouseleave=item_leave
                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        let action = puzzle.with_untracked(|opt| {
                            opt.as_ref().map(|v| {
                                drafts.with_untracked(|ds| escape_at((r, c), v, ds.first()))
                            })
                        });
                        if let Some(action) = action {
                            apply_edit(set_puzzle, set_flash_diff, drafts, active_cage, action);
                            sync_active_cage(puzzle, cursor, active_cage);
                        }
                        close();
                    }
                >
                    "Uncage"
                </div>
            })}
            {items.delete_cage.then(|| view! {
                <div
                    style=item_style()
                    on:mouseenter=item_enter
                    on:mouseleave=item_leave
                    on:mousedown=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        let action = puzzle.with_untracked(|opt| {
                            opt.as_ref().map(|v| {
                                drafts.with_untracked(|ds| delete_at((r, c), v, ds.first()))
                            })
                        });
                        if let Some(action) = action {
                            apply_edit(set_puzzle, set_flash_diff, drafts, active_cage, action);
                            sync_active_cage(puzzle, cursor, active_cage);
                        }
                        close();
                    }
                >
                    "Delete cage"
                </div>
            })}
            {items.can_move.then(|| {
                let close2 = close;
                view! {
                    <div
                        style=item_style()
                        on:mouseenter=item_enter
                        on:mouseleave=item_leave
                        on:mousedown=move |ev: leptos::ev::MouseEvent| {
                            ev.prevent_default();
                            // Enter move mode for the right-clicked cell
                            let targets = puzzle.with_untracked(|opt| {
                                opt.as_ref().map_or_else(Vec::new, |v| {
                                    crate::cage_edit::legal_move_targets(v, (r, c))
                                })
                            });
                            if !targets.is_empty() {
                                move_mode.set(Some(MoveState {
                                    cell: (r, c),
                                    targets,
                                    selected: None,
                                }));
                            }
                            close2();
                        }
                    >
                        "Move cell\u{2026}"
                    </div>
                }
            })}
        </div>
    }
}
