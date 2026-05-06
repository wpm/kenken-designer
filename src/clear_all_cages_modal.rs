use crate::theme::{ACCENT, BG, INK, INK2, LINE, SANS_FONT};
use leptos::prelude::*;

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn ClearAllCagesModal(
    cage_count: usize,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let overlay_style = "position:fixed;inset:0;background:rgba(0,0,0,0.35);z-index:2000;\
         display:flex;align-items:center;justify-content:center;";

    let dialog_style = format!(
        "background:{BG};border:0.5px solid {LINE};border-radius:8px;\
         box-shadow:0 4px 24px rgba(0,0,0,0.2);padding:24px 28px;\
         font-family:{SANS_FONT};min-width:280px;max-width:380px;"
    );

    let title_style = format!("font-size:16px;font-weight:600;color:{INK};margin:0 0 10px 0;");

    let body_style = format!("font-size:13.5px;color:{INK2};margin:0 0 20px 0;");

    let button_row_style = "display:flex;justify-content:flex-end;gap:10px;";

    let cancel_style = format!(
        "padding:6px 16px;border:0.5px solid {LINE};border-radius:5px;\
         background:{BG};color:{INK};font-family:{SANS_FONT};\
         font-size:13px;cursor:pointer;"
    );

    let confirm_style = format!(
        "padding:6px 16px;border:none;border-radius:5px;\
         background:{ACCENT};color:#fff;font-family:{SANS_FONT};\
         font-size:13px;cursor:pointer;"
    );

    let body_text = format!("This will remove all {cage_count} cages.");

    let cancel = move |_: leptos::ev::MouseEvent| on_cancel.run(());
    let confirm = move |_: leptos::ev::MouseEvent| on_confirm.run(());

    view! {
        <div
            style=overlay_style
            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                if ev.target() == ev.current_target() {
                    on_cancel.run(());
                }
            }
        >
            <div style=dialog_style>
                <p style=title_style>"Clear all cages?"</p>
                <p style=body_style>{body_text}</p>
                <div style=button_row_style>
                    <button
                        style=cancel_style
                        autofocus=true
                        on:click=cancel
                    >
                        "Cancel"
                    </button>
                    <button
                        style=confirm_style
                        on:click=confirm
                    >
                        "Clear all"
                    </button>
                </div>
            </div>
        </div>
    }
}
