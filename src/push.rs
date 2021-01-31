use std::any::Any;

use js_sys::{self, JsString, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, Url};

use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead};
use aes_gcm::Aes128Gcm;
use base64;
use ece::{
    crypto::set_boxed_cryptographer, encrypt, Cryptographer, EcKeyComponents, Error as EceError,
    LocalKeyPair, RemotePublicKey, Result as EceResult,
};
use hkdf::Hkdf;
use jwt_simple::prelude::*;
use p256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand::RngCore;
use sha2::Sha256;

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

pub struct RemoteKey(pub PublicKey);

impl RemoteKey {
    pub fn from_raw(key_bytes: &[u8]) -> EceResult<Self> {
        let key = PublicKey::from_sec1_bytes(key_bytes).map_err(|_| EceError::CryptoError)?;
        Ok(RemoteKey(key))
    }

    pub fn from_string(key_str: String) -> Result<Self, String> {
        let key = PublicKey::from_sec1_bytes(
            &base64::decode_config(key_str.as_bytes(), base64::URL_SAFE_NO_PAD)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(RemoteKey(key))
    }
}

impl RemotePublicKey for RemoteKey {
    fn as_raw(&self) -> EceResult<Vec<u8>> {
        Ok(EncodedPoint::from(self.0).as_bytes().to_vec())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct JWTLocalKeyPair(pub EphemeralSecret);

impl LocalKeyPair for JWTLocalKeyPair {
    fn pub_as_raw(&self) -> EceResult<Vec<u8>> {
        Ok(EncodedPoint::from(self.0.public_key()).as_bytes().to_vec())
    }
    fn raw_components(&self) -> EceResult<EcKeyComponents> {
        Err(EceError::CryptoError)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

struct CryptoHandler;

impl Cryptographer for CryptoHandler {
    fn generate_ephemeral_keypair(&self) -> EceResult<Box<dyn LocalKeyPair>> {
        let rng = rand::thread_rng();
        let secret = EphemeralSecret::random(rng);
        Ok(Box::new(JWTLocalKeyPair(secret)))
    }

    fn random_bytes(&self, dest: &mut [u8]) -> EceResult<()> {
        rand::thread_rng().fill_bytes(dest);
        Ok(())
    }

    fn compute_ecdh_secret(
        &self,
        remote: &dyn RemotePublicKey,
        local: &dyn LocalKeyPair,
    ) -> EceResult<Vec<u8>> {
        let local_any = local.as_any();
        let local = local_any.downcast_ref::<JWTLocalKeyPair>().unwrap();
        let remote_any = remote.as_any();
        let remote = remote_any.downcast_ref::<RemoteKey>().unwrap();
        let shared = local.0.diffie_hellman(&remote.0);
        Ok(shared.as_bytes().to_vec())
    }

    fn hkdf_sha256(
        &self,
        salt: &[u8],
        secret: &[u8],
        info: &[u8],
        len: usize,
    ) -> EceResult<Vec<u8>> {
        let (_, hk) = Hkdf::<Sha256>::extract(Some(&salt[..]), &secret);
        let mut okm = vec![0u8; len];
        hk.expand(&info, &mut okm).unwrap();
        Ok(okm)
    }

    fn aes_gcm_128_encrypt(&self, key: &[u8], iv: &[u8], data: &[u8]) -> EceResult<Vec<u8>> {
        let cipher = Aes128Gcm::new(GenericArray::from_slice(key));
        let nonce = GenericArray::from_slice(iv);
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|_| EceError::CryptoError)?;
        Ok(ciphertext)
    }

    fn import_key_pair(&self, _components: &EcKeyComponents) -> EceResult<Box<dyn LocalKeyPair>> {
        Err(EceError::CryptoError)
    }
    fn import_public_key(&self, raw: &[u8]) -> EceResult<Box<dyn RemotePublicKey>> {
        Ok(Box::new(RemoteKey::from_raw(raw)?))
    }
    fn aes_gcm_128_decrypt(
        &self,
        _key: &[u8],
        _iv: &[u8],
        _ciphertext_and_tag: &[u8],
    ) -> EceResult<Vec<u8>> {
        Err(EceError::CryptoError)
    }
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
    // TODO: Longer?
    request.headers().set("TTL", "0")?;
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
