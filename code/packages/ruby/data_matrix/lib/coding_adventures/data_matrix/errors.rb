# frozen_string_literal: true

# =============================================================================
# coding_adventures/data_matrix/errors — structured error hierarchy
# =============================================================================
#
# Ruby idiom: every error in this library descends from DataMatrixError so
# callers can write a single `rescue DataMatrixError` to handle all encoding
# failures, or rescue a specific subclass for targeted handling.
#
# Example:
#
#   begin
#     grid = CodingAdventures::DataMatrix.encode(huge_payload)
#   rescue CodingAdventures::DataMatrix::InputTooLongError => e
#     puts "Payload too large: #{e.message}"
#   rescue CodingAdventures::DataMatrix::DataMatrixError => e
#     puts "Encoding error: #{e.message}"
#   end

module CodingAdventures
  module DataMatrix
    # Base class for every Data Matrix ECC200 encoding error.
    #
    # Catch this if you want to handle *all* encoding failures in one place.
    class DataMatrixError < StandardError; end

    # Raised when the input encodes to more codewords than the largest symbol
    # can hold. The largest Data Matrix ECC200 symbol is 144×144, which holds
    # at most 1558 data codewords. Consider splitting across multiple symbols
    # (Structured Append) or switching to a different barcode format.
    class InputTooLongError < DataMatrixError; end

    # Raised when the caller requests an explicit symbol size via `size:`
    # that does not match any of the 30 ECC200 symbol sizes defined in
    # ISO/IEC 16022:2006. Valid square sizes are 10×10 through 144×144;
    # valid rectangular sizes are 8×18, 8×32, 12×26, 12×36, 16×36, 16×48.
    class InvalidSizeError < DataMatrixError; end
  end
end
