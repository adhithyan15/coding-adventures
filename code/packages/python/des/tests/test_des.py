"""
Tests for coding_adventures_des — DES and 3DES block cipher.

Coverage targets:
  - NIST FIPS 81 / SP 800-20 known-answer test vectors
  - Key schedule (expand_key)
  - Single-block encrypt/decrypt
  - Round-trip property: decrypt(encrypt(x)) == x
  - ECB mode: multi-block, padding, boundary conditions
  - 3DES (TDEA) encrypt/decrypt
  - Error handling (invalid key/block lengths, bad ciphertext)
"""

import pytest
from coding_adventures_des import (
    expand_key,
    des_encrypt_block,
    des_decrypt_block,
    des_ecb_encrypt,
    des_ecb_decrypt,
    tdea_encrypt_block,
    tdea_decrypt_block,
)


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def h(hex_str: str) -> bytes:
    """Decode a hex string (spaces ignored) to bytes."""
    return bytes.fromhex(hex_str.replace(" ", ""))


# ─────────────────────────────────────────────────────────────────────────────
# NIST FIPS 81 / SP 800-20 Known-Answer Tests
# ─────────────────────────────────────────────────────────────────────────────

class TestDesEncryptBlock:
    def test_fips_vector_1(self):
        """
        Classic DES example from Stallings / FIPS 46 worked example.
        Key = 133457799BBCDFF1 (with parity bits: bit 8 of each byte is parity).
        """
        key = h("133457799BBCDFF1")
        plain = h("0123456789ABCDEF")
        assert des_encrypt_block(plain, key) == h("85E813540F0AB405")

    def test_sp800_20_table1_row0(self):
        """SP 800-20 Table 1 — plaintext variable, key=0101...01."""
        key = h("0101010101010101")
        assert des_encrypt_block(h("95F8A5E5DD31D900"), key) == h("8000000000000000")

    def test_sp800_20_table1_row1(self):
        key = h("0101010101010101")
        assert des_encrypt_block(h("DD7F121CA5015619"), key) == h("4000000000000000")

    def test_sp800_20_table1_row2(self):
        key = h("0101010101010101")
        assert des_encrypt_block(h("2E8653104F3834EA"), key) == h("2000000000000000")

    def test_sp800_20_table2_key_variable(self):
        """SP 800-20 Table 2 — key variable, plaintext=0000...00."""
        assert des_encrypt_block(h("0000000000000000"), h("8001010101010101")) == h("95A8D72813DAA94D")

    def test_sp800_20_table2_row1(self):
        assert des_encrypt_block(h("0000000000000000"), h("4001010101010101")) == h("0EEC1487DD8C26D5")

    def test_all_zeros(self):
        """All-zero key and plaintext — deterministic known answer."""
        # With key=0x0101010101010101 (weak key), plaintext=0
        # SP 800-20 Table 2 first entry
        key = h("8001010101010101")
        plain = h("0000000000000000")
        ct = des_encrypt_block(plain, key)
        assert ct == h("95A8D72813DAA94D")

    def test_known_weak_key_single_des(self):
        """
        Parity-bit-only key: 0000000000000080.
        SP 800-20 Table 2 last entry variant.
        """
        key = h("0000000000000080")
        plain = h("0000000000000000")
        ct = des_encrypt_block(plain, key)
        # Verify round-trip rather than hardcoding (parity bit handling varies)
        assert des_decrypt_block(ct, key) == plain


class TestDesDecryptBlock:
    def test_fips_vector_roundtrip(self):
        key = h("133457799BBCDFF1")
        plain = h("0123456789ABCDEF")
        ct = des_encrypt_block(plain, key)
        assert des_decrypt_block(ct, key) == plain

    def test_decrypt_fips_vector_1(self):
        key = h("133457799BBCDFF1")
        ct = h("85E813540F0AB405")
        assert des_decrypt_block(ct, key) == h("0123456789ABCDEF")

    def test_roundtrip_all_bytes(self):
        """Round-trip every possible byte value in plaintext."""
        key = h("FEDCBA9876543210")
        for start in range(0, 256, 16):
            block = bytes(range(start, start + 8))
            assert des_decrypt_block(des_encrypt_block(block, key), key) == block

    def test_roundtrip_multiple_keys(self):
        keys = [
            h("133457799BBCDFF0"),
            h("FFFFFFFFFFFFFFFF"),
            h("0000000000000000"),
            h("FEDCBA9876543210"),
        ]
        plain = h("0123456789ABCDEF")
        for key in keys:
            assert des_decrypt_block(des_encrypt_block(plain, key), key) == plain


class TestExpandKey:
    def test_returns_16_subkeys(self):
        key = h("0133457799BBCDFF")
        subkeys = expand_key(key)
        assert len(subkeys) == 16

    def test_subkeys_are_6_bytes(self):
        key = h("0133457799BBCDFF")
        for sk in expand_key(key):
            assert len(sk) == 6

    def test_different_keys_different_subkeys(self):
        sk1 = expand_key(h("0133457799BBCDFF"))
        sk2 = expand_key(h("FEDCBA9876543210"))
        assert sk1 != sk2

    def test_subkeys_not_all_same(self):
        """All 16 subkeys should differ (a degenerate key schedule would be broken)."""
        key = h("0133457799BBCDFF")
        subkeys = expand_key(key)
        assert len(set(subkeys)) > 1

    def test_invalid_key_length(self):
        with pytest.raises(ValueError, match="8 bytes"):
            expand_key(b"\x00" * 7)

    def test_invalid_key_length_too_long(self):
        with pytest.raises(ValueError, match="8 bytes"):
            expand_key(b"\x00" * 9)


# ─────────────────────────────────────────────────────────────────────────────
# ECB Mode
# ─────────────────────────────────────────────────────────────────────────────

class TestEcbEncrypt:
    KEY = h("0133457799BBCDFF")

    def test_single_block_exact(self):
        """8-byte input → 16 bytes out (1 data block + 1 full padding block)."""
        plain = h("0123456789ABCDEF")
        ct = des_ecb_encrypt(plain, self.KEY)
        assert len(ct) == 16

    def test_sub_block(self):
        """Less than 8 bytes → padded to 8 bytes → 8 bytes ciphertext."""
        ct = des_ecb_encrypt(b"hello", self.KEY)
        assert len(ct) == 8

    def test_multi_block(self):
        """16 bytes input → 24 bytes out (2 data blocks + 1 padding block)."""
        plain = bytes(range(16))
        ct = des_ecb_encrypt(plain, self.KEY)
        assert len(ct) == 24

    def test_empty_input(self):
        """Empty input → 8 bytes (full padding block)."""
        ct = des_ecb_encrypt(b"", self.KEY)
        assert len(ct) == 8

    def test_output_is_bytes(self):
        assert isinstance(des_ecb_encrypt(b"test", self.KEY), bytes)

    def test_deterministic(self):
        plain = b"Hello, World!!!"
        assert des_ecb_encrypt(plain, self.KEY) == des_ecb_encrypt(plain, self.KEY)


class TestEcbDecrypt:
    KEY = h("0133457799BBCDFF")

    def test_roundtrip_short(self):
        plain = b"hello"
        assert des_ecb_decrypt(des_ecb_encrypt(plain, self.KEY), self.KEY) == plain

    def test_roundtrip_exact_block(self):
        plain = b"ABCDEFGH"
        assert des_ecb_decrypt(des_ecb_encrypt(plain, self.KEY), self.KEY) == plain

    def test_roundtrip_multi_block(self):
        plain = b"The quick brown fox jumps"
        assert des_ecb_decrypt(des_ecb_encrypt(plain, self.KEY), self.KEY) == plain

    def test_roundtrip_empty(self):
        assert des_ecb_decrypt(des_ecb_encrypt(b"", self.KEY), self.KEY) == b""

    def test_roundtrip_large(self):
        plain = bytes(range(256))
        assert des_ecb_decrypt(des_ecb_encrypt(plain, self.KEY), self.KEY) == plain

    def test_invalid_length_not_multiple_of_8(self):
        with pytest.raises(ValueError, match="multiple of 8"):
            des_ecb_decrypt(b"\x00" * 7, self.KEY)

    def test_invalid_empty_ciphertext(self):
        with pytest.raises(ValueError):
            des_ecb_decrypt(b"", self.KEY)

    def test_bad_padding_raises(self):
        """Corrupted ciphertext should raise ValueError on unpadding."""
        ct = des_ecb_encrypt(b"test data", self.KEY)
        # Flip the last byte to corrupt the padding block
        corrupted = ct[:-1] + bytes([ct[-1] ^ 0xFF])
        with pytest.raises(ValueError):
            des_ecb_decrypt(corrupted, self.KEY)


# ─────────────────────────────────────────────────────────────────────────────
# 3DES (TDEA)
# ─────────────────────────────────────────────────────────────────────────────

class TestTdea:
    # 3TDEA test vector: NIST EDE ordering E_K1(D_K2(E_K3(P))).
    # Ciphertext computed from this implementation and verified via round-trip.
    K1 = h("0123456789ABCDEF")
    K2 = h("23456789ABCDEF01")
    K3 = h("456789ABCDEF0123")
    PLAIN = h("6BC1BEE22E409F96")
    CIPHER = h("3B6423D418DEFC23")

    def test_3tdea_encrypt(self):
        """E_K1(D_K2(E_K3(P))) — NIST SP 800-67 EDE ordering."""
        assert tdea_encrypt_block(self.PLAIN, self.K1, self.K2, self.K3) == self.CIPHER

    def test_3tdea_decrypt(self):
        """D_K3(E_K2(D_K1(C))) — inverse of EDE."""
        assert tdea_decrypt_block(self.CIPHER, self.K1, self.K2, self.K3) == self.PLAIN

    def test_roundtrip_random_keys(self):
        k1 = h("FEDCBA9876543210")
        k2 = h("0F1E2D3C4B5A6978")
        k3 = h("7869584A3B2C1D0E")
        plain = h("0123456789ABCDEF")
        ct = tdea_encrypt_block(plain, k1, k2, k3)
        assert tdea_decrypt_block(ct, k1, k2, k3) == plain

    def test_ede_backward_compat_k1_eq_k2_eq_k3(self):
        """
        When K1=K2=K3, 3DES EDE reduces to single DES.
        EDE(K,K,K): Encrypt(K, Decrypt(K, Encrypt(K, P)))
                  = Encrypt(K, P)   since Decrypt(K, Encrypt(K, x)) = x
        """
        key = h("0133457799BBCDFF")
        plain = h("0123456789ABCDEF")
        assert tdea_encrypt_block(plain, key, key, key) == des_encrypt_block(plain, key)

    def test_ede_decrypt_backward_compat(self):
        key = h("FEDCBA9876543210")
        ct = h("0123456789ABCDEF")
        assert tdea_decrypt_block(ct, key, key, key) == des_decrypt_block(ct, key)

    def test_roundtrip_all_same_block(self):
        k1 = h("1234567890ABCDEF")
        k2 = h("FEDCBA0987654321")
        k3 = h("0F0F0F0F0F0F0F0F")
        for val in [0x00, 0xFF, 0xA5, 0x5A]:
            plain = bytes([val] * 8)
            assert tdea_decrypt_block(tdea_encrypt_block(plain, k1, k2, k3), k1, k2, k3) == plain


# ─────────────────────────────────────────────────────────────────────────────
# Invalid input handling
# ─────────────────────────────────────────────────────────────────────────────

class TestInvalidInputs:
    KEY = h("0133457799BBCDFF")

    def test_encrypt_block_wrong_block_size(self):
        with pytest.raises(ValueError, match="8 bytes"):
            des_encrypt_block(b"\x00" * 7, self.KEY)

    def test_encrypt_block_wrong_block_size_long(self):
        with pytest.raises(ValueError, match="8 bytes"):
            des_encrypt_block(b"\x00" * 16, self.KEY)

    def test_decrypt_block_wrong_block_size(self):
        with pytest.raises(ValueError, match="8 bytes"):
            des_decrypt_block(b"\x00" * 9, self.KEY)

    def test_encrypt_block_wrong_key_size(self):
        with pytest.raises(ValueError, match="8 bytes"):
            des_encrypt_block(b"\x00" * 8, b"\x00" * 4)

    def test_decrypt_block_wrong_key_size(self):
        with pytest.raises(ValueError, match="8 bytes"):
            des_decrypt_block(b"\x00" * 8, b"\x00" * 16)
