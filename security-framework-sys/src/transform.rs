use core_foundation_sys::{
    base::{Boolean, CFTypeID, CFTypeRef},
    error::CFErrorRef,
    string::CFStringRef,
};

pub type SecTransformRef = CFTypeRef;

extern "C" {
    pub static kSecTransformInputAttributeName: CFStringRef;

    pub fn SecTransformGetTypeID() -> CFTypeID;

    pub fn SecTransformSetAttribute(
        transformRef: SecTransformRef,
        key: CFStringRef,
        value: CFTypeRef,
        error: *mut CFErrorRef,
    ) -> Boolean;

    pub fn SecTransformExecute(
        transformRef: SecTransformRef,
        errorRef: *mut CFErrorRef,
    ) -> CFTypeRef;
}
