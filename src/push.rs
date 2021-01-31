use js_sys::{self, JsString, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, Url};

use base64;
use ece::{crypto::set_boxed_cryptographer, encrypt};
use jwt_simple::prelude::*;
use rand::RngCore;

use crate::crypto::CryptoHandler;
use crate::utils::worker_global_scope;
use crate::Subscription;

pub fn create_vapid_jwt(endpoint: String, sub: String) -> Result<String, String> {
    let url = Url::new(&endpoint).map_err(|_| "Error parsing URL".to_string())?;
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
        &base64::decode_config(private_key, base64::URL_SAFE_NO_PAD).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    key_pair.sign(claims).map_err(|e| e.to_string())
}

fn create_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

fn encrypt_message(p256dh: String, auth: String, message: String) -> Result<Vec<u8>, String> {
    let _ = set_boxed_cryptographer(Box::new(CryptoHandler {}));
    let salt = create_salt();

    let encrypted = encrypt(
        &base64::decode_config(p256dh.as_bytes(), base64::URL_SAFE_NO_PAD)
            .map_err(|e| e.to_string())?,
        &base64::decode_config(auth.as_bytes(), base64::URL_SAFE_NO_PAD)
            .map_err(|e| e.to_string())?,
        &salt,
        message.as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    Ok(encrypted)
}

pub async fn push_notification(subscription: Subscription) -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut opts = RequestInit::new();
    opts.method("POST");

    // Encrypt diff URL to be sent
    let message = encrypt_message(subscription.p256dh, subscription.auth, subscription.diff)?;
    let message_arr = Uint8Array::from(message.as_slice());
    let message_buff = message_arr.buffer();
    let content_length = message_buff.byte_length();

    opts.body(Some(&message_buff));

    let request = Request::new_with_str_and_init(&subscription.endpoint, &opts)?;

    let token = create_vapid_jwt(
        subscription.endpoint,
        option_env!("VAPID_SUBJECT").unwrap_or("").to_string(),
    )?;

    request.headers().set(
        "Authorization",
        &format!(
            "vapid t={}, k={}",
            token,
            option_env!("VAPID_PUBLIC_KEY").unwrap_or(""),
        ),
    )?;
    request.headers().set("TTL", "86400")?; // 24 hours
    request.headers().set("Content-Encoding", "aes128gcm")?;
    request
        .headers()
        .set("Content-Type", "application/octet-stream")?;
    request
        .headers()
        .set("Content-Length", &content_length.to_string())?;

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
