# frozen_string_literal: true

# ---------------------------------------------------------------------------
# token_classifier.rb — Classify a single argv token into a typed event
# ---------------------------------------------------------------------------
#
# The token classifier sits between raw argv strings and the parsing state
# machine. It reads one token at a time and determines what kind of thing
# it is: a flag, a positional argument, the end-of-flags sentinel, etc.
#
# === Why a separate classifier? ===
#
# The parser needs to know what kind of thing each token is before it can
# decide what to do with it. Mixing classification logic into the parser
# creates a tangled mess. Separating them means:
#
#   1. The classifier has exactly one responsibility: classify a token.
#   2. The classifier can be tested exhaustively in isolation.
#   3. The parser can be a clean state machine that reacts to typed events.
#
# === The disambiguation problem ===
#
# Unix flags have an inherent ambiguity. Consider the token "-classpath":
#   - Is it the single_dash_long flag "-classpath" (Java-style)?
#   - Or is it "-c" (short flag) followed by "lasspath" (inline value)?
#   - Or is it "-c", "-l", "-a", "-s", "-s", "-p", "-a", "-t", "-h" (stacked)?
#
# CLI Builder resolves this with **longest-match-first** disambiguation
# (spec §5.2):
#
#   Rule 1: Try to match the entire after-dash string as a single_dash_long flag.
#   Rule 2: Try to match the first character as a short flag.
#     - If boolean: continue processing the rest as a new potential stack.
#     - If non-boolean: the rest is the inline value (or the next token is).
#   Rule 3: Try to treat the whole token as stacked boolean short flags.
#   Rule 4: No match → unknown_flag error.
#
# The "longest match" wins because single_dash_long always takes precedence
# over stacking when there's an exact full match.
#
# === The bare dash special case ===
#
# The token "-" (a single dash with nothing after it) is always a positional
# argument. By Unix convention it means "read from stdin" or "write to stdout".
# Programs like `cat -`, `sort -`, and `diff - file.txt` all rely on this.
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Classifies argv tokens into typed events for the parser's state machine.
    #
    # The classifier is initialized with the set of flags active in the current
    # command scope. It must be reconstructed whenever the scope changes (i.e.,
    # after routing into a subcommand).
    #
    # @example
    #   flags = [{ "id" => "verbose", "short" => "v", "type" => "boolean" },
    #            { "id" => "output", "long" => "output", "type" => "string" }]
    #   c = TokenClassifier.new(flags)
    #   c.classify("--verbose")          # => { type: :long_flag, flag: {...} }
    #   c.classify("--output=file.txt")  # => { type: :long_flag_with_value, flag: {...}, value: "file.txt" }
    #   c.classify("-v")                 # => { type: :short_flag, flag: {...} }
    #   c.classify("file.txt")           # => { type: :positional, value: "file.txt" }
    class TokenClassifier
      # Create a classifier for the given active flag set.
      #
      # @param active_flags [Array<Hash>] Flag definition hashes from the current scope.
      def initialize(active_flags)
        @active_flags = active_flags

        # Build lookup indexes for O(1) classification.
        # These are built once at construction and reused for every classify() call.

        # Map: short character string → flag hash
        @short_index = {}
        # Map: long name string → flag hash
        @long_index = {}
        # Map: single_dash_long string → flag hash
        @sdl_index = {}

        active_flags.each do |f|
          @short_index[f["short"]] = f if f["short"]
          @long_index[f["long"]] = f if f["long"]
          @sdl_index[f["single_dash_long"]] = f if f["single_dash_long"]
        end
      end

      # Classify a single argv token.
      #
      # Returns a hash with at minimum a :type key. Additional keys depend
      # on the type:
      #
      #   { type: :end_of_flags }
      #   { type: :long_flag, flag: flag_hash }
      #   { type: :long_flag_with_value, flag: flag_hash, value: string }
      #   { type: :single_dash_long, flag: flag_hash }
      #   { type: :short_flag, flag: flag_hash }
      #   { type: :short_flag_with_value, flag: flag_hash, value: string }
      #   { type: :stacked_flags, flags: [flag_hash,...], last_value: string_or_nil }
      #   { type: :positional, value: string }
      #   { type: :unknown_flag, token: string }
      #
      # @param token [String] A single argv token to classify.
      # @return [Hash] Classification result.
      def classify(token)
        # The end-of-flags sentinel is exactly two dashes, nothing else.
        # Everything after "--" in argv is a positional argument.
        return {type: :end_of_flags} if token == "--"

        # The bare dash "-" is always positional (stdin/stdout convention).
        return {type: :positional, value: token} if token == "-"

        # Long flags start with "--"
        if token.start_with?("--")
          return classify_long(token)
        end

        # Single-dash tokens (one or more chars after the dash)
        if token.start_with?("-")
          return classify_single_dash(token)
        end

        # No dash prefix → always a positional argument
        {type: :positional, value: token}
      end

      private

      # ---------------------------------------------------------------------------
      # Long flag classification (tokens starting with "--")
      # ---------------------------------------------------------------------------
      #
      # Two sub-cases:
      #   "--name"        → LONG_FLAG(name)
      #   "--name=value"  → LONG_FLAG_WITH_VALUE(name, value)
      #
      # If the name after "--" is not a known long flag, we return unknown_flag.

      def classify_long(token)
        # Strip the leading "--"
        after_dashes = token[2..]

        if (eq_idx = after_dashes.index("="))
          # "--name=value" form
          name = after_dashes[0, eq_idx]
          value = after_dashes[eq_idx + 1..]
          flag = @long_index[name]
          if flag
            {type: :long_flag_with_value, flag: flag, value: value}
          else
            {type: :unknown_flag, token: token}
          end
        else
          # "--name" form
          name = after_dashes
          flag = @long_index[name]
          if flag
            {type: :long_flag, flag: flag}
          else
            {type: :unknown_flag, token: token}
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Single-dash classification (tokens starting with "-" but not "--")
      # ---------------------------------------------------------------------------
      #
      # This is the most complex case because of the three-way ambiguity between
      # single_dash_long, short flags, and stacking.
      #
      # We apply the longest-match-first rules in order:
      #
      #   Rule 1 — Single-dash-long: does the full after-dash string match a SDL flag?
      #   Rule 2 — Single-char short: does the first char match a short flag?
      #   Rule 3 — Stacking: can the whole token be parsed as concatenated booleans?
      #   Rule 4 — No match: unknown_flag

      def classify_single_dash(token)
        # Strip the leading "-". What remains is one or more characters.
        rest = token[1..]

        # --- Rule 1: Single-dash-long exact match ---
        #
        # "-classpath" → single_dash_long("classpath") if "classpath" is known.
        # This takes priority over everything else because it is the longest match.
        if (sdl_flag = @sdl_index[rest])
          return {type: :single_dash_long, flag: sdl_flag}
        end

        # --- Rule 2: Single-character short flag ---
        #
        # Take just the first character after "-". If it matches a known short flag:
        #   - boolean flag: emit SHORT_FLAG, then process the remainder
        #     as a potential stack or inline value
        #   - non-boolean flag: the remainder is the inline value (if non-empty)
        #     or the next token is the value (if empty)
        first_char = rest[0]
        remainder = rest[1..]

        if (short_flag = @short_index[first_char])
          if short_flag["type"] == "boolean"
            # Boolean flag. The remainder might be more stacked flags.
            if remainder.empty?
              # Just "-x" with a single boolean flag
              return {type: :short_flag, flag: short_flag}
            else
              # "-xyz..." — first char is a known boolean. Try to classify rest as stack.
              return classify_as_stack(rest, token)
            end
          else
            # Non-boolean flag. Remainder (if any) is the inline value.
            if remainder.empty?
              # "-f" — value will be the next argv token
              return {type: :short_flag, flag: short_flag}
            else
              # "-ffile.txt" — value is "file.txt"
              return {type: :short_flag_with_value, flag: short_flag, value: remainder}
            end
          end
        end

        # --- Rule 3: Stacking (no short flag matched first char) ---
        #
        # Try to parse the entire after-dash string as concatenated boolean flags.
        stacked = try_stack(rest)
        if stacked
          return stacked
        end

        # --- Rule 4: No match ---
        {type: :unknown_flag, token: token}
      end

      # ---------------------------------------------------------------------------
      # Stack classification helpers
      # ---------------------------------------------------------------------------
      #
      # Stack parsing walks the characters left-to-right:
      #   - Each character must be a known short flag (boolean or non-boolean).
      #   - All characters except possibly the last must be boolean.
      #   - If a non-boolean is found, the remaining characters are its inline value.
      #
      # Example: "-lah" with l=bool, a=bool, h=bool → STACKED_FLAGS([l,a,h], nil)
      # Example: "-lf"  with l=bool, f=file         → STACKED_FLAGS([l], f), value=""
      # Example: "-lfoo" with l=bool, f=file         → STACKED_FLAGS([l,f]), last_value="oo"

      def classify_as_stack(chars, original_token)
        result = try_stack(chars)
        result || {type: :unknown_flag, token: original_token}
      end

      def try_stack(chars)
        collected_flags = []
        i = 0

        while i < chars.length
          ch = chars[i]
          flag = @short_index[ch]

          unless flag
            # Unknown character in what we hoped was a stack
            return nil
          end

          if flag["type"] == "boolean"
            collected_flags << flag
            i += 1
          else
            # Non-boolean flag: rest of the string is its inline value
            last_value = chars[i + 1..]
            collected_flags << flag
            return {type: :stacked_flags, flags: collected_flags, last_value: last_value.empty? ? nil : last_value}
          end
        end

        # All flags were boolean
        {type: :stacked_flags, flags: collected_flags, last_value: nil}
      end
    end
  end
end
