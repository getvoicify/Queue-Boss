use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, PartialEq, Eq)]
pub enum SecretStoreError {
    Unavailable,
    Backend,
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
}

pub struct NoopSecretStore;

impl SecretStore for NoopSecretStore {
    fn get(&self, _key: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(None)
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }

    fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }
}

pub struct InMemorySecretStore {
    entries: Mutex<HashMap<String, String>>,
}

impl InMemorySecretStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for InMemorySecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self
            .entries
            .lock()
            .expect("secret store mutex poisoned")
            .get(key)
            .cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        self.entries
            .lock()
            .expect("secret store mutex poisoned")
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        self.entries
            .lock()
            .expect("secret store mutex poisoned")
            .remove(key);
        Ok(())
    }
}

fn map_get(result: Result<String, keyring::Error>) -> Result<Option<String>, SecretStoreError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(SecretStoreError::Backend),
    }
}

fn map_delete(result: Result<(), keyring::Error>) -> Result<(), SecretStoreError> {
    match result {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(SecretStoreError::Backend),
    }
}

pub struct OsSecretStore {
    service: String,
}

impl OsSecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(&self.service, key).map_err(|_| SecretStoreError::Backend)
    }
}

impl SecretStore for OsSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        map_get(self.entry(key)?.get_password())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|_| SecretStoreError::Backend)
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        map_delete(self.entry(key)?.delete_credential())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_secret_store_get_returns_none() {
        let store = NoopSecretStore;
        assert_eq!(store.get("api_key"), Ok(None));
    }

    #[test]
    fn noop_secret_store_set_and_delete_are_no_ops() {
        let store = NoopSecretStore;
        assert_eq!(store.set("api_key", "value"), Ok(()));
        assert_eq!(store.delete("api_key"), Ok(()));
    }

    #[test]
    fn in_memory_set_then_get_round_trips() {
        let store = InMemorySecretStore::new();
        store.set("api_key", "s3cr3t").expect("set must succeed");
        assert_eq!(store.get("api_key"), Ok(Some("s3cr3t".to_owned())));
    }

    #[test]
    fn in_memory_get_absent_key_is_none() {
        let store = InMemorySecretStore::new();
        assert_eq!(store.get("absent"), Ok(None));
    }

    #[test]
    fn in_memory_delete_then_get_is_none() {
        let store = InMemorySecretStore::new();
        store.set("api_key", "s3cr3t").expect("set must succeed");
        store.delete("api_key").expect("delete must succeed");
        assert_eq!(store.get("api_key"), Ok(None));
    }

    #[test]
    fn in_memory_delete_absent_key_is_ok() {
        let store = InMemorySecretStore::new();
        assert_eq!(store.delete("absent"), Ok(()));
    }

    #[test]
    fn map_get_present_value_is_some() {
        assert_eq!(
            map_get(Ok("s3cr3t".to_owned())),
            Ok(Some("s3cr3t".to_owned()))
        );
    }

    #[test]
    fn map_get_no_entry_is_none() {
        assert_eq!(map_get(Err(keyring::Error::NoEntry)), Ok(None));
    }

    #[test]
    fn map_get_other_error_is_backend() {
        assert_eq!(
            map_get(Err(keyring::Error::Invalid(
                "attr".to_owned(),
                "reason".to_owned()
            ))),
            Err(SecretStoreError::Backend)
        );
    }

    #[test]
    fn map_delete_ok_is_ok() {
        assert_eq!(map_delete(Ok(())), Ok(()));
    }

    #[test]
    fn map_delete_no_entry_is_ok() {
        assert_eq!(map_delete(Err(keyring::Error::NoEntry)), Ok(()));
    }

    #[test]
    fn map_delete_other_error_is_backend() {
        assert_eq!(
            map_delete(Err(keyring::Error::Invalid(
                "attr".to_owned(),
                "reason".to_owned()
            ))),
            Err(SecretStoreError::Backend)
        );
    }

    #[test]
    #[ignore = "requires a real OS keychain / Secret Service"]
    fn os_secret_store_roundtrips_on_a_real_keychain() {
        let store = OsSecretStore::new("queue-boss-e2e3-roundtrip-test");
        let key = "e2e3-roundtrip-key";
        store.set(key, "s3cr3t").expect("set must succeed");
        assert_eq!(store.get(key), Ok(Some("s3cr3t".to_owned())));
        store.delete(key).expect("delete must succeed");
        assert_eq!(store.get(key), Ok(None));
    }
}
