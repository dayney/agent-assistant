use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
