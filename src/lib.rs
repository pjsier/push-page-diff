use futures::future::try_join_all;

use wasm_bindgen::prelude::*;

use cfg_if::cfg_if;
use serde_derive::{Deserialize, Serialize};

mod crypto;
mod html;
mod kv;
pub mod push;
mod utils;

use html::load_html_text;
use kv::{keys_for_prefix, DIFF_KV};
use push::push_notification;

cfg_if! {
    // When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
    // allocator.
    if #[cfg(feature = "wee_alloc")] {
        // extern crate wee_alloc;
        #[global_allocator]
        static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
    }
}

// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen(js_namespace = console)]
//     fn log(s: &str);
// }

// #[macro_use]
// macro_rules! console_log {
//     // Note that this is using the `log` function imported above during
//     // `bare_bones`
//     ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Subscription {
    pub diff: String,
    pub endpoint: String,
    pub auth: String,
    pub p256dh: String,
}

async fn create_site_if_not_exists(sub: Subscription) -> Result<(), JsValue> {
    let key = format!("site:{}", sub.diff);
    let res = DIFF_KV::get(key.clone(), None).await;
    if res.is_null() {
        let html_str = load_html_text(sub.diff).await?;
        DIFF_KV::put(key, html_str, JsValue::NULL).await?;
    }
    Ok(())
}

async fn create_sub(sub: Subscription) -> Result<(), JsValue> {
    DIFF_KV::put(
        format!("sub:{}:{}", sub.diff, sub.p256dh),
        serde_json::to_string(&sub).map_err(|e| e.to_string())?,
        JsValue::NULL,
    )
    .await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn register_subscription(payload: JsValue) -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let sub: Subscription = payload.into_serde().map_err(|e| e.to_string())?;
    // TODO: Join?
    create_site_if_not_exists(sub.clone()).await?;
    create_sub(sub).await?;
    Ok(())
}

async fn handle_sub_change(key: String) -> Result<(), JsValue> {
    let sub_val = DIFF_KV::get(key, Some("json".to_string())).await;
    let sub: Subscription = sub_val.into_serde().map_err(|e| e.to_string())?;
    push_notification(sub.clone()).await?;
    DIFF_KV::delete(format!("sub:{}:{}", sub.diff, sub.p256dh)).await;
    Ok(())
}

async fn has_url_changed(url: String, content: String) -> bool {
    if let Ok(new_content) = load_html_text(url).await {
        content != new_content
    } else {
        true
    }
}

// TODO: Operate with concurrency, max simultaneous
async fn handle_site_change(url: String) -> Result<(), JsValue> {
    let prefix = format!("sub:{}:", url);
    let all_keys = keys_for_prefix(prefix.clone()).await?;

    try_join_all(all_keys.into_iter().map(handle_sub_change)).await?;

    DIFF_KV::delete(format!("site:{}", url)).await;
    Ok(())
}

async fn check_update_site(key: String) -> Result<(), JsValue> {
    let res = DIFF_KV::get(key.clone(), None).await;
    let url = key.replace("site:", "");
    if let Some(content) = res.as_string() {
        if has_url_changed(url.clone(), content).await {
            handle_site_change(url).await?;
        }
    }
    Ok(())
}

// Two types of keys, "site:" and "sub:".
// First should follow pattern "site:<URL>"
// Second should follow pattern "sub:<URL>:<RAND>:<ID>"
// On each query, do "site:*" to check all URLs. If there is a change, query for "sub:<URL>:*", then
// notify and delete all keys that are returned
// Handle schedule event
#[wasm_bindgen]
pub async fn check_diffs_and_push() -> Result<(), JsValue> {
    let all_keys = keys_for_prefix("site:".to_string()).await?;
    try_join_all(all_keys.into_iter().map(check_update_site)).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn send_push(sub_val: JsValue) -> Result<(), JsValue> {
    let subscription = sub_val.into_serde().map_err(|e| e.to_string())?;
    push_notification(subscription).await
}
