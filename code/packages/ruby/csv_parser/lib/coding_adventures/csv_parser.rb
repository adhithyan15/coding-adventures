# frozen_string_literal: true

require_relative "csv_parser/version"
require_relative "csv_parser/errors"
require_relative "csv_parser/parser"

module CodingAdventures
  # CsvParser converts CSV text into an array of row hashes.
  #
  # Each row is a Hash mapping column name (String) to field value (String).
  # The first row of the CSV is always treated as the header row that defines
  # the column names. No type coercion is performed — all values are Strings.
  #
  # This module does NOT use Ruby's standard library CSV class. The
  # implementation is a hand-rolled character-by-character state machine,
  # which makes the parsing logic explicit and educational.
  #
  # == Example
  #
  #   require "coding_adventures/csv_parser"
  #
  #   rows = CodingAdventures::CsvParser.parse_csv("name,age\nAlice,30\nBob,25")
  #   # => [{"name"=>"Alice", "age"=>"30"}, {"name"=>"Bob", "age"=>"25"}]
  #
  #   rows = CodingAdventures::CsvParser.parse_csv("a\tb\nX\t42", delimiter: "\t")
  #   # => [{"a"=>"X", "b"=>"42"}]
  #
  module CsvParser
    # Parse CSV text and return an array of row hashes.
    #
    # The first row of the source is treated as the header, defining the
    # column names used as keys in each subsequent row's Hash.
    #
    # All field values are returned as Strings. Type coercion (e.g., "42" → 42)
    # is the responsibility of the caller.
    #
    # @param source    [String] the CSV text (UTF-8 recommended)
    # @param delimiter [String] the field delimiter character (default: ",").
    #   Common alternatives: "\t" (TSV), ";" (European CSV), "|" (pipe).
    # @return [Array<Hash{String => String}>] array of row hashes
    # @raise [UnclosedQuoteError] if a quoted field is never closed before EOF
    #
    # @example Basic usage
    #   parse_csv("name,age\nAlice,30")
    #   # => [{"name"=>"Alice", "age"=>"30"}]
    #
    # @example Custom delimiter
    #   parse_csv("name\tage\nAlice\t30", delimiter: "\t")
    #   # => [{"name"=>"Alice", "age"=>"30"}]
    #
    # @example Quoted field with comma
    #   parse_csv('a,b\nWidget,"A small, gadget"')
    #   # => [{"a"=>"Widget", "b"=>"A small, gadget"}]
    #
    # @example Escaped double-quote
    #   parse_csv('a\n"say ""hi"""')
    #   # => [{"a"=>'say "hi"'}]
    #
    def self.parse_csv(source, delimiter: ",")
      # ── Step 1: tokenise into raw rows (array of array of strings) ─────────
      raw_rows = Parser.scan(source, delimiter)

      # ── Step 2: empty file → empty result ──────────────────────────────────
      return [] if raw_rows.empty?

      # ── Step 3: first row is the header ────────────────────────────────────
      header = raw_rows[0]
      data_rows = raw_rows[1..]

      # ── Step 4: header-only file → empty result ─────────────────────────────
      return [] if data_rows.empty?

      # ── Step 5: zip each data row with the header ───────────────────────────
      data_rows.map { |row| zip_row(header, row) }
    end

    # Combine a header row and a data row into a Hash.
    #
    # Implements ragged-row handling:
    #   - Data row shorter than header → missing fields filled with "".
    #   - Data row longer than header  → extra fields discarded.
    #
    # @param header [Array<String>] column names from the first CSV row
    # @param row    [Array<String>] field values from a data row
    # @return [Hash{String => String}]
    #
    # @example Padding
    #   zip_row(["a","b","c"], ["1","2"])
    #   # => {"a"=>"1", "b"=>"2", "c"=>""}
    #
    # @example Truncating
    #   zip_row(["a","b"], ["1","2","3","4"])
    #   # => {"a"=>"1", "b"=>"2"}
    #
    def self.zip_row(header, row)
      result = {}
      header.each_with_index do |col_name, idx|
        # Use the field value if present; otherwise pad with empty string.
        result[col_name] = idx < row.length ? row[idx] : ""
      end
      result
    end

    private_class_method :zip_row
  end
end
