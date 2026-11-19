use wasm_bindgen::prelude::*;

// Import the `console.log` function from the browser
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! console_log {
    ( $( $t:tt )* ) => {
        log(&format!( $( $t )* ))
    }
}

// Export a `greet` function from Rust to JavaScript, that alerts a hello message.
#[wasm_bindgen]
pub fn greet(name: &str) {
    console_log!("Hello, {}!", name);
}

// Re-export existing modules for WASM compatibility
pub mod graph;
pub mod utilities;
pub mod wasm_graph;
pub mod wasm_calculator;
pub mod data_management;

// Only include Yew frontend for non-WASM targets and when not testing
#[cfg(all(not(target_arch = "wasm32"), not(test)))]
pub mod yew_frontend;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod wasm_tests;