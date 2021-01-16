mod utils;

use cfg_if::cfg_if;
use js_sys::JsString;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use scraper::{Html, Selector};

// const VAPID_PUBLIC_KEY: &str = env!("VAPID_PUBLIC_KEY", "");
// const VAPID_PRIVATE_KEY: &str = env!("VAPID_PRIVATE_KEY", "");
const VAPID_PUBLIC_KEY: &str = "test";
const VAPID_PRIVATE_KEY: &str = "test";

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        // extern crate wee_alloc;
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

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(static_method_of = KV)]
    pub async fn get(s: String) -> JsValue;

    #[wasm_bindgen(static_method_of = KV)]
    pub async fn list(opts: JsValue) -> JsValue;
}

macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

async fn future_stub() -> Result<&'static str, ()> {
    Ok(VAPID_PUBLIC_KEY)
}

// Only handling body, token, auth, endpoint? URL
// TODO: Handle request like https://github.com/thedamian/Web-Push-Example/blob/master/server.js
// pub async fn

// TODO:
// Two types of keys, "site:" and "sub:".
// First should follow pattern "site:<URL>"
// Second should follow pattern "sub:<URL>:<RAND>:<ID>"
// On each query, do "site:*" to check all URLs. If there is a change, query for "sub:<URL>:*", then
// notify and delete all keys that are returned
// Handle schedule event
// #[wasm_bindgen]
// pub async fn check_diffs_and_push() {

// }

#[wasm_bindgen]
pub async fn register_subscription(payload: JsValue) -> Result<String, JsValue> {
    Ok("test".to_string())
    // TODO: Should set initial key for URL if it doesn't exist, add sub:* key
}

#[wasm_bindgen]
pub async fn parse_html_text(html_text: String) -> Result<String, JsValue> {
    // Have to use scraper crate because DomParser isn't present in SW context
    let document = Html::parse_document(&html_text);
    let selector = Selector::parse("body *:not(script)").map_err(|_| JsValue::NULL)?;
    let doc_text = document
        .select(&selector)
        .flat_map(|e| e.text())
        .collect::<Vec<&str>>()
        .join(" ");

    // Using split_whitespace to split on all whitespace characters (including repeated) and then
    // join back with single space separators
    Ok(doc_text
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" "))
}

#[wasm_bindgen]
pub async fn load_html(url: String) -> Result<String, JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request.headers().set("Accept", "text/html")?;

    let global = worker_global_scope().ok_or(JsValue::NULL)?;
    let resp_value = JsFuture::from(global.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into()?;

    let html_text_val = JsFuture::from(resp.text()?).await?;
    let html_text: &JsString = html_text_val.dyn_ref().ok_or(JsValue::NULL)?;

    let clean_text = parse_html_text(html_text.into()).await?;
    Ok(clean_text)
}

#[wasm_bindgen]
pub async fn greet() -> String {
    // KV::get("key-0".to_string())
    //     .await
    //     .as_string()
    //     .unwrap_or_else(|| "".to_string())
    format!(
        "Hello, wasm-worker! from KV: {}",
        future_stub().await.unwrap_or("0")
    )
}

// Using window doesn't work because not in the Cloudflare worker context
// https://www.fpcomplete.com/blog/serverless-rust-wasm-cloudflare/
#[wasm_bindgen]
pub fn worker_global_scope() -> Option<web_sys::ServiceWorkerGlobalScope> {
    js_sys::global()
        .dyn_into::<web_sys::ServiceWorkerGlobalScope>()
        .ok()
}
