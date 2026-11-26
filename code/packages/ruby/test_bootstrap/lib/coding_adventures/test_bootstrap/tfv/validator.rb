# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    # Strict validator for TFV v1 text, the line format that carries test corpora
    # between languages (TF01 section 2).  This is a VALIDATOR, not a reader: its
    # one job here is to let the corpus runner check the pbt-v1 `corpus_reject`
    # cases with a real parser instead of skipping them.
    #
    #   tfv 1
    #   # comment
    #   <id> R<n> K<field> <value> K<field> <value> ...
    #
    # A value is one of
    #
    #   N  T  F                 null, true, false
    #   I<int>                  canonical decimal integer, at most 1000 digits
    #   D<16 hex>               IEEE-754 bit pattern, lowercase
    #   X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
    #   L<n> v1 ... vn          list
    #   M<n> k1 v1 ... kn vn    map; no two keys deep-equal
    #
    # Tokens are separated by exactly one ASCII space.  Rejected: uppercase or
    # odd-length hex, `I-0`, `I007`, a count that disagrees with the tokens,
    # trailing tokens, doubled spaces, a duplicate field or record id, invalid
    # UTF-8 inside S, a carriage return in a record line, nesting deeper than 64
    # containers and integers of more than 1000 digits.
    #
    # Worked example: "tfv 1\nr R1 Kop S706274 Kdata XFF\n" fails (XFF is uppercase
    # and odd length); "bad header\n" fails (the first line must be exactly "tfv 1").
    module Tfv
      HEADER = "tfv 1"
      MAX_DEPTH = 64
      MAX_INT_DIGITS = 1000
      MAX_INPUT = 1 << 20

      class InvalidTfvError < StandardError
        def initialize(message)
          super("invalid TFV: #{message}")
        end
      end

      def self.valid?(text)
        validate(text)
        true
      rescue InvalidTfvError
        false
      end

      def self.q(text)
        shown = (text.length > 64) ? "#{text[0, 64]}..." : text
        "\"#{CorpusJson::Canon.escape(shown, quote: true)}\""
      end

      # Returns nil for a well-formed document, raises InvalidTfvError otherwise.
      # Reserves no memory from a count in the input.
      def self.validate(text)
        raise InvalidTfvError, "input must be a String" unless text.is_a?(String)
        raise InvalidTfvError, "input of #{text.bytesize} bytes exceeds #{MAX_INPUT}" if text.bytesize > MAX_INPUT

        text = text.dup.force_encoding(Encoding::UTF_8)
        raise InvalidTfvError, "document is not valid UTF-8" unless text.valid_encoding?

        lines = text.split("\n", -1)
        lines.pop if lines.last == "" # the final newline is optional
        raise InvalidTfvError, "the first line must be exactly #{q(HEADER)}" if lines.empty? || lines[0] != HEADER

        seen_ids = {}
        lines.each_with_index do |line, n|
          next if n == 0 || line.empty? || line.start_with?("#")

          begin
            id = validate_record(line)
          rescue InvalidTfvError => e
            raise InvalidTfvError, "line #{n + 1}: #{e.message.delete_prefix("invalid TFV: ")}"
          end
          raise InvalidTfvError, "line #{n + 1}: duplicate record id #{q(id)}" if seen_ids.key?(id)

          seen_ids[id] = true
        end
        nil
      end

      def self.valid_id?(id) = id.match?(%r{\A[A-Za-z0-9._/-]+\z})

      def self.valid_field_name?(name) = name.match?(/\A[a-z][a-z0-9_]*\z/)

      def self.lower_hex_bytes(text)
        return nil unless text.length.even? && text.match?(/\A[0-9a-f]*\z/)

        [text].pack("H*")
      end

      def self.canonical_nat(text)
        raise InvalidTfvError, "empty count" if text.empty?
        raise InvalidTfvError, "non-digit in count #{q(text)}" unless text.match?(/\A[0-9]+\z/)
        raise InvalidTfvError, "leading zero in count #{q(text)}" if text.length > 1 && text.start_with?("0")
        raise InvalidTfvError, "count #{q(text)} too large" if text.length > 9

        text.to_i
      end

      def self.valid_int!(body)
        neg = body.start_with?("-")
        digits = neg ? body[1..] : body
        raise InvalidTfvError, "empty integer" if digits.empty?
        raise InvalidTfvError, "non-digit in integer #{q(body)}" unless digits.match?(/\A[0-9]+\z/)
        raise InvalidTfvError, "non-canonical integer #{q(body)}" if digits.start_with?("0") && (digits.length > 1 || neg)
        raise InvalidTfvError, "integer of #{digits.length} digits exceeds #{MAX_INT_DIGITS}" if digits.length > MAX_INT_DIGITS
      end

      def self.validate_record(line)
        raise InvalidTfvError, "carriage return in a record line" if line.include?("\r")

        # a REGEXP, not " ": split(" ") is awk-style and would swallow doubled spaces
        tokens = line.split(/ /, -1)
        raise InvalidTfvError, "leading, trailing or doubled space" if tokens.any?(&:empty?)
        raise InvalidTfvError, "a record needs an id and R<n>" if tokens.size < 2

        id = tokens[0]
        raise InvalidTfvError, "bad record id #{q(id)}" unless valid_id?(id)
        raise InvalidTfvError, "expected R<n>, got #{q(tokens[1])}" unless tokens[1].start_with?("R")

        count = canonical_nat(tokens[1][1..])
        reader = Reader.new(tokens, 2)
        seen = {}
        count.times do
          key_tok = reader.next_token
          raise InvalidTfvError, "bad field key #{q(key_tok)}" unless key_tok.start_with?("K") && valid_field_name?(key_tok[1..])
          raise InvalidTfvError, "duplicate field #{q(key_tok[1..])}" if seen.key?(key_tok)

          seen[key_tok] = true
          reader.value(0)
        end
        raise InvalidTfvError, "trailing tokens after the last field" unless reader.done?

        id
      end

      # Walks the tokens of one record.  `value` returns a deep-equality key: equal
      # for two values exactly when TF01 3.1 calls them deep-equal.
      class Reader
        def initialize(tokens, start)
          @tokens = tokens
          @pos = start
        end

        def done? = @pos == @tokens.size

        def next_token
          raise InvalidTfvError, "record ends early" if @pos >= @tokens.size

          tok = @tokens[@pos]
          @pos += 1
          tok
        end

        def value(depth)
          tok = next_token
          tag = tok[0]
          body = tok[1..]
          case tag
          when "N", "T", "F"
            raise InvalidTfvError, "unexpected body in #{Tfv.q(tok)}" unless body.empty?

            tag
          when "I"
            Tfv.valid_int!(body)
            tok
          when "D" then float(tok, body)
          when "X"
            raise InvalidTfvError, "bytes need even-length lowercase hex: #{Tfv.q(tok)}" unless Tfv.lower_hex_bytes(body)

            tok
          when "S" then text(tok, body)
          when "L", "M" then container(tag, body, depth)
          else raise InvalidTfvError, "unknown tag in #{Tfv.q(tok)}"
          end
        end

        private

        def float(tok, body)
          raw = (body.length == 16) ? Tfv.lower_hex_bytes(body) : nil
          raise InvalidTfvError, "float needs exactly 16 lowercase hex digits: #{Tfv.q(tok)}" unless raw

          exp = ((raw.getbyte(0) & 0x7f) << 4) | (raw.getbyte(1) >> 4)
          mantissa_zero = (raw.getbyte(1) & 0x0f) == 0 && raw.byteslice(2, 6).bytes.all?(&:zero?)
          # Every NaN equals every NaN; -0 and +0 stay distinct.
          (exp == 0x7ff && !mantissa_zero) ? "Dnan" : tok
        end

        def text(tok, body)
          raw = Tfv.lower_hex_bytes(body)
          raise InvalidTfvError, "text needs even-length lowercase hex: #{Tfv.q(tok)}" unless raw
          raise InvalidTfvError, "text is not strict UTF-8: #{Tfv.q(tok)}" unless raw.dup.force_encoding(Encoding::UTF_8).valid_encoding?

          tok
        end

        def container(tag, body, depth)
          raise InvalidTfvError, "nesting deeper than #{MAX_DEPTH}" if depth + 1 > MAX_DEPTH

          count = Tfv.canonical_nat(body)
          if tag == "L"
            items = []
            count.times { items << value(depth + 1) } # never reserve memory from a count
            return "L[#{items.join(",")},]"
          end
          keys = {}
          entries = []
          count.times do
            k = value(depth + 1)
            v = value(depth + 1)
            raise InvalidTfvError, "duplicate map key" if keys.key?(k)

            keys[k] = true
            entries << "(#{k}=>#{v})"
          end
          "M{#{entries.sort.join}}"
        end
      end
    end
  end
end
