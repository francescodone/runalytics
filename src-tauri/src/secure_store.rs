//! Cross-platform access to the OS credential store.
//!
//! `keyring::Entry::new` only ever succeeds on macOS: the crate installs its
//! default store for macOS, Windows and *nix, and on iOS it returns
//! `NoDefaultStore` for every call, so a token saved there is simply never
//! written and never found. iOS has to address the protected-data store
//! explicitly. Both backends speak the same `keyring_core::Entry` API, so
//! callers get one type and one error type either way.

use keyring_core::{Entry, Error};

/// An entry for `service`/`user` in the platform's secure store.
pub fn entry(service: &str, user: &str) -> Result<Entry, Error> {
    #[cfg(target_os = "ios")]
    {
        use apple_native_keyring_store::protected::Store;
        use keyring_core::api::CredentialStoreApi as _;

        Store::new()?.build(service, user, None)
    }
    #[cfg(not(target_os = "ios"))]
    {
        keyring::Entry::new(service, user).map(|e| e.inner)
    }
}
