//! Keychain item support.

use std::fmt;

use core_foundation::base::TCFType;
use security_framework_sys::{base::SecKeychainItemRef, keychain_item::SecKeychainItemGetTypeID};

declare_TCFType! {
    /// A type representing a keychain item.
    SecKeychainItem, SecKeychainItemRef
}
impl_TCFType!(
    SecKeychainItem,
    SecKeychainItemRef,
    SecKeychainItemGetTypeID
);

unsafe impl Sync for SecKeychainItem {}
unsafe impl Send for SecKeychainItem {}

impl fmt::Debug for SecKeychainItem {
    #[cold]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SecKeychainItem").finish_non_exhaustive()
    }
}
