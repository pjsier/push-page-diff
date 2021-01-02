mod utils;

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

// Rust KV example https://github.com/koeninger/rust-kv-example/blob/master/src/lib.rs
// Skipping formatting due to outstanding bug in rustfmt for async
#[rustfmt::skip]
#[wasm_bindgen]
extern "C" {
    type KV;

    #[wasm_bindgen(static_method_of = KV)]
    pub async fn get(s: String) -> JsValue;
}

async fn future_stub() -> Result<usize, ()> {
    Ok(1)
}

#[wasm_bindgen]
pub async fn greet() -> String {
    // KV::get("key-0".to_string())
    //     .await
    //     .as_string()
    //     .unwrap_or_else(|| "".to_string())
    format!(
        "Hello, wasm-worker! from KV: {}",
        future_stub().await.unwrap_or(0)
    )
}
