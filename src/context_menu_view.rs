use crate::app::{
    apply_edit, call_edit, dispatch_edit, sync_active_cage, ContextMenuState, DraftCage,
    FlipCellArgs, PuzzleView,
};
use crate::cage_edit::{delete_at, escape_at, splinter_at};
use crate::cage_index::cage_anchor;
use crate::operator_entry::{ActiveCage, OperatorEntry};
use crate::theme::{BG, INK, INK3, LINE, SANS_FONT};
use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;

fn item_style() -> String {
    format!("padding:5px 14px;cursor:pointer;white-space:nowrap;color:{INK};user-select:none;")
}

fn label_style() -> String {
    format!("padding:3px 14px 1px;font-size:11px;color:{INK3};pointer-events:none;")
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
    drafts: RwSignal<Vec<DraftCage>>,
    active_cage: RwSignal<Option<usize>>,
    cursor: RwSignal<(usize, usize)>,
    entry: RwSignal<Option<OperatorEntry>>,
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

    let sep_style = format!("height:1px;background:{LINE};margin:3px 0;");

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
                            apply_edit(set_puzzle, drafts, active_cage, action);
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
                            apply_edit(set_puzzle, drafts, active_cage, action);
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
                            apply_edit(set_puzzle, drafts, active_cage, action);
                            sync_active_cage(puzzle, cursor, active_cage);
                        }
                        close();
                    }
                >
                    "Delete cage"
                </div>
            })}
            {(!items.adjacent_targets.is_empty()).then({
                let targets = items.adjacent_targets;
                move || view! {
                    <div style=sep_style />
                    <div style=label_style()>"Flip to cage\u{2026}"</div>
                    {targets.iter().map(|ft| {
                        let anchor = ft.anchor;
                        let label = ft.label.clone();
                        let close2 = close;
                        view! {
                            <div
                                style=item_style()
                                on:mouseenter=item_enter
                                on:mouseleave=item_leave
                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                    ev.prevent_default();
                                    dispatch_edit(
                                        set_puzzle,
                                        drafts,
                                        Box::pin(call_edit("flip_cell", FlipCellArgs {
                                            cell: (r, c),
                                            target_anchor: anchor,
                                        })),
                                        None,
                                    );
                                    close2();
                                }
                            >
                                {format!("  {label}")}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                }
            })}
        </div>
    }
}
