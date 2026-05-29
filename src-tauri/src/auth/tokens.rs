use super::error::Result;
use super::pkce::TokenSet;
use std::sync::Mutex;

/// Persists access + refresh tokens. The default implementation uses the OS
/// keyring; tests use the in-memory backing exposed via `TokenStore::memory()`.
pub trait TokenBackend: Send + Sync {
    fn save(&self, value: &str) -> Result<()>;
    fn load(&self) -> Result<Option<String>>;
    fn clear(&self) -> Result<()>;
}

pub struct TokenStore {
    backend: Box<dyn TokenBackend>,
}

impl TokenStore {
    pub fn new(backend: Box<dyn TokenBackend>) -> Self {
        Self { backend }
    }

    pub fn os_keyring(service: &'static str, user: &'static str) -> Self {
        Self {
            backend: Box::new(KeyringBackend { service, user }),
        }
    }

    pub fn memory() -> Self {
        Self {
            backend: Box::new(MemoryBackend::default()),
        }
    }

    pub fn save(&self, t: &TokenSet) -> Result<()> {
        let serialized = serde_json::to_string(t).expect("TokenSet always serializes");
        self.backend.save(&serialized)
    }

    pub fn load(&self) -> Result<Option<TokenSet>> {
        let Some(raw) = self.backend.load()? else {
            return Ok(None);
        };
        let parsed: TokenSet = serde_json::from_str(&raw)
            .map_err(|_| super::error::AuthError::MissingField("tokens"))?;
        Ok(Some(parsed))
    }

    pub fn clear(&self) -> Result<()> {
        self.backend.clear()
    }
}

pub struct KeyringBackend {
    service: &'static str,
    user: &'static str,
}

impl TokenBackend for KeyringBackend {
    fn save(&self, value: &str) -> Result<()> {
        let entry = keyring_core::Entry::new(self.service, self.user)?;
        entry.set_password(value)?;
        Ok(())
    }

    fn load(&self) -> Result<Option<String>> {
        let entry = keyring_core::Entry::new(self.service, self.user)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn clear(&self) -> Result<()> {
        let entry = keyring_core::Entry::new(self.service, self.user)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default)]
pub struct MemoryBackend {
    inner: Mutex<Option<String>>,
}

impl TokenBackend for MemoryBackend {
    fn save(&self, value: &str) -> Result<()> {
        *self.inner.lock().unwrap() = Some(value.to_string());
        Ok(())
    }

    fn load(&self) -> Result<Option<String>> {
        Ok(self.inner.lock().unwrap().clone())
    }

    fn clear(&self) -> Result<()> {
        *self.inner.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> TokenSet {
        TokenSet {
            access_token: "AT".into(),
            refresh_token: "RT".into(),
            expires_in: 3600,
            token_type: "Bearer".into(),
            scope: "user-library-read".into(),
        }
    }

    #[test]
    fn memory_store_round_trips() {
        let s = TokenStore::memory();
        assert!(s.load().unwrap().is_none());
        s.save(&token()).unwrap();
        assert_eq!(s.load().unwrap().unwrap(), token());
        s.clear().unwrap();
        assert!(s.load().unwrap().is_none());
    }

    #[test]
    fn corrupted_payload_returns_error() {
        let s = TokenStore::memory();
        s.backend.save("not json").unwrap();
        assert!(s.load().is_err());
    }

    #[test]
    fn clear_when_empty_is_noop() {
        let s = TokenStore::memory();
        s.clear().unwrap();
        s.clear().unwrap();
    }
}
