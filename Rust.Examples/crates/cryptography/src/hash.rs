extern crate bcrypt;
use bcrypt::{hash_with_result, verify as vh, BcryptError, HashParts, DEFAULT_COST};
pub fn hash(password: &str) -> Result<HashParts, BcryptError> {
    hash_with_result(password, DEFAULT_COST)
}
pub fn verify(password: &str, hashed: &str) -> Result<bool, BcryptError> {
    vh(password, hashed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_hash_functions() {
        let password = "password";
        let not_password = "not_password";
        let h = hash(password).unwrap();
        let should_true = verify(password, &h.to_string()).unwrap();
        let should_false = verify(not_password, &h.to_string()).unwrap();
        println!(
            "Hash Functions -> Password: {} Hash: {} Salt: {} Valid pass: {} Not valid: {}",
            password,
            h.to_string(),
            h.get_salt(),
            should_true,
            should_false
        );
        assert_eq!(should_true, true);
        assert_eq!(should_false, false);
    }
}
