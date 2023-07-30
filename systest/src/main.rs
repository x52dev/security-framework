#![allow(bad_style)]
#![allow(unused)]
#![allow(clippy::all)]
#![allow(deprecated)]
#![allow(deref_nullptr)]
#![allow(invalid_value)] // mem::uninitialized has to stay

use std::os::raw::*;

use core_foundation_sys::{
    base::{CFOptionFlags, OSStatus},
    string::CFStringRef,
};
#[cfg(target_os = "macos")]
use security_framework_sys::access::*;
#[cfg(target_os = "macos")]
use security_framework_sys::certificate_oids::*;
#[cfg(target_os = "macos")]
use security_framework_sys::code_signing::*;
#[cfg(target_os = "macos")]
use security_framework_sys::digest_transform::*;
#[cfg(target_os = "macos")]
use security_framework_sys::encrypt_transform::*;
#[cfg(target_os = "macos")]
use security_framework_sys::keychain::*;
#[cfg(target_os = "macos")]
use security_framework_sys::keychain_item::*;
#[cfg(target_os = "macos")]
use security_framework_sys::transform::*;
use security_framework_sys::{
    access_control::*, authorization::*, base::*, certificate::*, cipher_suite::*, identity::*,
    import_export::*, item::*, key::*, policy::*, random::*, secure_transport::*, trust::*,
    trust_settings::*,
};

include!(concat!(env!("OUT_DIR"), "/all.rs"));
