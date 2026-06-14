# frozen_string_literal: true

# =============================================================================
# coding_adventures/pdf417/errors — structured error hierarchy for PDF417
# =============================================================================
#
# Ruby idiom: every error in this library descends from PDF417Error so callers
# can write a single `rescue PDF417Error` to handle all encoding failures, or
# rescue a specific subclass for targeted error handling.
#
# Example usage:
#
#   begin
#     grid = CodingAdventures::PDF417.encode(huge_payload)
#   rescue CodingAdventures::PDF417::InputTooLongError => e
#     puts "Payload too large: #{e.message}"
#   rescue CodingAdventures::PDF417::PDF417Error => e
#     puts "Encoding error: #{e.message}"
#   end

module CodingAdventures
  module PDF417
    # Base class for every PDF417 encoding error.
    #
    # Catch this if you want to handle *all* encoding failures in one place.
    class PDF417Error < StandardError; end

    # Raised when the input byte count exceeds the maximum capacity of any
    # valid PDF417 symbol (90 rows × 30 columns = 2700 codewords minus ECC).
    #
    # The maximum usable byte payload depends on the ECC level chosen:
    # at ECC level 0 you can fit somewhat more raw data than at level 8.
    class InputTooLongError < PDF417Error; end

    # Raised when the caller supplies a `columns:` option outside the legal
    # range 1..30, or when the resulting row count would exceed 90.
    #
    # PDF417 is defined for 1–30 data columns and 3–90 rows. Exceeding either
    # bound would require reading a symbol that no compliant scanner can
    # process.
    class InvalidDimensionsError < PDF417Error; end

    # Raised when the caller supplies an `ecc_level:` option outside 0..8.
    #
    # PDF417 defines eight error-correction levels (0–8). Level 0 provides the
    # minimum two ECC codewords; level 8 provides 512 ECC codewords and can
    # recover from severe physical damage.
    class InvalidECCLevelError < PDF417Error; end
  end
end
