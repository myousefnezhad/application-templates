use rsa::{
    Error as RsaError, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey, Error as Pkcs1Error},
};
use thiserror::Error;
use getrandom::SysRng;
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid RSA key: {0}")]
    InvalidKey(#[from] Pkcs1Error),

    #[error("RSA operation failed: {0}")]
    Rsa(#[from] RsaError),
}

pub fn encrypt(data: &[u8], public_key_pem: &str) -> Result<Vec<u8>, CryptoError> {
    let public_key = RsaPublicKey::from_pkcs1_pem(public_key_pem)?;

    // Seeds ChaCha20Rng securely from the operating system.
    let mut sys_rng = SysRng;
    let mut rng = ChaCha20Rng::try_from_rng(&mut sys_rng).expect("failed to seed ChaCha20Rng");

    let encrypted = public_key.encrypt(&mut rng, Pkcs1v15Encrypt, data)?;

    Ok(encrypted)
}

pub fn decrypt(encrypted_data: &[u8], private_key_pem: &str) -> Result<Vec<u8>, CryptoError> {
    let private_key = RsaPrivateKey::from_pkcs1_pem(private_key_pem)?;

    let decrypted = private_key.decrypt(Pkcs1v15Encrypt, encrypted_data)?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRIVATE_KEY: &str = include_str!("../../../../data/rsa_example_keys/private.pem");
    const PUBLIC_KEY: &str = include_str!("../../../../data/rsa_example_keys/public.pem");

    #[test]
    fn check_rsa_functions() -> Result<(), CryptoError> {
        let data = b"hello world";

        let encrypted = encrypt(data, PUBLIC_KEY)?;
        let decrypted = decrypt(&encrypted, PRIVATE_KEY)?;

        assert_ne!(data.as_slice(), encrypted.as_slice());
        assert_eq!(data.as_slice(), decrypted.as_slice());

        Ok(())
    }
}
