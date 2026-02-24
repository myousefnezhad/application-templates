pub fn base64_encode_from_u8(data: &Vec<u8>) -> String {
    base64_url::encode(&data)
}

pub fn base64_decode_to_u8(data: &str) -> Vec<u8> {
    base64_url::decode(data.as_bytes()).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn check_base64() {
        let data = b"Hello World".to_vec();
        let enc_data = base64_encode_from_u8(&data);
        let dec_data = base64_decode_to_u8(&enc_data);
        println!(
            "Data {:?}\nEncoded {:?}\nDecoded {:?}",
            data, enc_data, dec_data
        );
        assert_eq!(data, dec_data);
    }
}
