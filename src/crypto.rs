use std::any::Any;

use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead};
use aes_gcm::Aes128Gcm;
use base64;
use ece::{
    Cryptographer, EcKeyComponents, Error as EceError, LocalKeyPair, RemotePublicKey,
    Result as EceResult,
};
use hkdf::Hkdf;
use p256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand::RngCore;
use sha2::Sha256;

/// Implements Cryptographer from `ece` crate in a format compatible with WASM

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

pub struct CryptoHandler;

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
