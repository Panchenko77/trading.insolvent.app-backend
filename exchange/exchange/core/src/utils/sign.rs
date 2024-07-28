use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::Hmac;
use hmac::Mac;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use sha2::{Sha256, Sha512};

macro_rules! hmac_sha256 {
    ($s: expr, $pri_key: expr) => {{
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice($pri_key.as_bytes()).unwrap();
        mac.update($s);
        let result = mac.finalize();
        let result = result.into_bytes();
        result
    }};
}
macro_rules! hmac_sha512 {
    ($s: expr, $pri_key: expr) => {{
        type HmacSha512 = Hmac<Sha512>;
        let mut mac = HmacSha512::new_from_slice($pri_key.as_bytes()).unwrap();
        mac.update($s);
        let result = mac.finalize();
        let result = result.into_bytes();
        result
    }};
}
pub fn sign_hmac_sha256_hex(s: impl AsRef<[u8]>, pri_key: &str) -> String {
    hex::encode(hmac_sha256!(s.as_ref(), pri_key))
}
pub fn sign_hmac_sha512_hex(s: impl AsRef<[u8]>, pri_key: &str) -> String {
    hex::encode(hmac_sha512!(s.as_ref(), pri_key))
}
pub fn sign_hmac_sha256_base64(s: impl AsRef<[u8]>, pri_key: &str) -> String {
    STANDARD.encode(hmac_sha256!(s.as_ref(), pri_key))
}
pub fn hash_sha512_hex(s: impl AsRef<[u8]>) -> String {
    use sha2::Digest;
    let mut hasher = Sha512::default();
    hasher.update(s);
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn percent_encode(source: &str) -> String {
    const FRAGMENT: &AsciiSet = &CONTROLS.add(b'+').add(b',');
    let signature = utf8_percent_encode(&source, FRAGMENT).to_string();
    signature
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn hmac_sha256() {
        let b = b"symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
        let key = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        assert_eq!(
            &sign_hmac_sha256_hex(b, key),
            "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71"
        )
    }
}
