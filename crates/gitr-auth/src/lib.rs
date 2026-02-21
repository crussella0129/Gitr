use gitr_core::error::GitrError;

/// Trait for credential storage backends.
pub trait CredentialStore: Send + Sync {
    /// Store a token under the given key.
    fn store(&self, key: &str, token: &str) -> Result<(), GitrError>;

    /// Retrieve a token by key.
    fn get(&self, key: &str) -> Result<Option<String>, GitrError>;

    /// Delete a stored token.
    fn delete(&self, key: &str) -> Result<(), GitrError>;
}

/// OS keychain-backed credential store using the `keyring` crate.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new() -> Self {
        Self {
            service: "gitr".to_string(),
        }
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for KeyringStore {
    fn store(&self, key: &str, token: &str) -> Result<(), GitrError> {
        let entry = keyring::Entry::new(&self.service, key).map_err(|e| {
            GitrError::CredentialError {
                message: e.to_string(),
            }
        })?;
        entry
            .set_password(token)
            .map_err(|e| GitrError::CredentialError {
                message: e.to_string(),
            })
    }

    fn get(&self, key: &str) -> Result<Option<String>, GitrError> {
        let entry = keyring::Entry::new(&self.service, key).map_err(|e| {
            GitrError::CredentialError {
                message: e.to_string(),
            }
        })?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(GitrError::CredentialError {
                message: e.to_string(),
            }),
        }
    }

    fn delete(&self, key: &str) -> Result<(), GitrError> {
        let entry = keyring::Entry::new(&self.service, key).map_err(|e| {
            GitrError::CredentialError {
                message: e.to_string(),
            }
        })?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(GitrError::CredentialError {
                message: e.to_string(),
            }),
        }
    }
}

/// In-memory credential store for testing.
pub struct MemoryStore {
    store: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for MemoryStore {
    fn store(&self, key: &str, token: &str) -> Result<(), GitrError> {
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), token.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, GitrError> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), GitrError> {
        self.store.lock().unwrap().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store_crud() {
        let store = MemoryStore::new();
        assert_eq!(store.get("test-key").unwrap(), None);
        store.store("test-key", "secret-token").unwrap();
        assert_eq!(store.get("test-key").unwrap(), Some("secret-token".to_string()));
        store.delete("test-key").unwrap();
        assert_eq!(store.get("test-key").unwrap(), None);
    }

    #[test]
    fn test_memory_store_delete_nonexistent() {
        let store = MemoryStore::new();
        store.delete("no-such-key").unwrap();
    }
}
