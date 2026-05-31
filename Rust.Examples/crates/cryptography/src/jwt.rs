pub use jsonwebtoken::Algorithm;
use jsonwebtoken::errors::Error;
pub use jsonwebtoken::errors::ErrorKind as JWTErrorKind;
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, TokenData, Validation, decode as jdec, encode as jenc,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: i64,
    pub iat: i64,
    pub id: String,
    pub name: String,
    pub email: String,
    pub session: u64,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub session: u64,
    pub token: String,
    pub source: String,
}

pub fn generate_token<T>(algorithm: Algorithm, key: &str, claims: T) -> Result<String, Error>
where
    T: Serialize,
{
    jenc(
        &Header::new(algorithm),
        &claims,
        &EncodingKey::from_secret(key.as_bytes()),
    )
}

pub fn validate_token<T>(
    algorithm: Algorithm,
    key: &str,
    token: &str,
) -> Result<TokenData<T>, Error>
where
    for<'a> T: Deserialize<'a>,
{
    let validation = Validation::new(algorithm);
    jdec::<T>(
        &token,
        &DecodingKey::from_secret(key.as_bytes()),
        &validation,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use app_config::AppConfig;
    #[test]
    fn check_jwt_functions() {
        let mut my_claim = Claims {
            exp: i64::MAX,
            iat: 0,
            id: "".to_owned(),
            name: "test".to_owned(),
            email: "test@unit".to_owned(),
            session: 1,
        };
        let config = AppConfig::new();
        let secret = config.jwt_access_key;
        match generate_token(Algorithm::HS256, &secret, &my_claim) {
            Ok(token) => {
                println!("Valid Token: {}", token);
                let validate = validate_token::<Claims>(Algorithm::HS256, &secret, &token).unwrap();
                println!("Validation Claim {:#?}", validate.claims);
                println!("Validation Header {:#?}", validate.header);
                assert_eq!(validate.claims.email, "test@unit".to_owned());
            }
            Err(e) => println!("{}", e),
        };
        my_claim.exp = 0;
        let token = generate_token(Algorithm::HS256, &secret, &my_claim).unwrap();
        println!("Invalid Token: {}", token);
        let error = validate_token::<Claims>(Algorithm::HS256, &secret, &token).err();
        assert_eq!(error.unwrap().kind(), &JWTErrorKind::ExpiredSignature)
    }
}
