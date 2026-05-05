mod app;
mod cage_band;
mod cage_colors;
mod cage_edit;
mod cage_index;
mod context_menu;
mod context_menu_view;
mod grid;
mod navigation;
mod operator_entry;
mod theme;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    });
}
