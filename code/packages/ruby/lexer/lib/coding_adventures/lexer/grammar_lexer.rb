# frozen_string_literal: true

require "coding_adventures_grammar_tools"

# ==========================================================================
# GrammarLexer -- Grammar-Driven Tokenization from .tokens Files
# ==========================================================================
#
# Instead of hardcoded character-dispatching logic, this lexer reads token
# definitions from a .tokens file (parsed by grammar_tools) and uses those
# definitions to drive tokenization at runtime.
#
# How it works:
#
# 1. Compile each token definition into a Regexp. Literal patterns are
#    escaped so that + and * are treated as literal characters.
#
# 2. At each position in the source code, try each compiled pattern in
#    order (first match wins).
#
# 3. Emit a Token with the matched type and value.
#
# Extended features (Starlark support):
#
# - Skip patterns: matched and consumed without producing tokens.
# - Type aliases: a definition with alias_name emits that alias type.
# - Reserved keywords: identifiers matching reserved words raise errors.
# - Indentation mode: Python-style INDENT/DEDENT/NEWLINE tracking.
#
# Pattern Groups and On-Token Callbacks
# --------------------------------------
#
# Pattern groups enable context-sensitive lexing. A grammar can define
# named groups of token patterns (e.g., "tag" for XML attribute patterns).
# The lexer maintains a stack of active groups -- only the patterns from
# the group on top of the stack are tried during token matching.
#
# An on-token callback controls group transitions. When registered via
# +set_on_token+, the callback fires after each token match (before
# emission) and receives a LexerContext object. Through the context,
# the callback can:
#
# - Push/pop pattern groups (context-sensitive lexing)
# - Emit synthetic tokens (token injection)
# - Suppress the current token (token filtering)
# - Toggle skip pattern processing (significant whitespace)
# - Peek ahead in the source text
# - Inspect the previous token (lookbehind)
# - Query bracket nesting depth
# - Test whether a newline preceded the current token
#
# Because both lexers produce identical Token objects, downstream consumers
# (the parser) don't care which lexer generated the tokens.
# ==========================================================================

module CodingAdventures
  module Lexer
    # ========================================================================
    # LexerContext -- Callback Interface for Group Transitions
    # ========================================================================
    #
    # When a callback is registered via GrammarLexer#set_on_token, it
    # receives a LexerContext on every token match. The context provides
    # controlled access to the group stack, token emission, and skip control.
    #
    # Methods that modify state (push/pop/emit/suppress) take effect after
    # the callback returns -- they do not interrupt the current match.
    #
    # Example -- XML lexer callback:
    #
    #   lexer.set_on_token(proc { |token, ctx|
    #     if token.type == "OPEN_TAG_START"
    #       ctx.push_group("tag")
    #     elsif ["TAG_CLOSE", "SELF_CLOSE"].include?(token.type)
    #       ctx.pop_group
    #     end
    #   })
    #
    # Design notes:
    #
    # - Actions are buffered, not immediate. This prevents surprising
    #   mid-match mutations. The tokenizer's main loop applies them after
    #   the callback returns, in the order they were recorded.
    #
    # - The callback is NOT invoked for skip matches, emitted tokens, or
    #   the EOF token. This prevents infinite loops (emitted tokens don't
    #   re-trigger the callback) and avoids noisy callbacks for whitespace.
    # ========================================================================
    class LexerContext
      # @param lexer [GrammarLexer] the lexer instance (for reading state)
      # @param source [String] the full source code being tokenized
      # @param pos_after_token [Integer] position in source after current token
      # @param previous_token [Token, nil] the most recently emitted token
      # @param current_token_line [Integer] the current token's line number
      def initialize(lexer, source, pos_after_token, previous_token = nil, current_token_line = 1)
        @lexer = lexer
        @source = source
        @pos_after = pos_after_token
        @previous_token = previous_token
        @current_token_line = current_token_line

        # Buffered actions -- applied by the tokenizer after callback returns.
        @suppressed = false
        @emitted = []
        @group_actions = []
        @skip_enabled = nil # nil = no change requested
      end

      # These readers allow the tokenizer to inspect buffered actions.
      attr_reader :suppressed, :emitted, :group_actions, :skip_enabled

      # Push a pattern group onto the stack.
      #
      # The pushed group becomes active for the next token match.
      # Raises ArgumentError if the group name is not defined in the grammar.
      #
      # @param group_name [String] name of the group to push
      def push_group(group_name)
        unless @lexer.group_patterns.key?(group_name)
          available = @lexer.group_patterns.keys.sort
          raise ArgumentError,
            "Unknown pattern group: #{group_name.inspect}. " \
            "Available groups: #{available}"
        end
        @group_actions << [:push, group_name]
      end

      # Pop the current group from the stack.
      #
      # If only the default group remains, this is a no-op. The default
      # group is the floor and cannot be popped.
      def pop_group
        @group_actions << [:pop, ""]
      end

      # Return the name of the currently active group.
      #
      # @return [String] the group name at the top of the stack
      def active_group
        @lexer.group_stack.last
      end

      # Return the depth of the group stack (always >= 1).
      #
      # @return [Integer] the stack depth
      def group_stack_depth
        @lexer.group_stack.length
      end

      # Inject a synthetic token after the current one.
      #
      # Emitted tokens do NOT trigger the callback (prevents infinite
      # loops). Multiple emit calls produce tokens in call order.
      #
      # @param token [Token] the synthetic token to inject
      def emit(token)
        @emitted << token
      end

      # Suppress the current token -- do not include it in output.
      def suppress
        @suppressed = true
      end

      # Peek at a source character past the current token.
      #
      # @param offset [Integer] characters ahead (1 = immediately after token)
      # @return [String] the character, or "" if past EOF
      def peek(offset = 1)
        idx = @pos_after + offset - 1
        if idx >= 0 && idx < @source.length
          @source[idx]
        else
          ""
        end
      end

      # Peek at the next +length+ characters past the current token.
      #
      # @param length [Integer] number of characters to read
      # @return [String] the substring (may be shorter than length near EOF)
      def peek_str(length)
        @source[@pos_after, length] || ""
      end

      # Toggle skip pattern processing.
      #
      # When disabled, skip patterns (whitespace, comments) are not tried.
      # Useful for groups where whitespace is significant (e.g., CDATA).
      #
      # @param enabled [Boolean] whether skip patterns should be active
      def set_skip_enabled(enabled)
        @skip_enabled = enabled
      end

      # -----------------------------------------------------------------------
      # Extension: Token Lookbehind
      # -----------------------------------------------------------------------

      # Return the most recently emitted token, or nil at the start of input.
      #
      # "Emitted" means the token actually made it into the output list --
      # suppressed tokens are not counted. This provides lookbehind
      # capability for context-sensitive decisions.
      #
      # For example, in JavaScript / is a regex literal after =, ( or ,
      # but a division operator after ), ], identifiers, or numbers. The
      # callback can check ctx.previous_token&.type to decide which
      # interpretation to use.
      #
      # @return [Token, nil] the last emitted token, or nil if none yet
      def previous_token
        @previous_token
      end

      # -----------------------------------------------------------------------
      # Extension: Bracket Depth Tracking
      # -----------------------------------------------------------------------

      # Return the current nesting depth for a specific bracket type,
      # or the total depth across all types if no argument is given.
      #
      # Depth starts at 0 and increments on each opener ((, [, {),
      # decrements on each closer (), ], }). The count never goes
      # below 0 -- unmatched closers are clamped.
      #
      # This is essential for template literal interpolation in languages
      # like JavaScript, Kotlin, and Ruby, where } at brace-depth 0
      # closes the interpolation rather than being part of a nested
      # expression.
      #
      # @param kind [Symbol, nil] :paren, :bracket, or :brace. nil for total.
      # @return [Integer] the nesting depth
      def bracket_depth(kind = nil)
        @lexer.bracket_depth(kind)
      end

      # -----------------------------------------------------------------------
      # Extension: Newline Detection
      # -----------------------------------------------------------------------

      # Return true if a newline appeared between the previous token
      # and the current token (i.e., they are on different lines).
      #
      # This is used by languages with automatic semicolon insertion
      # (JavaScript, Go) to detect line breaks that trigger implicit
      # statement termination. The lexer exposes this as a convenience
      # so callbacks and post-tokenize hooks can set the
      # TOKEN_PRECEDED_BY_NEWLINE flag on tokens that need it.
      #
      # Returns false if there is no previous token (start of input).
      #
      # @return [Boolean]
      def preceded_by_newline?
        return false if @previous_token.nil?
        @previous_token.line < @current_token_line
      end
    end

    # ========================================================================
    # GrammarLexer -- The Grammar-Driven Lexer
    # ========================================================================
    class GrammarLexer
      # Expose group_patterns and group_stack for LexerContext to read.
      # These are internal state -- not part of the public API for callers,
      # but needed by LexerContext which acts as a controlled window into
      # the lexer's state.
      attr_reader :group_patterns, :group_stack

      # Allow tests to reset source/position for re-tokenization.
      attr_writer :source, :pos, :line, :column

      # @param source [String] the raw source code to tokenize
      # @param grammar [CodingAdventures::GrammarTools::TokenGrammar]
      def initialize(source, grammar)
        # Preserve the original source so that string literal values can
        # retain their case even when the grammar is case-insensitive.
        # For example, 'Alice' in a SQL INSERT should tokenize as
        # STRING("Alice"), not STRING("alice"), even though SQL keywords
        # like SELECT and FROM are compared case-insensitively.
        @original_source = source

        # Case sensitivity: when the grammar is case-insensitive, we
        # lowercase the working copy of the source before tokenization.
        # This means all pattern matching happens against lowercase text,
        # ensuring keywords are matched regardless of their case in the
        # input. Keyword promotion works automatically because both the
        # lowercased source and the (uppercased) keyword set are in a
        # consistent case. Used by case-insensitive languages like VHDL
        # and SQL.
        # NOTE: @source is only used for pattern matching. String literal
        # values are extracted from @original_source to preserve case.
        @source = grammar.case_sensitive ? source : source.downcase
        @grammar = grammar
        @pos = 0
        @line = 1
        @column = 1
        # Case-insensitive keyword matching.
        #
        # When the grammar declares `# @case_insensitive true`, the lexer
        # normalises every keyword to uppercase at compile time and then
        # compares incoming NAME values against that uppercase set. This
        # means "select", "SELECT", and "Select" all resolve to KEYWORD
        # with the normalised value "SELECT".
        #
        # Default is false — keywords are compared exactly as written in
        # the .tokens file, preserving the original case-sensitive behaviour.
        @case_insensitive = grammar.case_insensitive

        if @case_insensitive
          @keyword_set = grammar.keywords.map(&:upcase).to_set.freeze
          @reserved_set = grammar.reserved_keywords.map(&:upcase).to_set.freeze
        else
          @keyword_set = grammar.keywords.to_set.freeze
          @reserved_set = grammar.reserved_keywords.to_set.freeze
        end

        # Context-sensitive keywords -- words that are sometimes keywords
        # and sometimes identifiers depending on syntactic position. These
        # are emitted as NAME tokens with the TOKEN_CONTEXT_KEYWORD flag,
        # leaving the final keyword-vs-identifier decision to the parser.
        @context_keyword_set = (grammar.context_keywords || []).to_set.freeze

        # Compile token patterns into Regexp objects.
        # Each entry is [name, pattern, alias_name] -- the alias is used
        # when emitting the token type (e.g., STRING_DQ -> STRING).
        @patterns = grammar.definitions.map do |defn|
          pattern = if defn.is_regex
            Regexp.new(defn.pattern)
          else
            Regexp.new(Regexp.escape(defn.pattern))
          end
          [defn.name, pattern, defn.alias_name]
        end

        # Compile skip patterns.
        @skip_patterns = grammar.skip_definitions.map do |defn|
          if defn.is_regex
            Regexp.new(defn.pattern)
          else
            Regexp.new(Regexp.escape(defn.pattern))
          end
        end

        # Whether the grammar has skip patterns. When skip patterns exist,
        # they replace the default whitespace-skipping behavior.
        @has_skip_patterns = !grammar.skip_definitions.empty?

        # Case sensitivity mode. When false, the lexer lowercases input
        # before matching and promotes NAME -> KEYWORD for lowercased values
        # that match keywords. Used by case-insensitive languages like VHDL.
        @case_sensitive = grammar.case_sensitive

        # Lexer modes.
        @indentation_mode = grammar.mode == "indentation"
        @layout_mode = grammar.mode == "layout"
        @indent_stack = [0]
        @bracket_depth_indent = 0
        @layout_keyword_set = (grammar.respond_to?(:layout_keywords) ? grammar.layout_keywords : []).to_set.freeze

        # --- Pattern groups ---
        # Compile per-group patterns. The "default" group uses the
        # top-level definitions. Named groups use their own definitions.
        # When no groups are defined, @group_patterns has only "default".
        @group_patterns = {
          "default" => @patterns.dup
        }
        grammar.groups.each do |group_name, group|
          compiled = group.definitions.map do |defn|
            pat = if defn.is_regex
              Regexp.new(defn.pattern)
            else
              Regexp.new(Regexp.escape(defn.pattern))
            end
            [defn.name, pat, defn.alias_name]
          end
          @group_patterns[group_name] = compiled
        end

        # The group stack. Bottom is always "default". Top is the active
        # group whose patterns are tried during token matching.
        @group_stack = ["default"]

        # On-token callback -- nil means no callback (zero overhead).
        # When set, fires after each token match, before emission.
        @on_token = nil

        # Skip enabled flag -- can be toggled by callbacks for groups
        # where whitespace is significant (e.g., CDATA, raw content).
        @skip_enabled = true

        # Extension: Token lookbehind -- the most recently emitted token.
        # Updated after each token push (including callback-emitted tokens).
        # Reset to nil on each tokenize() call.
        @last_emitted_token = nil

        # Extension: Bracket depth tracking -- per-type nesting counters.
        # Tracks (), [], and {} independently. Updated after each token
        # match in both standard and indentation modes. Exposed to
        # callbacks via LexerContext#bracket_depth.
        @bracket_depths = { paren: 0, bracket: 0, brace: 0 }

        # Transform hooks — pluggable pipeline stages for language-specific
        # processing. Hooks compose left-to-right.
        @pre_tokenize_hooks = []
        @post_tokenize_hooks = []
      end

      # Register a callback that fires on every token match.
      #
      # The callback receives the matched token and a LexerContext.
      # It can use the context to push/pop groups, emit extra tokens,
      # or suppress the current token.
      #
      # Only one callback can be registered. Pass nil to clear.
      #
      # The callback is NOT invoked for:
      # - Skip pattern matches (they produce no tokens)
      # - Tokens emitted via context.emit (prevents infinite loops)
      # - The EOF token
      #
      # @param callback [Proc, nil] the callback, or nil to clear
      def set_on_token(callback)
        @on_token = callback
      end

      # Register a text transform to run before tokenization.
      # The hook receives the raw source string and returns a (possibly
      # modified) source string. Multiple hooks compose left-to-right.
      def add_pre_tokenize(hook)
        @pre_tokenize_hooks << hook
      end

      # Register a token transform to run after tokenization.
      # The hook receives the full token list and returns a (possibly
      # modified) token list. Multiple hooks compose left-to-right.
      def add_post_tokenize(hook)
        @post_tokenize_hooks << hook
      end

      # Return the current nesting depth for a specific bracket type,
      # or the total depth across all types if no argument is given.
      #
      # This is the public API used by LexerContext to expose bracket
      # depth to callbacks. Language packages use this for template
      # literal interpolation and similar nested constructs.
      #
      # @param kind [Symbol, nil] :paren, :bracket, or :brace. nil for total.
      # @return [Integer]
      def bracket_depth(kind = nil)
        if kind.nil?
          @bracket_depths[:paren] + @bracket_depths[:bracket] + @bracket_depths[:brace]
        else
          @bracket_depths[kind]
        end
      end

      # Tokenize the source code using the grammar's token definitions.
      #
      # Dispatches to the appropriate tokenization method based on whether
      # indentation mode is active.
      #
      # @return [Array<Token>] list of tokens, always ending with EOF
      def tokenize
        # Stage 1: Pre-tokenize hooks transform the source text.
        unless @pre_tokenize_hooks.empty?
          source = @source
          @pre_tokenize_hooks.each { |hook| source = hook.call(source) }
          @source = source
        end

        # Reset extension state for reuse.
        @last_emitted_token = nil
        @bracket_depths = { paren: 0, bracket: 0, brace: 0 }

        # Stage 2: Core tokenization.
        tokens = if @indentation_mode
          tokenize_indentation
        elsif @layout_mode
          tokenize_layout
        else
          tokenize_standard
        end

        # Stage 3: Post-tokenize hooks transform the token list.
        @post_tokenize_hooks.each { |hook| tokens = hook.call(tokens) }

        tokens
      end

      private

      # Standard tokenization (no indentation tracking).
      #
      # The algorithm:
      #
      # 1. While there are characters left:
      #    a. If skip patterns exist and skip is enabled, try them.
      #    b. If no skip patterns, use default whitespace skip.
      #    c. If the current character is a newline, emit NEWLINE.
      #    d. Try active group's token patterns (first match wins).
      #    e. If callback registered, invoke it and process actions.
      #    f. If nothing matches, raise LexerError.
      # 2. Append EOF.
      #
      # When pattern groups are active, the lexer uses @group_stack.last
      # to determine which set of patterns to try. When a callback is
      # registered via set_on_token, it fires after each token match
      # and can push/pop groups, emit extra tokens, or suppress the
      # current token.
      def tokenize_standard
        tokens = []

        while @pos < @source.length
          char = @source[@pos]

          # --- Skip patterns (grammar-defined) ---
          # When the grammar has skip patterns AND skip is enabled, they
          # take over whitespace handling. The callback can disable skip
          # processing for groups where whitespace is significant.
          if @has_skip_patterns
            if @skip_enabled && try_skip
              next
            end
          elsif char == " " || char == "\t" || char == "\r"
            # --- Default whitespace skip ---
            # Without skip patterns, use the hardcoded behavior: skip
            # spaces, tabs, carriage returns silently.
            do_advance
            next
          end

          # --- Newlines become NEWLINE tokens ---
          # Newlines are structural -- they mark line boundaries.
          if char == "\n"
            newline_tok = Token.new(
              type: TokenType::NEWLINE, value: "\\n",
              line: @line, column: @column
            )
            tokens << newline_tok
            @last_emitted_token = newline_tok
            do_advance
            next
          end

          # --- Try active group's token patterns (first match wins) ---
          # The active group is the top of the group stack. When no
          # groups are defined, this is always "default" (the top-level
          # definitions), preserving backward compatibility.
          active_group = @group_stack.last
          token = try_match_token_in_group(active_group)
          if token
            # Update bracket depth tracking.
            update_bracket_depth(token.value)

            # --- Invoke on-token callback ---
            # The callback can push/pop groups, emit extra tokens,
            # suppress the current token, or toggle skip processing.
            # Emitted tokens do NOT re-trigger the callback.
            if @on_token
              ctx = LexerContext.new(
                self, @source, @pos,
                @last_emitted_token, token.line
              )
              @on_token.call(token, ctx)

              # Apply suppression: if the callback suppressed this
              # token, don't add it to the output.
              unless ctx.suppressed
                tokens << token
                @last_emitted_token = token
              end

              # Append any tokens emitted by the callback.
              ctx.emitted.each do |emitted|
                tokens << emitted
                @last_emitted_token = emitted
              end

              # Apply group stack actions in order.
              ctx.group_actions.each do |action, group_name|
                if action == :push
                  @group_stack.push(group_name)
                elsif action == :pop && @group_stack.length > 1
                  @group_stack.pop
                end
              end

              # Apply skip toggle if the callback changed it.
              @skip_enabled = ctx.skip_enabled unless ctx.skip_enabled.nil?
            else
              tokens << token
              @last_emitted_token = token
            end
            next
          end

          raise LexerError.new(
            "Unexpected character: #{char.inspect}",
            line: @line, column: @column
          )
        end

        # --- Append EOF sentinel ---
        tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        # Reset group stack for reuse (in case tokenize is called again).
        @group_stack = ["default"]
        @skip_enabled = true

        tokens
      end

      # Update bracket depth counters based on a token's value.
      #
      # Called after each token match in both standard and indentation modes.
      # Only single-character values are checked -- multi-character tokens
      # cannot be brackets.
      def update_bracket_depth(value)
        return unless value.length == 1
        case value
        when "(" then @bracket_depths[:paren] += 1
        when ")" then @bracket_depths[:paren] -= 1 if @bracket_depths[:paren] > 0
        when "[" then @bracket_depths[:bracket] += 1
        when "]" then @bracket_depths[:bracket] -= 1 if @bracket_depths[:bracket] > 0
        when "{" then @bracket_depths[:brace] += 1
        when "}" then @bracket_depths[:brace] -= 1 if @bracket_depths[:brace] > 0
        end
      end

      # Indentation-aware tokenization (Python/Starlark style).
      #
      # At each logical line start, count leading spaces, compare to the
      # indent stack, and emit INDENT or DEDENT tokens. Inside brackets
      # (parens, brackets, braces), suppress NEWLINE/INDENT/DEDENT.
      def tokenize_indentation
        tokens = []
        at_line_start = true

        while @pos < @source.length
          char = @source[@pos]

          # Process line start: count indentation, emit INDENT/DEDENT.
          if at_line_start && @bracket_depth_indent == 0
            indent_tokens = process_line_start
            if indent_tokens == :skip_line
              # Blank or comment-only line -- skip entirely.
              next
            end
            tokens.concat(indent_tokens) if indent_tokens
            at_line_start = false
            next if @pos >= @source.length
            char = @source[@pos]
          end

          # Newline handling.
          if char == "\n"
            if @bracket_depth_indent == 0
              tokens << Token.new(
                type: TokenType::NEWLINE, value: "\\n",
                line: @line, column: @column
              )
            end
            do_advance
            at_line_start = true
            next
          end

          # Inside brackets: skip whitespace (implicit line joining).
          if @bracket_depth_indent > 0 && (char == " " || char == "\t" || char == "\r")
            do_advance
            next
          end

          # Try skip patterns.
          next if try_skip

          # Try each token pattern.
          token = try_match_token
          if token
            # Track bracket depth (local for INDENT/DEDENT logic).
            case token.value
            when "(", "[", "{"
              @bracket_depth_indent += 1
            when ")", "]", "}"
              @bracket_depth_indent -= 1
            end
            # Track bracket depth (shared for callback access).
            update_bracket_depth(token.value)
            tokens << token
            @last_emitted_token = token
            next
          end

          raise LexerError.new(
            "Unexpected character: #{char.inspect}",
            line: @line, column: @column
          )
        end

        # EOF: emit remaining DEDENTs.
        while @indent_stack.length > 1
          @indent_stack.pop
          tokens << Token.new(
            type: "DEDENT", value: "",
            line: @line, column: @column
          )
        end

        # Final NEWLINE if the last token isn't one.
        if tokens.empty? || tokens.last.type != TokenType::NEWLINE
          tokens << Token.new(
            type: TokenType::NEWLINE, value: "\\n",
            line: @line, column: @column
          )
        end

        tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        tokens
      end

      # Layout-aware tokenization (prototype for Haskell-style offside rules).
      #
      # We intentionally reuse the standard tokenization pass first so
      # callbacks, pattern groups, and skip handling all continue to work.
      # A second pass injects virtual "{", ";", and "}" tokens after
      # layout introducers declared in the grammar's layout_keywords: section.
      #
      # The current prototype is intentionally narrow:
      # - basic let/where/do/of-style blocks
      # - explicit "{" cancels implicit layout for that introducer
      # - dedent and EOF close open layout contexts
      #
      # This is enough to validate the engine shape before we tackle the
      # full Haskell report.
      def tokenize_layout
        apply_layout(tokenize_standard)
      end

      # Process the start of a logical line in indentation mode.
      # Returns an array of INDENT/DEDENT tokens, :skip_line for blank/comment
      # lines, or nil if no indent change.
      def process_line_start
        # Count leading spaces. Tabs are not allowed in indentation.
        indent = 0
        while @pos < @source.length
          char = @source[@pos]
          if char == " "
            indent += 1
            do_advance
          elsif char == "\t"
            raise LexerError.new(
              "Tab character in indentation (use spaces only)",
              line: @line, column: @column
            )
          else
            break
          end
        end

        # Blank line or end of file -- skip without emitting NEWLINE.
        return :skip_line if @pos >= @source.length
        if @source[@pos] == "\n"
          do_advance  # Consume the newline to avoid infinite loop.
          return :skip_line
        end

        # Comment-only line -- consume the comment via skip patterns,
        # then check if we're at newline/EOF.
        remaining = @source[@pos..]
        @skip_patterns.each do |pattern|
          m = pattern.match(remaining)
          if m && m.begin(0) == 0
            # Check if after skipping we're at newline or EOF.
            peek_pos = @pos + m[0].length
            if peek_pos >= @source.length || @source[peek_pos] == "\n"
              m[0].length.times { do_advance }
              do_advance if @pos < @source.length && @source[@pos] == "\n"
              return :skip_line
            end
          end
        end

        # Compare indent to current level.
        current_indent = @indent_stack.last
        tokens = []

        if indent > current_indent
          @indent_stack.push(indent)
          tokens << Token.new(
            type: "INDENT", value: "",
            line: @line, column: 1
          )
        elsif indent < current_indent
          while @indent_stack.length > 1 && @indent_stack.last > indent
            @indent_stack.pop
            tokens << Token.new(
              type: "DEDENT", value: "",
              line: @line, column: 1
            )
          end
          unless @indent_stack.last == indent
            raise LexerError.new(
              "Inconsistent dedent (indent level #{indent} does not match any outer level)",
              line: @line, column: 1
            )
          end
        end

        tokens.empty? ? nil : tokens
      end

      # Try to match and consume a skip pattern at the current position.
      # Returns true if something was skipped.
      def try_skip
        remaining = @source[@pos..]
        @skip_patterns.each do |pattern|
          m = pattern.match(remaining)
          if m && m.begin(0) == 0
            m[0].length.times { do_advance }
            return true
          end
        end
        false
      end

      def apply_layout(tokens)
        result = []
        layout_stack = []
        pending_layouts = 0
        suppress_depth = 0

        tokens.each_with_index do |token, index|
          type_name = token.type_name

          if type_name == "NEWLINE"
            result << token
            next_token = next_layout_token(tokens, index + 1)
            if suppress_depth == 0 && next_token
              while !layout_stack.empty? && next_token.column < layout_stack.last
                result << virtual_layout_token("VIRTUAL_RBRACE", "}", next_token)
                layout_stack.pop
              end

              if !layout_stack.empty? &&
                  next_token.type_name != "EOF" &&
                  next_token.value != "}" &&
                  next_token.column == layout_stack.last
                result << virtual_layout_token("VIRTUAL_SEMICOLON", ";", next_token)
              end
            end
            next
          end

          if type_name == "EOF"
            while !layout_stack.empty?
              result << virtual_layout_token("VIRTUAL_RBRACE", "}", token)
              layout_stack.pop
            end
            result << token
            next
          end

          if pending_layouts.positive?
            if token.value == "{"
              pending_layouts -= 1
            else
              pending_layouts.times do
                layout_stack << token.column
                result << virtual_layout_token("VIRTUAL_LBRACE", "{", token)
              end
              pending_layouts = 0
            end
          end

          result << token

          unless virtual_layout_token?(token)
            case token.value
            when "(", "[", "{"
              suppress_depth += 1
            when ")", "]", "}"
              suppress_depth -= 1 if suppress_depth > 0
            end
          end

          pending_layouts += 1 if layout_keyword?(token)
        end

        result
      end

      def next_layout_token(tokens, start_index)
        idx = start_index
        while idx < tokens.length
          token = tokens[idx]
          return token unless token.type_name == "NEWLINE"
          idx += 1
        end
        nil
      end

      def virtual_layout_token(type_name, value, anchor)
        Token.new(
          type: type_name,
          value: value,
          line: anchor.line,
          column: anchor.column
        )
      end

      def virtual_layout_token?(token)
        token.type_name.start_with?("VIRTUAL_")
      end

      def layout_keyword?(token)
        return false if @layout_keyword_set.empty?

        value = token.value.to_s
        @layout_keyword_set.include?(value) || @layout_keyword_set.include?(value.downcase)
      end

      # Try to match a token using the default group's patterns.
      #
      # This is the original method used by the indentation tokenizer,
      # which does not support pattern groups. It delegates to
      # try_match_token_in_group("default").
      #
      # @return [Token, nil] a Token if matched, nil otherwise
      def try_match_token
        try_match_token_in_group("default")
      end

      # Try to match a token from a specific pattern group.
      #
      # Tries each compiled pattern in the named group in priority order
      # (first match wins). Handles keyword detection, reserved word
      # checking, aliases, context keywords, and string escape processing.
      #
      # @param group_name [String] the pattern group to use
      # @return [Token, nil] a Token if matched, nil otherwise
      def try_match_token_in_group(group_name)
        remaining = @source[@pos..]
        patterns = @group_patterns.fetch(group_name, @patterns)

        patterns.each do |token_name, pattern, alias_name|
          m = pattern.match(remaining)
          next unless m && m.begin(0) == 0

          value = m[0]
          start_line = @line
          start_column = @column

          token_type = resolve_token_type(token_name, value, alias_name)

          # Case-insensitive keyword normalisation.
          #
          # When the grammar is case-insensitive and a NAME matched a
          # keyword, normalise the emitted value to uppercase so that
          # "select", "SELECT", and "Select" all produce the same token.
          # Non-keyword NAMEs retain their original casing.
          if @case_insensitive && token_type == TokenType::KEYWORD
            value = value.upcase
          end

          # Handle STRING tokens: strip quotes and process escapes.
          # When escape_mode is "none", we strip quotes but leave escape
          # sequences as raw text. This is used by CSS and TOML where
          # escape semantics differ from JSON and are handled in the
          # semantic layer.
          #
          # IMPORTANT: use the ORIGINAL source (not the lowercased working
          # copy) to extract the string body. This preserves the case of
          # string literal values like 'Alice' even in case-insensitive
          # grammars (e.g. SQL). Pattern matching uses the lowercased source
          # to locate the token; value extraction uses the original source.
          if token_name == "STRING" || (alias_name && alias_name.include?("STRING"))
            # Only strip if the value is quoted.
            original_value = @original_source[@pos, value.length]
            if original_value && original_value.length >= 2 && original_value.start_with?('"', "'")
              inner = original_value[1..-2]
              inner = process_escapes(inner) unless @grammar.escape_mode == "none"
              value = inner
            end
          end

          # Check if this NAME token is a context keyword -- a word that
          # is sometimes a keyword and sometimes an identifier depending
          # on syntactic position. Context keywords are emitted as NAME
          # with the TOKEN_CONTEXT_KEYWORD flag, leaving the final
          # decision to the language-specific parser or callback.
          flags = nil
          if token_type_is_name?(token_type) &&
              !@context_keyword_set.empty? &&
              @context_keyword_set.include?(value)
            flags = TOKEN_CONTEXT_KEYWORD
          end

          result = Token.new(
            type: token_type, value: value,
            line: start_line, column: start_column,
            flags: flags
          )

          m[0].length.times { do_advance }
          return result
        end

        nil
      end

      # Check if a token type represents a NAME token.
      # Works with both string and constant-based token types.
      def token_type_is_name?(token_type)
        token_type == "NAME" || token_type == TokenType::NAME
      end

      def do_advance
        return if @pos >= @source.length
        if @source[@pos] == "\n"
          @line += 1
          @column = 1
        else
          @column += 1
        end
        @pos += 1
      end

      # Map a token name from the grammar to a token type string.
      #
      # Resolution order:
      # 1. Check reserved keywords -- raise error if matched.
      # 2. Check regular keywords -- return KEYWORD type.
      # 3. Use alias_name if present.
      # 4. Look up in TokenType::ALL.
      # 5. Fall back to the token name as a string (grammar-driven types).
      #
      # When @case_insensitive is true, keyword and reserved-word lookups
      # are performed against the uppercased value so that "select",
      # "SELECT", and "Select" all resolve the same way.
      def resolve_token_type(token_name, value, alias_name)
        # Normalise the lookup key when case-insensitive mode is active.
        # The keyword sets were already uppercased at initialisation, so
        # we just need to upcase the incoming value before comparing.
        lookup_value = @case_insensitive ? value.upcase : value

        # Reserved keyword check.
        if token_name == "NAME" && @reserved_set.include?(lookup_value)
          raise LexerError.new(
            "Reserved keyword '#{value}' cannot be used as an identifier",
            line: @line, column: @column
          )
        end

        # Regular keyword check.
        if token_name == "NAME" && @keyword_set.include?(lookup_value)
          return TokenType::KEYWORD
        end

        # Alias takes precedence.
        if alias_name
          return TokenType::ALL[alias_name] || alias_name
        end

        # Standard lookup, falling back to the name as a string type.
        TokenType::ALL[token_name] || token_name
      end

      # Process escape sequences in a string value.
      def process_escapes(s)
        result = +""
        i = 0
        while i < s.length
          if s[i] == "\\" && i + 1 < s.length
            escape_map = {"n" => "\n", "t" => "\t", "\\" => "\\", '"' => '"'}
            next_char = s[i + 1]
            result << (escape_map[next_char] || next_char)
            i += 2
          else
            result << s[i]
            i += 1
          end
        end
        result
      end
    end
  end
end
