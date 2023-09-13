fn encrypt_basic_af(plain_text: &str, key: &str) -> String {
    String::from_utf8(
        plain_text
            .as_bytes()
            .iter()
            .zip(key.as_bytes().iter().cycle())
            .map(|(&pl, &kl)| pl ^ kl)
            .collect(),
    )
    .unwrap()
}
fn encrypt_basic(plain_text: &str, key: &str, random_num: u8) -> String {
    String::from_utf8(
        plain_text
            .as_bytes()
            .iter()
            .zip(key.as_bytes().iter().cycle())
            .map(|(&pl, &kl)| (pl ^ kl).wrapping_shl(random_num.into()))
            .collect(),
    )
    .unwrap()
}
fn decrypt_basic(plain_text: &str, key: &str, random_num: u8) -> String {
    String::from_utf8(
        plain_text
            .as_bytes()
            .iter()
            .zip(key.as_bytes().iter().cycle())
            .map(|(&pl, &kl)| (pl ^ kl).wrapping_shr(random_num.into()))
            .collect(),
    )
    .unwrap()
}




#[cfg(test)]
mod tests {
    use super::encrypt_basic_af;
    use super::*;
    #[test]
    fn test_basic_af_encryption() {
        let string = "hello world";
        let key = "oga boga";

        let encrypted = encrypt_basic_af(string, key);
        assert_ne!(encrypted, string);

        let decrypted = encrypt_basic_af(&encrypted, key);

        assert_eq!(decrypted, string)
    }
    #[test]
    fn test_basic_encryption() {
        let string = "hello world";
        let key = "oga boga";
        let random_num = 10;
        let encrypted = encrypt_basic(string, key, random_num);
        assert_ne!(encrypted, string);

        let decrypted = decrypt_basic(&encrypted, key, random_num);

        assert_eq!(decrypted, string)
    }
}

// key gen - rsa?
// aysametric encryption - ?
//
