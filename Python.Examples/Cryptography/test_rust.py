from main import decrypt

def test_rust_cipher():
    with open("../../data/rsa_example_keys/private.pem", "r") as f:
        private_key = f.read()

    with open("../../Rust.Examples/crates/cryptography/rust_cipher.bin", "rb") as f:
        encrypted = f.read()

    decrypted = decrypt(encrypted, private_key)

    print("Encrypted (hex):", encrypted.hex())
    print("Decrypted:", decrypted.decode())

    assert decrypted == b"hello world"

    print("✅ Successfully decrypted Rust-produced ciphertext.")


if __name__ == "__main__":
    test_rust_cipher()
