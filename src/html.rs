use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};
use js_sys::JsString;

use scraper::{Html, Selector};

use crate::utils::worker_global_scope;

async fn parse_html_text(html_text: String) -> Result<String, JsValue> {
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

pub async fn load_html_text(url: String) -> Result<String, JsValue> {
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
