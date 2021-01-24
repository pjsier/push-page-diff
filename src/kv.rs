use wasm_bindgen::prelude::*;

use serde_derive::{Deserialize, Serialize};

// Rust KV example https://github.com/koeninger/rust-kv-example/blob/master/src/lib.rs
// Skipping formatting due to outstanding bug in rustfmt for async
#[rustfmt::skip]
#[wasm_bindgen]
extern "C" {
    pub type DIFF_KV;

    #[wasm_bindgen(static_method_of = DIFF_KV)]
    pub async fn get(s: String, type_: Option<String>) -> JsValue;

    #[wasm_bindgen(catch, static_method_of = DIFF_KV)]
    pub async fn put(key: String, value: String, opts: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, static_method_of = DIFF_KV)]
    pub async fn list(opts: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(static_method_of = DIFF_KV)]
    pub async fn delete(s: String) -> JsValue;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVListOptions {
    prefix: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVKey {
    name: String,
    expiration: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct KVListResponse {
    keys: Vec<KVKey>,
}

pub async fn keys_for_prefix(prefix: String) -> Result<Vec<String>, JsValue> {
    let opts = KVListOptions { prefix };
    let res_val = DIFF_KV::list(JsValue::from_serde(&opts).unwrap()).await;
    let res: KVListResponse = res_val?.into_serde().map_err(|e| e.to_string())?;
    Ok(res.keys.into_iter().map(|k| k.name).collect())
}
