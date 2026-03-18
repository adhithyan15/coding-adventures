# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Floating-Point Arithmetic -- IEEE 754 operations built from logic gates.
# ---------------------------------------------------------------------------
#
# This gem implements IEEE 754 floating-point arithmetic at the bit level,
# using logic gates (AND, OR, NOT, XOR) and adder circuits from the lower
# layers of the computing stack.
#
# The dependency chain:
#   Logic Gates (AND, OR, NOT, XOR)
#     +-- Arithmetic (half_adder, full_adder, ripple_carry_adder)
#         +-- FP Arithmetic (this package)
#             +-- Clock (for pipelined units)
#
# Supported formats:
#   FP32 -- 32-bit single precision (the default)
#   FP16 -- 16-bit half precision (GPU mixed-precision training)
#   BF16 -- 16-bit brain float (Google TPU native format)
#
# What you can do:
#   - Encode/decode floats to/from bit-level representations
#   - Add, subtract, multiply floating-point numbers
#   - Fused multiply-add (FMA) with single rounding
#   - Convert between FP32, FP16, and BF16
#   - Run pipelined FP units driven by a clock

require "coding_adventures_logic_gates"
require "coding_adventures_arithmetic"
require "coding_adventures_clock"

require_relative "coding_adventures/fp_arithmetic/version"
require_relative "coding_adventures/fp_arithmetic/formats"
require_relative "coding_adventures/fp_arithmetic/ieee754"
require_relative "coding_adventures/fp_arithmetic/fp_adder"
require_relative "coding_adventures/fp_arithmetic/fp_multiplier"
require_relative "coding_adventures/fp_arithmetic/fma"
require_relative "coding_adventures/fp_arithmetic/pipeline"
