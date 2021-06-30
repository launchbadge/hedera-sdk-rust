use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::hash::Hasher;
use std::str;
use std::str::FromStr;

use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer, SECRET_KEY_LENGTH, SIGNATURE_LENGTH};
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};

use crate::key_error::KeyError;

const DER_PREFIX: &str = "302e020100300506032b657004220420";
const DER_PREFIX_BYTES: Lazy<Vec<u8>> = Lazy::new(|| hex::decode(DER_PREFIX).unwrap());

/// A private key on the Hedera™ Network
#[derive(Debug)]
pub struct PrivateKey {
    pub(crate) keypair: Keypair,
    pub(crate) chain_code: Option<[u8; 32]>,
}

pub(crate) fn to_keypair(entropy: &[u8]) -> Result<Keypair, KeyError> {
    let secret = SecretKey::from_bytes(&entropy[0..32]).map_err(KeyError::Signature)?;

    Ok(Keypair {
        public: PublicKey::from(&secret),
        secret,
    })
}

impl PrivateKey {
    pub fn generate() -> Self {
        let mut entropy = [0u8; 64];
        thread_rng().fill(&mut entropy[..]);

        Self {
            keypair: to_keypair(&entropy[0..32]).unwrap(),
            chain_code: Some(<[u8; 32]>::try_from(&entropy[32..64]).unwrap()),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, KeyError> {
        match data.len() {
            32 => Ok(Self {
                keypair: to_keypair(&data)?,
                chain_code: None,
            }),

            48 if data.starts_with(&DER_PREFIX_BYTES) => Ok(Self {
                keypair: to_keypair(&data[16..])?,
                chain_code: None,
            }),

            64 => Ok(Self {
                keypair: to_keypair(&data[..SECRET_KEY_LENGTH])?,
                chain_code: None,
            }),

            _ => Err(KeyError::Length(data.len())),
        }
    }

    pub fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.keypair.secret.to_bytes()
    }

    /// Sign a message with this private key.
    pub fn sign(&self, data: &[u8]) -> [u8; SIGNATURE_LENGTH] {
        self.keypair.sign(data).to_bytes()
    }

    /// Get the public key associated with this private key.
    ///
    /// The public key can be freely given and used by other parties
    /// to verify the signatures generated by this private key.
    ///
    pub fn public_key(&self) -> crate::PublicKey {
        crate::PublicKey(self.keypair.public)
    }
}

impl Hash for PrivateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.keypair.secret.as_bytes().hash(state)
    }
}

impl PartialEq for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.keypair.secret.as_bytes() == other.keypair.secret.as_bytes()
    }
}

impl Eq for PrivateKey {}

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.keypair.secret.as_bytes()
    }
}

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", DER_PREFIX, hex::encode(self))
    }
}

impl FromStr for PrivateKey {
    type Err = KeyError;

    fn from_str(text: &str) -> Result<Self, KeyError> {
        Ok(PrivateKey::from_bytes(&hex::decode(&text)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyError, PrivateKey};
    use ed25519_dalek::{Signature, Signer, SIGNATURE_LENGTH};
    use rand::{thread_rng, Rng};
    use std::str::FromStr;

    const PRIVATE_KEY_STR: &str = "302e020100300506032b6570042204204072d365d02199b5103336cf6a187578ffb6eba4ad6f8b2383c5cc54d00c4409";
    const PRIVATE_KEY_BYTES: &[u8] = &[
        64, 114, 211, 101, 208, 33, 153, 181, 16, 51, 54, 207, 106, 24, 117, 120, 255, 182, 235,
        164, 173, 111, 139, 35, 131, 197, 204, 84, 208, 12, 68, 9,
    ];

    #[test]
    fn test_generate() -> Result<(), KeyError> {
        let private_key = PrivateKey::generate();

        assert_eq!(private_key.keypair.secret.to_bytes().len(), 32 as usize);

        Ok(())
    }

    #[test]
    fn test_from_str() -> Result<(), KeyError> {
        let key = PrivateKey::from_str(&PRIVATE_KEY_STR)?;

        assert_eq!(&key.to_bytes(), PRIVATE_KEY_BYTES);

        Ok(())
    }

    #[test]
    fn test_to_bytes() -> Result<(), KeyError> {
        let private_key = PrivateKey::from_str(PRIVATE_KEY_STR)?;
        assert_eq!(
            &PrivateKey::to_bytes(&private_key),
            &private_key.keypair.secret.to_bytes()
        );

        Ok(())
    }

    #[test]
    fn test_public_key() -> Result<(), KeyError> {
        let private_key = PrivateKey::from_str(PRIVATE_KEY_STR)?;

        assert_eq!(
            PrivateKey::public_key(&private_key).0,
            private_key.keypair.public
        );

        Ok(())
    }

    #[test]
    fn test_to_from_string() -> Result<(), KeyError> {
        assert_eq!(
            PrivateKey::from_str(PRIVATE_KEY_STR)?.to_string(),
            PRIVATE_KEY_STR
        );

        Ok(())
    }

    #[test]
    fn test_sign() -> Result<(), KeyError> {
        let mut entropy = [0u8; 64];
        thread_rng().fill(&mut entropy[..]);
        let key = PrivateKey::from_bytes(&entropy[..32])?;
        let message: &[u8] = b"This is a test";
        let signature: Signature = key.keypair.sign(message);
        let signature_bytes: [u8; SIGNATURE_LENGTH] = signature.to_bytes();

        assert_eq!(PrivateKey::sign(&key, message), signature_bytes);

        Ok(())
    }
}
