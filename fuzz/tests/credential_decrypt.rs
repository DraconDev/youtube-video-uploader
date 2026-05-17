use proptest::prelude::*;
use video_uploader::config::CredentialStore;

proptest! {
    #[test]
    fn credential_decrypt_fuzz(passphrase: String, ciphertext: Vec<u8>) {
        let _ = CredentialStore::decrypt_store_for_testing(&passphrase, &ciphertext);
    }
}
