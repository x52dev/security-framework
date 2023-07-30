//! OSX specific functionality for keys.
use std::ptr;

use core_foundation::{
    base::TCFType, data::CFData, dictionary::CFDictionary, error::CFError, string::CFString,
};
use security_framework_sys::{item::kSecAttrKeyType, key::SecKeyCreateFromData};

use crate::key::{KeyType, SecKey};

/// An extension trait adding OSX specific functionality to `SecKey`.
pub trait SecKeyExt {
    /// Creates a new `SecKey` from a buffer containing key data.
    fn from_data(key_type: KeyType, key_data: &CFData) -> Result<SecKey, CFError>;
}

impl SecKeyExt for SecKey {
    fn from_data(key_type: KeyType, key_data: &CFData) -> Result<Self, CFError> {
        unsafe {
            let key = CFString::wrap_under_get_rule(kSecAttrKeyType);
            let dict = CFDictionary::from_CFType_pairs(&[(key, key_type.to_str())]);

            let mut err = ptr::null_mut();
            let key = SecKeyCreateFromData(
                dict.as_concrete_TypeRef(),
                key_data.as_concrete_TypeRef(),
                &mut err,
            );
            if key.is_null() {
                Err(CFError::wrap_under_create_rule(err))
            } else {
                Ok(Self::wrap_under_create_rule(key))
            }
        }
    }
}
