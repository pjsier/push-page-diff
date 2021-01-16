use cfg_if::cfg_if;
use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};
use js_sys::JsString;

use serde_derive::{Deserialize, Serialize};
use serde_json;
use scraper::{Html, Selector};

mod utils;

use utils::worker_global_scope;

// https://github.com/snoyberg/sortasecret

const VAPID_PUBLIC_KEY: &str = env!("VAPID_PUBLIC_KEY");
const VAPID_PRIVATE_KEY: &str = env!("VAPID_PRIVATE_KEY");

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
    pub async fn put(key: String, value: String, opts: JsValue) -> JsValue;

    #[wasm_bindgen(static_method_of = KV)]
    pub async fn list(opts: JsValue) -> JsValue;

    #[wasm_bindgen(static_method_of = KV)]
    pub async fn delete(s: String) -> JsValue;
}

macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}


#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVListOptions {
    prefix: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVKey {
    name: String,
    expiration: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVListResponse {
    keys: Vec<KVKey>
}

async fn keys_for_prefix(prefix: String) -> Result<Vec<String>, JsValue> {
    let opts = KVListOptions { prefix };
    let res_val = KV::list(JsValue::from_serde(&opts).unwrap()).await;
    let res: KVListResponse = res_val.into_serde().map_err(|e| e.to_string())?;
    Ok(res.keys.into_iter().map(|k| k.name).collect())
}


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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Subscription {
    diff: String,
    endpoint: String,
    auth: String,
    p256dh: String,
}

async fn create_site_if_not_exists(sub: Subscription) -> Result<(), JsValue> {
    let key = format!("site:{}", sub.diff);
    let res = KV::get(key.clone()).await;
    if res.is_null() {
        let html_str = load_html(sub.diff).await?;
        // TODO: Implement options here
        KV::put(key, html_str, JsValue::NULL).await;
    }
    Ok(())
}

async fn create_sub(sub: Subscription) -> Result<(), JsValue> {
    KV::put(
        format!("sub:{}:{}", sub.diff, sub.p256dh),
        serde_json::to_string(&sub).map_err(|e| e.to_string())?,
        JsValue::NULL,
    )
    .await;
    Ok(())
}

// TODO: Handle request like https://github.com/thedamian/Web-Push-Example/blob/master/server.js
#[wasm_bindgen]
pub async fn register_subscription(payload: JsValue) -> Result<(), JsValue> {
    let sub: Subscription = payload.into_serde().map_err(|e| e.to_string())?;
    // TODO: Join?
    create_site_if_not_exists(sub.clone()).await?;
    create_sub(sub).await?;

    Ok(())
}

async fn handle_sub_change(sub: Subscription) -> Result<(), JsValue> {
    push_notification(&sub.endpoint).await?;
    KV::delete(format!("sub:{}:{}", sub.endpoint, sub.p256dh)).await;
    Ok(())
}

// https://github.com/pimeys/rust-web-push/blob/master/src/services/autopush.rs
// Not sending payload (generated in service worker, so simpler)
async fn push_notification(endpoint: &str) -> Result<(), JsValue> {
    let global = worker_global_scope().ok_or(JsValue::NULL)?;
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&JsValue::from_str("")));
    // TODO: Is TTL needed?

    let request = Request::new_with_str_and_init(&endpoint, &opts)?;
    let resp_value = JsFuture::from(global.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into()?;
    if resp.ok() {
        Ok(())
    } else {
        Err(JsValue::from(resp))
    }
}

async fn has_url_changed(url: String, content: String) -> bool {
    if let Ok(new_content) = load_html(url).await {
        content == new_content
    } else {
        false
    }
}

// TODO: Operate with concurrency
async fn handle_site_change(url: String) -> Result<(), JsValue> {
    Ok(())
}

// TODO:
// Two types of keys, "site:" and "sub:".
// First should follow pattern "site:<URL>"
// Second should follow pattern "sub:<URL>:<RAND>:<ID>"
// On each query, do "site:*" to check all URLs. If there is a change, query for "sub:<URL>:*", then
// notify and delete all keys that are returned
// Handle schedule event
#[wasm_bindgen]
pub async fn check_diffs_and_push() -> Result<(), JsValue> {
    let all_keys = keys_for_prefix("site:".to_string()).await?;
    // TODO: For each one, check if it's changed, run notifications if so
    Ok(())
}
