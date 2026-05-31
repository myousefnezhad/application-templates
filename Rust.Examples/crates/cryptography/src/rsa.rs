use rsa::{
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

pub fn encrypt(data: &[u8], public_key: &str) -> Result<Vec<u8>, rsa::Error> {
    let mut rng = rand::thread_rng();
    let pub_key = RsaPublicKey::from_pkcs1_pem(&public_key).unwrap(); // TODO: Error Handling
    pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &data[..])
}

pub fn decrypt(enc_data: &[u8], private_key: &str) -> Result<Vec<u8>, rsa::Error> {
    let priv_key = RsaPrivateKey::from_pkcs1_pem(private_key).unwrap();
    priv_key.decrypt(Pkcs1v15Encrypt, &enc_data)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_rsa_functions() {
        let private_key = r#"
-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAqmggV3RAdY3HCJexvy3wugOp0y4pP3NZwLsvkkII8GIIGPqC
0Ms6GxFP0ymcuea6qgFJaMQiypDlo9W+MFBVNAOxbHq9xLm7xR10lAW64RtPfwUP
Ey1RK08kBkt14koZEufWmM0E3wcaHzkBTij3W3MobuREGnN9BZWA1IffWL8bhKDn
Ri8cAqrDQHDzrDgIii1/IkWJifTmJbdTAesWb8Gs3cXKVZyQEu/0RltzjzUigkOw
U07aBGDjr1RMIaH4orihkrWkTrpPECvabYJns0hgJ8gLr1TW3AYgh2Z1zwdxQISi
Zy5Q5V9hbT31l4V6eC7Mf8wF9DBNmlwX0OztdwIDAQABAoIBAA+SuYkhUI/0F4e8
/6g2a7Gl13WhwKPeuyNmcYs4kE/4alk7pHKuOyhLG9QmXhjZ/oTxd8LkndgmVGsP
HYHIoNv1wT3mAuuOHJwy724xqENqdzuz+8DOAbhNoL/khVvBvgRHMYqNJGm5E+8p
8TXlf4mZGIDESRPW9xeCDLm5Lk7e7b8WzS1G0MLQbAqyU9il8MmwlJxm36w9TxKA
JwHwzoBpjZjFTml87PGgXBmQLXsIO/TE1DUP6p9/nUujNvGBMLZDlEYpMaIy3HTJ
1DrAZx6sL/7pj+07ilAIG+mtI9yZwb9IoAPhvaBdWVsN21Vt4DxfYSt1PUuDXKuV
BKa3VykCgYEA7MokttbJgVoIKW9UMek5Xu2+wQnLVTGJPWblAQ+LDT30k/jB4cB9
DjteFOJgC4X/fmQAettcDs5F2GaF6t+vFKvY8gYK5vmaQOciexETfUM9wuCQUB/w
AycxkxctVaZTQze90vaE1Y4IdwxWCxh5VkmSAMTP1PXmr4N3KgwteQ8CgYEAuDtI
1K8nC0WsW034S5ziwilJ8w8gOSUVNPaKRtADdih+YsU3/LkkEh5UnMU/rYBVVd95
Tz9GTAQzo3snEE0/oneTyAJwFFgz8yUJWJmXulgrU6zJAFIi2w0A8GfoSvxStg66
Q+w3bjtQZXlBijFf5/SMxQs3qET146Ju5arUNRkCgYAH3K5bY8ZNBTBAL/IbtiKx
hDNRNYIK0ho8b64q2tlDF0uHnjrRxBJi5pW5G8sUlt5k0+0wJxhnZXHsPXs00Th+
9zW803aJNMBqZ0I909NR1Wm3jr0769OMR0DeIWbf8SXSlYc3+fdvH8goK8CvnaBp
vQeb/uznpUKDkVnji4P3AQKBgQCtSyaw2KizU7BSmj7qcQJjfI8Q0LOQzS7riNR8
04HODT0FD1ogoM4rNNiGNow1WbfrdLAZOP2Y14LzTg5qqR8oZa5evtxKZsVG+xmV
e7RbFkB7DtZrNNeQkOTHPhYlIr9yfNv8tB9X0vdG3A4oPVVnTIN1fZ227FcW7l2+
Lzg5UQKBgGbrqfth+0y5DZH4Xi3NO/Xpz21KY1CmbpMzKCmsnHiQMUSbxS+nohnW
7599JAEVJbvNpADUEfIoiqB/HTwHVYkjhb+Y+e81AifohUEaMWsN9ykxmf7oWihB
D93ioz6xk67xs/x2c1Yg9W2uwhSMtqIMyE364YdECEkWBdak+mh+
-----END RSA PRIVATE KEY-----
"#;

        let public_key = r#"
-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAqmggV3RAdY3HCJexvy3wugOp0y4pP3NZwLsvkkII8GIIGPqC0Ms6
GxFP0ymcuea6qgFJaMQiypDlo9W+MFBVNAOxbHq9xLm7xR10lAW64RtPfwUPEy1R
K08kBkt14koZEufWmM0E3wcaHzkBTij3W3MobuREGnN9BZWA1IffWL8bhKDnRi8c
AqrDQHDzrDgIii1/IkWJifTmJbdTAesWb8Gs3cXKVZyQEu/0RltzjzUigkOwU07a
BGDjr1RMIaH4orihkrWkTrpPECvabYJns0hgJ8gLr1TW3AYgh2Z1zwdxQISiZy5Q
5V9hbT31l4V6eC7Mf8wF9DBNmlwX0OztdwIDAQAB
-----END RSA PUBLIC KEY-----
"#;        
        let data = b"hello world";
        let enc_data = encrypt(data, public_key).unwrap();
        println!("Encrypted: {:?}", enc_data);
        let dec_data = decrypt(&enc_data, private_key).unwrap();
        println!("Decrypted: {}", std::str::from_utf8(&dec_data).unwrap());
        // Encrypt
        assert_ne!(&data[..], &enc_data[..]);
        // Decrypt
        assert_eq!(&data[..], &dec_data[..]);
    }
}
