# frozen_string_literal: true

module CodingAdventures
  module CsvParser
    # UnclosedQuoteError is raised when a quoted field is opened with '"' but
    # the matching closing '"' is never found before the end of the input.
    #
    # Example input that triggers this error:
    #
    #   id,value
    #   1,"this is never closed
    #
    # The parser reaches end-of-input while still in IN_QUOTED_FIELD state.
    # There is no safe way to guess where the field was meant to end, so we
    # raise rather than producing corrupt data.
    #
    # Inherits from ArgumentError because the source string is invalid — it
    # violates the CSV grammar. ArgumentError is the Ruby convention for "this
    # argument does not satisfy the contract of this method."
    class UnclosedQuoteError < ArgumentError
    end
  end
end
