#![allow(
    bad_style,
    unused,
    clippy::all,
    deprecated,
    invalid_value, // mem::uninitialized has to stay
    deref_nullptr,
)]

extern crate apple_security_framework_sys as security_framework_sys;

use std::os::raw::*;

use core_foundation_sys::{
    base::{CFOptionFlags, OSStatus},
    string::CFStringRef,
};
#[cfg(target_os = "macos")]
use security_framework_sys::{
    access::*, certificate_oids::*, code_signing::*, digest_transform::*, encrypt_transform::*,
    keychain::*, keychain_item::*, transform::*,
};
use security_framework_sys::{
    access_control::*, authorization::*, base::*, certificate::*, cipher_suite::*, identity::*,
    import_export::*, item::*, key::*, policy::*, random::*, secure_transport::*, trust::*,
    trust_settings::*,
};

include!(concat!(env!("OUT_DIR"), "/all.rs"));
