# frozen_string_literal: true

require_relative "errors"

module CodingAdventures
  module CsvParser
    # Parser — the hand-rolled CSV state machine.
    #
    # =========================================================================
    # BACKGROUND: WHY A STATE MACHINE?
    # =========================================================================
    #
    # Most text formats (JSON, SQL, HTML) can be tokenised with a regular
    # expression or a context-free grammar. CSV cannot, because the meaning of
    # a comma depends on *context* — specifically, whether you are currently
    # inside a quoted field.
    #
    # Consider:
    #   Widget,9.99,"A small, round widget"
    #                       ^
    #                       This comma is NOT a field separator.
    #
    # A regex sees all three commas equally. Only by tracking state (am I inside
    # quotes?) can we correctly parse the third field.
    #
    # This is a classic use-case for a finite automaton (state machine). We
    # define four states, and for each state we decide what to do with the
    # current character.
    #
    # =========================================================================
    # THE FOUR STATES
    # =========================================================================
    #
    #  FIELD_START
    #    The parser is about to begin a new field. It looks at the first
    #    character to decide: quoted or unquoted?
    #
    #  IN_UNQUOTED_FIELD
    #    Consuming a plain field character by character. Stop at delimiter,
    #    newline, or end of input.
    #
    #  IN_QUOTED_FIELD
    #    Inside "...". Any character is literal EXCEPT '"', which moves to
    #    IN_QUOTED_MAYBE_END.
    #
    #  IN_QUOTED_MAYBE_END
    #    Just saw '"' inside a quoted field. The next character decides:
    #      '"'  → escaped quote ("") → emit one '"', stay in IN_QUOTED_FIELD
    #      else → end of quoted field
    #
    # =========================================================================
    # STATE TRANSITION DIAGRAM
    # =========================================================================
    #
    #                   ┌──────────────────────────────────────┐
    #                   │            FIELD_START               │◄──────┐
    #                   └────────────────┬─────────────────────┘       │
    #                                    │                             │
    #         ┌──────────────────────────┼──────────────┐             │
    #         │                          │              │             │
    #        '"'                     other char   delimiter/newline    │
    #         │                          │         (empty field)      │
    #         ▼                          ▼               │            │
    #  ┌──────────────────┐  ┌────────────────────┐     └────────────┘
    #  │  IN_QUOTED_FIELD │  │ IN_UNQUOTED_FIELD  │
    #  │                  │  │                    │
    #  │  any non-'"'     │  │ non-special chars  │
    #  │  → append        │  │ → append           │
    #  │                  │  │                    │
    #  │  '"' → move to ──┼─►│ delimiter → end    │
    #  └──────────────────┘  │ newline → end row  │
    #         │              │ EOF → end row       │
    #         ▼              └────────────────────┘
    #  ┌────────────────────────────────┐
    #  │      IN_QUOTED_MAYBE_END       │
    #  └────────────────────────────────┘
    #         │
    #  ┌──────┴──────────────────────────────────┐
    #  │                                         │
    # '"'                             delimiter/newline/EOF
    # (escape "" → emit '"',         (field ends cleanly)
    #  back to IN_QUOTED_FIELD)
    #
    # =========================================================================
    # ESCAPE LOGIC TRUTH TABLE (IN_QUOTED_MAYBE_END)
    # =========================================================================
    #
    #   Next char after '"'  │  Meaning
    #   ─────────────────────┼──────────────────────────────────────────────
    #   '"'                  │  Escape: "" → emit '"', stay in quoted field
    #   delimiter            │  End of quoted field, next field follows
    #   '\n' or '\r'         │  End of quoted field, record ends
    #   EOF                  │  End of quoted field, file ends
    #   anything else        │  Lenient: treat as end of quote
    #
    # =========================================================================
    # NEWLINE NORMALISATION
    # =========================================================================
    #
    # RFC 4180 specifies \r\n as the record terminator. In practice:
    #   - '\n'   — Unix / modern macOS
    #   - '\r\n' — Windows
    #   - '\r'   — old Mac OS 9
    #
    # We treat all three as record terminators.
    # Inside a quoted field, newlines are preserved literally (not normalised).
    #
    class Parser
      # State constants — used as symbols rather than a separate class/enum
      # to keep the Ruby implementation idiomatic.
      #
      # Ruby does not have a built-in Enum type. We use frozen symbols as an
      # alternative: they are lightweight, human-readable, and safe for ==
      # comparison.
      FIELD_START = :field_start
      IN_UNQUOTED_FIELD = :in_unquoted_field
      IN_QUOTED_FIELD = :in_quoted_field
      IN_QUOTED_MAYBE_END = :in_quoted_maybe_end

      # Parse CSV source text into an array of raw rows (each row is an array
      # of String field values).
      #
      # @param source    [String] the CSV text
      # @param delimiter [String] single character used as field separator
      # @return [Array<Array<String>>] list of rows, each row a list of strings
      # @raise [UnclosedQuoteError] if a quoted field is never closed
      def self.scan(source, delimiter)
        new(source, delimiter).scan
      end

      # -----------------------------------------------------------------------
      # Constructor — set up internal state
      # -----------------------------------------------------------------------
      def initialize(source, delimiter)
        # We append a sentinel newline so the final record is always flushed by
        # the main loop without special EOF handling. The sentinel is a real "\n"
        # character, which means the IN_UNQUOTED_FIELD and IN_QUOTED_MAYBE_END
        # cases that handle '\n' will naturally close the last record.
        @chars = (source + "\n").chars
        @delimiter = delimiter

        # Parser state
        @state = FIELD_START
        @current_field = []   # characters accumulating for the current field
        @current_row = []     # fields accumulating for the current row
        @all_rows = []        # completed rows
      end

      # -----------------------------------------------------------------------
      # Main scanning method — drives the state machine
      # -----------------------------------------------------------------------
      def scan
        i = 0
        n = @chars.length

        while i < n
          ch = @chars[i]

          case @state

          # ── FIELD_START ──────────────────────────────────────────────────
          # We are at the beginning of a new field.
          when FIELD_START
            if ch == '"'
              # Opening quote → enter quoted field mode.
              # The '"' itself is NOT included in the field value.
              @state = IN_QUOTED_FIELD
              i += 1

            elsif ch == @delimiter
              # Delimiter immediately = empty unquoted field.
              # e.g., a,,b → middle field is ''
              @current_row << ""
              # Stay in FIELD_START for the next field.
              i += 1

            elsif newline?(ch)
              # Newline immediately = empty field + end of row.
              # This handles trailing delimiters and blank lines.
              @current_row << ""
              finish_row
              i = consume_newline(i)

            else
              # Any other character starts an unquoted field.
              @state = IN_UNQUOTED_FIELD
              @current_field << ch
              i += 1
            end

          # ── IN_UNQUOTED_FIELD ─────────────────────────────────────────────
          # Consuming a plain field. Stop at delimiter, newline, or EOF.
          when IN_UNQUOTED_FIELD
            if ch == @delimiter
              # End of field; another field follows on the same row.
              @current_row << @current_field.join
              @current_field = []
              @state = FIELD_START
              i += 1

            elsif newline?(ch)
              # End of field AND end of row.
              @current_row << @current_field.join
              @current_field = []
              finish_row
              @state = FIELD_START
              i = consume_newline(i)

            else
              # Regular character: accumulate.
              @current_field << ch
              i += 1
            end

          # ── IN_QUOTED_FIELD ───────────────────────────────────────────────
          # Inside a "..." field. Any character is literal EXCEPT '"'.
          when IN_QUOTED_FIELD
            if ch == '"'
              # Could be end-of-field or start of "" escape sequence.
              # Transition to the "maybe end" state — we'll decide on next char.
              @state = IN_QUOTED_MAYBE_END
              i += 1
            else
              # Literal character — including delimiter and newline inside quotes.
              @current_field << ch
              i += 1
            end

          # ── IN_QUOTED_MAYBE_END ───────────────────────────────────────────
          # Just saw '"' inside a quoted field.
          #
          # Truth table:
          #   next '"'        → "" escape → emit '"', back to IN_QUOTED_FIELD
          #   delimiter       → field ends cleanly
          #   newline         → field ends, row ends
          #   EOF (sentinel)  → field ends, file ends
          #   anything else   → lenient end-of-quote; re-process char
          when IN_QUOTED_MAYBE_END
            if ch == '"'
              # Escape sequence "" → one literal '"' in output
              @current_field << '"'
              @state = IN_QUOTED_FIELD
              i += 1

            elsif ch == @delimiter
              # Quoted field ends; next field follows.
              @current_row << @current_field.join
              @current_field = []
              @state = FIELD_START
              i += 1

            elsif newline?(ch)
              # Quoted field ends; record ends.
              @current_row << @current_field.join
              @current_field = []
              finish_row
              @state = FIELD_START
              i = consume_newline(i)

            else
              # Unexpected character after closing '"'. Be lenient: treat the '"'
              # as the end of the quoted portion, then continue as unquoted.
              # Do NOT advance i — re-process this character in the next iteration.
              @state = IN_UNQUOTED_FIELD
            end

          end # case @state
        end # while

        # ── Post-loop: check for unclosed quoted field ─────────────────────
        # If the loop exited while we're still in IN_QUOTED_FIELD, the quote
        # was never closed. Our sentinel '\n' should have triggered
        # IN_QUOTED_MAYBE_END → field-end for a properly closed field.
        if @state == IN_QUOTED_FIELD
          raise UnclosedQuoteError,
            "Unclosed quoted field at end of input. " \
            "A field was opened with '\"' but the matching closing '\"' was never found."
        end

        # Safety flush: the sentinel should have taken care of everything, but
        # just in case there's a stray field or row:
        if @current_field.any? || @current_row.any?
          @current_row << @current_field.join unless @current_field.empty?
          finish_row unless @current_row.empty?
        end

        @all_rows
      end

      private

      # Append @current_row to @all_rows, then reset it.
      # Skips the artefact row [""] produced by a trailing newline in the source
      # (the sentinel '\n' can create one of these).
      def finish_row
        @all_rows << @current_row unless @current_row == [""]
        @current_row = []
      end

      # Return true if the character is a newline of any style.
      #   '\n' — Unix / modern macOS
      #   '\r' — old Mac, also the first byte of '\r\n'
      def newline?(ch)
        ch == "\n" || ch == "\r"
      end

      # Advance past a newline, consuming '\r\n' as a single terminator.
      # Returns the new index.
      def consume_newline(i)
        ch = @chars[i]
        if ch == "\r" && i + 1 < @chars.length && @chars[i + 1] == "\n"
          i + 2  # skip both '\r' and '\n'
        else
          i + 1  # skip just the '\n' or '\r'
        end
      end
    end
  end
end
