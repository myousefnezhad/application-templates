from cryptography.hazmat.primitives.serialization import (
    load_pem_private_key,
    load_pem_public_key,
)
from cryptography.hazmat.primitives.asymmetric import padding
from cryptography.hazmat.backends import default_backend


class CryptoError(Exception):
    """Base cryptography exception."""


def encrypt(data: bytes, public_key_pem: str) -> bytes:
    """
    Encrypt data using an RSA public key (PKCS#1 v1.5).

    Args:
        data: Plaintext bytes.
        public_key_pem: RSA public key in PEM format.

    Returns:
        Encrypted bytes.
    """
    try:
        public_key = load_pem_public_key(
            public_key_pem.encode("utf-8"),
            backend=default_backend(),
        )

        return public_key.encrypt(
            data,
            padding.PKCS1v15(),
        )

    except Exception as e:
        raise CryptoError(f"RSA encryption failed: {e}") from e


def decrypt(encrypted_data: bytes, private_key_pem: str) -> bytes:
    """
    Decrypt data using an RSA private key (PKCS#1 v1.5).

    Args:
        encrypted_data: Ciphertext.
        private_key_pem: RSA private key in PEM format.

    Returns:
        Decrypted plaintext bytes.
    """
    try:
        private_key = load_pem_private_key(
            private_key_pem.encode("utf-8"),
            password=None,
            backend=default_backend(),
        )

        return private_key.decrypt(
            encrypted_data,
            padding.PKCS1v15(),
        )

    except Exception as e:
        raise CryptoError(f"RSA decryption failed: {e}") from e


if __name__ == "__main__":
    with open("../../data/rsa_example_keys/private.pem", "r") as f:
        private_key = f.read()

    with open("../../data/rsa_example_keys/public.pem", "r") as f:
        public_key = f.read()

    data = input("Enter data to encrypt: ").encode()

    encrypted = encrypt(data, public_key)
    decrypted = decrypt(encrypted, private_key)

    print("Encrypted:", encrypted.hex())
    print("Decrypted:", decrypted.decode())

    assert encrypted != data
    assert decrypted == data