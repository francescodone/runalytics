//! Cross-platform access to the OS credential store.
//!
//! `keyring::Entry::new` only ever succeeds on macOS: the crate installs its
//! default store for macOS, Windows and *nix, and on iOS it returns
//! `NoDefaultStore` for every call, so a token saved there is simply never
//! written and never found. iOS has to address the protected-data store
//! explicitly. Both backends speak the same `keyring_core::Entry` API, so
//! callers get one type and one error type either way.

use keyring_core::{Entry, Error};

/// Service and account for the one GitHub token the app uses.
///
/// Both features that talk to GitHub — publishing the calendar to a gist and
/// the cross-device plan channel — authenticate as the same person, so they
/// share one entry rather than asking for the same token twice. Keeping it in
/// its own service (rather than inside either feature's) also means turning one
/// feature off does not silently take the other one's credential with it.
pub const GITHUB_SERVICE: &str = "com.fradone.runalytics.github";
pub const GITHUB_TOKEN_USER: &str = "token";

/// Service and account the plan sync token used before it moved into the shared
/// entry. Kept here so both the migration in `plan_sync` and the clearing in
/// `calendar_publish` agree on where the old copy was.
pub const LEGACY_PLAN_SERVICE: &str = "com.fradone.runalytics.plan";
pub const LEGACY_PLAN_TOKEN_USER: &str = "github_token";

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
