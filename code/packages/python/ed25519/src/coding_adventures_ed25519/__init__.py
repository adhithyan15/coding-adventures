"""ed25519 -- Ed25519 digital signatures (RFC 8032) from scratch.

What Is Ed25519?
================
Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
Daniel J. Bernstein, Niels Duif, Tanja Lange, Peter Schwabe, and Bo-Yin Yang.
It uses the twisted Edwards curve:

    -x^2 + y^2 = 1 + d*x^2*y^2    (mod p)

where p = 2^255 - 19 (hence the name "25519").

Ed25519 is widely used in SSH, TLS, and cryptocurrency because it is:
  - Fast: signing and verification are very efficient
  - Compact: 32-byte keys and 64-byte signatures
  - Secure: 128-bit security level against all known attacks
  - Deterministic: no random nonce needed (prevents Sony PS3-style failures)

How It Works (High Level)
=========================

  1. KEY GENERATION: Hash a 32-byte seed with SHA-512. The first 32 bytes
     (after "clamping") become the secret scalar. Multiply the base point B
     by this scalar to get the public key A.

  2. SIGNING: Hash the seed again to get a "prefix". Hash prefix||message to
     get a deterministic nonce r. Compute R = r*B. Hash R||pubkey||message
     to get k. Compute S = (r + k*a) mod L. Signature = R || S.

  3. VERIFICATION: Recompute k from the signature and message. Check that
     S*B == R + k*A. This works because S = r + k*a, so S*B = r*B + k*a*B
     = R + k*A.

The Curve
=========
Ed25519 uses the "twisted Edwards" form of Curve25519:

    -x^2 + y^2 = 1 + d*x^2*y^2    (mod 2^255 - 19)

This is birationally equivalent to the Montgomery curve used in X25519 key
exchange. The same underlying mathematical group is used, but the Edwards
form allows efficient, complete addition formulas.

"Complete" means the same formula works for ALL point pairs -- no special
cases for doubling, identity, or points at infinity. This simplifies code
and eliminates timing side channels from conditional branches.

Extended Coordinates
====================
Rather than working with (x, y) directly (which requires expensive modular
division for every point addition), we use "extended twisted Edwards
coordinates" (X, Y, Z, T) where:

    x = X/Z,   y = Y/Z,   T = X*Y/Z

This trades one division for several multiplications per operation -- a huge
win since multiplication mod p is ~100x faster than division.

The identity point in extended coordinates is (0, 1, 1, 0), corresponding
to the affine point (0, 1) which satisfies -0 + 1 = 1 + d*0*1 = 1.
"""

from __future__ import annotations

from coding_adventures_ed25519.ed25519 import (
    generate_keypair,
    sign,
    verify,
)

__version__ = "0.1.0"

__all__ = ["generate_keypair", "sign", "verify"]
