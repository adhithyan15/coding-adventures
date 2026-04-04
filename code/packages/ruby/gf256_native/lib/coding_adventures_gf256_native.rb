# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_gf256_native.rb — Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The version constant
# 2. The compiled Rust native extension (.so/.bundle/.dll)
#
# The native extension defines:
#   CodingAdventures::GF256Native
#
# which exposes GF(2^8) field arithmetic as module-level functions.
# GF(256) is the finite field used by Reed-Solomon error correction
# (QR codes, CDs, DVDs) and AES encryption.
#
# Elements of GF(256) are Ruby Integers in the range 0..=255.
# Arithmetic is NOT ordinary integer arithmetic — it is polynomial
# arithmetic over GF(2), reduced modulo the primitive polynomial
# x^8 + x^4 + x^3 + x^2 + 1 (= 0x11D = 285).
#
# Key fact: addition = subtraction = XOR (characteristic 2 field).
#
# Module constants:
#   CodingAdventures::GF256Native::ZERO                = 0
#   CodingAdventures::GF256Native::ONE                 = 1
#   CodingAdventures::GF256Native::PRIMITIVE_POLYNOMIAL = 285
#
# Example usage:
#
#   require "coding_adventures_gf256_native"
#
#   M = CodingAdventures::GF256Native
#   M.add(0x53, 0xCA)       #=> 0x99  (XOR)
#   M.multiply(2, 4)        #=> 8
#   M.multiply(128, 2)      #=> 29    (overflow, reduced mod 0x11D)
#   M.inverse(2)            #=> 142   (2 * 142 = 1 in GF(256))

require_relative "coding_adventures/gf256_native/version"

# Load the compiled native extension.
# Ruby searches for gf256_native.so (Linux),
# gf256_native.bundle (macOS), or gf256_native.dll (Windows).
require "gf256_native"
