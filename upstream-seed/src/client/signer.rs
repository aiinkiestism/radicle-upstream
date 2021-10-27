use librad::crypto::keystore::sign::ed25519;
use librad::SecretKey;

#[derive(Clone)]
pub struct Signer {
    pub(super) key: SecretKey,
}

impl Signer {
    pub fn new(key: librad::SecretKey) -> Self {
        Self { key }
    }
}

#[async_trait::async_trait]
impl ed25519::Signer for Signer {
    type Error = std::convert::Infallible;

    fn public_key(&self) -> ed25519::PublicKey {
        self.key.public_key()
    }

    async fn sign(&self, data: &[u8]) -> Result<ed25519::Signature, Self::Error> {
        <SecretKey as ed25519::Signer>::sign(&self.key, data).await
    }
}
