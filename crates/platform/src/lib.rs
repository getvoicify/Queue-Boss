#[derive(Debug, PartialEq, Eq)]
pub enum SecretStoreError {
    Unavailable,
}

pub trait SecretStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
}

pub struct NoopSecretStore;

impl SecretStore for NoopSecretStore {
    fn get(&self, _key: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(None)
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
}
