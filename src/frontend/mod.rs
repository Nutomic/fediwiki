pub mod api;
pub mod app;
pub mod error;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::frontend::app::App;
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
