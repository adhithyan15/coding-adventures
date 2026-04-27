# frozen_string_literal: true

# coding_adventures_ed25519 -- Ed25519 digital signatures (RFC 8032).
#
# Ed25519 is a high-speed, high-security signature scheme on the twisted
# Edwards curve -x^2 + y^2 = 1 + d*x^2*y^2 over GF(2^255-19).
#
# Features:
# - 128-bit security level
# - Deterministic signatures (no random nonce)
# - 32-byte public keys, 64-byte signatures
# - Complete addition formula (timing-attack resistant)

require_relative "coding_adventures_ed25519/version"
require_relative "coding_adventures_ed25519/ed25519"
