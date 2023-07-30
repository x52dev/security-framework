//! OSX specific functionality for items.
// Moved to crate::Key
pub use crate::key::KeyType;
use crate::{
    item::ItemSearchOptions, os::macos::keychain::SecKeychain, ItemSearchOptionsInternals,
};

/// An extension trait adding OSX specific functionality to `ItemSearchOptions`.
pub trait ItemSearchOptionsExt {
    /// Search within the specified keychains.
    ///
    /// If this is not called, the default keychain will be searched.
    fn keychains(&mut self, keychains: &[SecKeychain]) -> &mut Self;
}

impl ItemSearchOptionsExt for ItemSearchOptions {
    #[inline(always)]
    fn keychains(&mut self, keychains: &[SecKeychain]) -> &mut Self {
        ItemSearchOptionsInternals::keychains(self, keychains)
    }
}

#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use crate::{
        item::*,
        os::macos::{certificate::SecCertificateExt, item::ItemSearchOptionsExt, test::keychain},
    };

    #[test]
    fn find_certificate() {
        let dir = p!(tempdir());
        let keychain = keychain(dir.path());
        let results = p!(ItemSearchOptions::new()
            .keychains(&[keychain])
            .class(ItemClass::certificate())
            .search());
        assert_eq!(1, results.len());
        let certificate = match results[0] {
            SearchResult::Ref(Reference::Certificate(ref cert)) => cert,
            _ => panic!("expected certificate"),
        };
        assert_eq!("foobar.com", p!(certificate.common_name()));
    }
}
