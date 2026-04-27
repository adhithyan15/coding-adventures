"""
coding-adventures-x25519 — X25519 Elliptic Curve Diffie-Hellman (RFC 7748)
===========================================================================

A pure-Python, zero-dependency implementation of X25519 key agreement.
All field arithmetic over GF(2^255 - 19) is implemented from scratch.

Quick start::

    import os
    from coding_adventures_x25519 import x25519, x25519_base, generate_keypair

    # Alice generates her keypair
    alice_private = os.urandom(32)
    alice_public = generate_keypair(alice_private)

    # Bob generates his keypair
    bob_private = os.urandom(32)
    bob_public = generate_keypair(bob_private)

    # Both compute the same shared secret
    shared_a = x25519(alice_private, bob_public)
    shared_b = x25519(bob_private, alice_public)
    assert shared_a == shared_b  # Diffie-Hellman magic!
"""

from coding_adventures_x25519.x25519 import (
    BASE_POINT,
    P,
    A24,
    cswap,
    decode_scalar,
    decode_u_coordinate,
    encode_u_coordinate,
    field_add,
    field_invert,
    field_mul,
    field_square,
    field_sub,
    generate_keypair,
    x25519,
    x25519_base,
)

__all__ = [
    "A24",
    "BASE_POINT",
    "P",
    "cswap",
    "decode_scalar",
    "decode_u_coordinate",
    "encode_u_coordinate",
    "field_add",
    "field_invert",
    "field_mul",
    "field_square",
    "field_sub",
    "generate_keypair",
    "x25519",
    "x25519_base",
]
