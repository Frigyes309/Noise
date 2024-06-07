use noise_protocol::DH;

struct Dh25519 {
    privkey: [u8; 32],
    pubkey:  [u8; 32],
}

impl Dh25519 {
    fn derive_pubkey(&mut self) {
        let point = MontgomeryPoint::mul_base_clamped(self.privkey);
        self.pubkey = point.to_bytes();
    }
}

impl Dh for Dh25519 {
    fn name(&self) -> &'static str {
        "25519"
    }

    fn pub_len(&self) -> usize {
        32
    }

    fn priv_len(&self) -> usize {
        32
    }

    fn set(&mut self, privkey: &[u8]) {
        let mut bytes = [0u8; CIPHERKEYLEN];
        copy_slices!(privkey, bytes);
        self.privkey = bytes;
        self.derive_pubkey();
    }

    fn generate(&mut self, rng: &mut dyn Random) {
        let mut bytes = [0u8; CIPHERKEYLEN];
        rng.fill_bytes(&mut bytes);
        self.privkey = bytes;
        self.derive_pubkey();
    }

    fn pubkey(&self) -> &[u8] {
        &self.pubkey
    }

    fn privkey(&self) -> &[u8] {
        &self.privkey
    }

    fn dh(&self, pubkey: &[u8], out: &mut [u8]) -> Result<(), Error> {
        let mut pubkey_owned = [0u8; CIPHERKEYLEN];
        copy_slices!(&pubkey[..32], pubkey_owned);
        let result = MontgomeryPoint(pubkey_owned).mul_clamped(self.privkey).to_bytes();
        copy_slices!(result, out);
        Ok(())
    }
}