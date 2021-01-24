use js_sys::{self, JsString};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, Url};

use base64;
use jwt_simple::prelude::*;

use crate::utils::worker_global_scope;

fn create_vapid_jwt(endpoint: String, sub: String) -> Result<String, String> {
    let url = Url::new(&endpoint).map_err(|_| "".to_string())?;
    let audience = format!("{}//{}", url.protocol(), url.host());

    // Have to create manually to avoid error in default wasm duration implementation
    let now = Duration::new((js_sys::Date::now() / 1000.) as u64, 0);
    let claims = JWTClaims {
        issued_at: Some(now),
        expires_at: Some(now + Duration::from_hours(2)),
        invalid_before: Some(now),
        audiences: None,
        issuer: None,
        jwt_id: None,
        subject: None,
        nonce: None,
        custom: NoCustomClaims {},
    }
    .with_audience(audience)
    .with_subject(sub);

    let private_key = option_env!("VAPID_PRIVATE_KEY").unwrap_or("");
    let key_pair = ES256KeyPair::from_bytes(
        &base64::decode(private_key).map_err(|_| "decode failed".to_string())?,
    )
    .map_err(|_| "pair failed".to_string())?;

    key_pair.sign(claims).map_err(|_| "".to_string())
}

pub async fn push_notification(endpoint: String) -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut opts = RequestInit::new();
    opts.method("POST");

    let request = Request::new_with_str_and_init(&endpoint, &opts)?;

    let token = create_vapid_jwt(
        endpoint,
        option_env!("VAPID_SUBJECT").unwrap_or("").to_string(),
    )?;
    request
        .headers()
        .set("Authorization", &format!("WebPush {}", token))?;
    request.headers().set(
        "Crypto-Key",
        &format!(
            "p256ecdsa={}",
            option_env!("VAPID_PUBLIC_KEY").unwrap_or("")
        ),
    )?;
    request.headers().set("TTL", "0")?;

    let global = worker_global_scope().ok_or(JsValue::NULL)?;
    let resp_value = JsFuture::from(global.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into()?;
    if resp.ok() {
        Ok(())
    } else {
        let res_text_val = JsFuture::from(resp.text()?).await?;
        let res_text: &JsString = res_text_val.dyn_ref().ok_or(JsValue::NULL)?;
        Err(JsValue::from(res_text))
    }
}
