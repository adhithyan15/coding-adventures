# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_polynomial_native.rb — Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The version constant
# 2. The compiled Rust native extension (.so/.bundle/.dll)
#
# The native extension defines:
#   CodingAdventures::PolynomialNative
#
# which exposes polynomial arithmetic over f64 (real-number coefficients) as
# module-level functions. Polynomials are represented as Ruby Arrays of Floats,
# where the array index equals the degree:
#
#   [3.0, 0.0, 2.0]  =>  3 + 0·x + 2·x²
#
# Example usage:
#
#   require "coding_adventures_polynomial_native"
#
#   include CodingAdventures::PolynomialNative  # optional: use as free functions
#
#   a = [1.0, 2.0, 3.0]   # 1 + 2x + 3x²
#   b = [4.0, 5.0]         # 4 + 5x
#   CodingAdventures::PolynomialNative.add(a, b)  #=> [5.0, 7.0, 3.0]

require_relative "coding_adventures/polynomial_native/version"

# Load the compiled native extension.
# Ruby searches for polynomial_native.so (Linux),
# polynomial_native.bundle (macOS), or polynomial_native.dll (Windows).
require "polynomial_native"
