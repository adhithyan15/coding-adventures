# frozen_string_literal: true

# ==========================================================================
# token_grammar.rb -- Parser and Validator for .tokens Files
# ==========================================================================
#
# A .tokens file is a declarative description of the lexical grammar of a
# programming language. It lists every token the lexer should recognize, in
# priority order (first match wins), along with optional sections for
# keywords, reserved words, skip patterns, and lexer mode configuration.
#
# File format overview
# --------------------
#
# Each non-blank, non-comment line has one of these forms:
#
#   TOKEN_NAME = /regex_pattern/           -- a regex-based token
#   TOKEN_NAME = "literal_string"          -- a literal-string token
#   TOKEN_NAME = /regex/ -> ALIAS          -- emits token type ALIAS instead
#   TOKEN_NAME = "literal" -> ALIAS        -- same for literals
#   mode: indentation                      -- sets the lexer mode
#   keywords:                              -- begins the keywords section
#   reserved:                              -- begins the reserved keywords section
#   skip:                                  -- begins the skip patterns section
#   group NAME:                            -- begins a named pattern group
#
# Lines starting with # are comments. Blank lines are ignored.
#
# Pattern groups (group NAME:)
# ----------------------------
#
# Pattern groups enable context-sensitive lexing. The lexer maintains a
# stack of active groups and only tries patterns from the group on top of
# the stack. Language-specific callback code pushes/pops groups in response
# to matched tokens. For example, an XML lexer pushes a "tag" group when
# it sees ``<`` and pops it on ``>``, so attribute-related patterns are
# only active inside tags. Patterns outside any group section belong to
# the implicit "default" group. The grammar file contains no transition
# logic -- just pattern definitions labeled by group.
#
# Design decisions
# ----------------
#
# Why hand-parse instead of using regex or a parser library? Because the
# format is simple enough that a line-by-line parser is clearer, faster, and
# produces better error messages than any generic tool would. Every error
# includes the line number where the problem occurred, which matters a lot
# when users are writing grammars by hand.
# ==========================================================================

module CodingAdventures
  module GrammarTools
    # Raised when a .tokens file cannot be parsed.
    class TokenGrammarError < StandardError
      attr_reader :message, :line_number

      def initialize(message, line_number)
        @message = message
        @line_number = line_number
        super("Line #{line_number}: #{message}")
      end
    end

    # A single token rule from a .tokens file.
    #
    # Attributes:
    #   name        -- the token name, e.g. "NUMBER" or "PLUS"
    #   pattern     -- the pattern string (regex body or literal body)
    #   is_regex    -- true if written as /regex/, false if "literal"
    #   line_number -- 1-based line where this definition appeared
    #   alias_name  -- optional type alias (e.g. STRING_DQ -> STRING means
    #                  alias_name is "STRING"). The lexer emits the alias
    #                  as the token type instead of the definition name.
    TokenDefinition = Data.define(:name, :pattern, :is_regex, :line_number, :alias_name) do
      def initialize(name:, pattern:, is_regex:, line_number:, alias_name: nil)
        super(name: name, pattern: pattern, is_regex: is_regex,
              line_number: line_number, alias_name: alias_name)
      end
    end

    # A named set of token definitions that are active together.
    #
    # When this group is at the top of the lexer's group stack, only these
    # patterns are tried during token matching. Skip patterns are global
    # and always tried regardless of the active group.
    #
    # Pattern groups enable context-sensitive lexing. For example, an XML
    # lexer defines a "tag" group with patterns for attribute names, equals
    # signs, and attribute values. These patterns are only active inside
    # tags -- the callback pushes the "tag" group when ``<`` is matched and
    # pops it when ``>`` is matched.
    #
    # Attributes:
    #   name        -- the group name, e.g. "tag" or "cdata". Must be a
    #                  lowercase identifier matching [a-z_][a-z0-9_]*.
    #   definitions -- ordered list of token definitions in this group.
    #                  Order matters (first-match-wins), just like the
    #                  top-level definitions list.
    PatternGroup = Data.define(:name, :definitions)

    # The complete contents of a parsed .tokens file.
    #
    # definitions       -- ordered list of TokenDefinition (order = priority)
    # keywords          -- list of reserved words from the keywords: section
    # mode              -- optional lexer mode (e.g. "indentation")
    # skip_definitions  -- patterns matched and consumed without producing tokens
    # reserved_keywords -- keywords that cause lex errors if used as identifiers
    # groups            -- hash of group_name => PatternGroup for named
    #                      pattern groups (context-sensitive lexing)
    # case_sensitive    -- whether the lexer should match case-sensitively
    #                      (default true). When false, the lexer lowercases
    #                      the source text before matching. Used by
    #                      case-insensitive languages like VHDL and SQL.
    # version           -- integer schema version from "# @version N" magic
    #                      comment; defaults to 0 when not specified
    # case_insensitive  -- when true, token patterns should be matched without
    #                      regard to letter case; set via "# @case_insensitive
    #                      true" magic comment; defaults to false
    # The complete contents of a parsed .tokens file.
    #
    # layout_keywords  -- list of keywords that introduce a layout context
    #                     when mode is "layout". The generic lexer uses
    #                     these to inject virtual "{", ";", and "}"
    #                     tokens following Haskell-style offside rules.
    #                     Examples: let, where, do, of.
    # context_keywords -- list of context-sensitive keywords from the
    #                     context_keywords: section. These are words that
    #                     are keywords in some syntactic positions but
    #                     identifiers in others (e.g., JavaScript's async,
    #                     await, yield, get, set). The lexer emits them as
    #                     NAME tokens with the TOKEN_CONTEXT_KEYWORD flag
    #                     set, leaving the final keyword-vs-identifier
    #                     decision to the language-specific parser.
    # soft_keywords    -- list of soft keywords from the soft_keywords:
    #                     section. These are words that act as keywords
    #                     only in specific syntactic contexts, remaining
    #                     ordinary identifiers everywhere else.
    #
    #                     Unlike context_keywords (which set a flag on the
    #                     token), soft keywords produce plain NAME tokens
    #                     with NO special flag. The lexer is completely
    #                     unaware of their keyword status -- the parser
    #                     handles disambiguation entirely based on
    #                     syntactic position.
    #
    #                     This distinction matters because:
    #                       - context_keywords: lexer hints to parser
    #                         ("this NAME might be special")
    #                       - soft_keywords: lexer ignores them completely,
    #                         parser owns the decision
    #
    #                     Examples:
    #                       Python 3.10+: match, case, _
    #                       Python 3.12+: type
    class TokenGrammar
      attr_reader :definitions, :keywords, :skip_definitions, :error_definitions,
                  :reserved_keywords, :groups, :layout_keywords,
                  :context_keywords, :soft_keywords
      attr_accessor :mode, :escape_mode, :case_sensitive, :version, :case_insensitive

      def initialize(definitions: [], keywords: [], mode: nil,
                     skip_definitions: [], error_definitions: [],
                     reserved_keywords: [], escape_mode: nil, groups: {},
                     layout_keywords: [],
                     case_sensitive: true, version: 0, case_insensitive: false,
                     context_keywords: [], soft_keywords: [])
        @definitions = definitions
        @keywords = keywords
        @mode = mode
        @skip_definitions = skip_definitions
        @error_definitions = error_definitions
        @reserved_keywords = reserved_keywords
        @escape_mode = escape_mode
        @groups = groups
        @layout_keywords = layout_keywords
        @case_sensitive = case_sensitive
        @version = version
        @case_insensitive = case_insensitive
        @context_keywords = context_keywords
        @soft_keywords = soft_keywords
      end

      # Return the set of all defined token names (including aliases).
      #
      # Includes names from all pattern groups, since group tokens can
      # also appear in parser grammars.
      def token_names
        names = Set.new
        all_defs = @definitions.dup
        @groups.each_value { |g| all_defs.concat(g.definitions) }
        all_defs.each do |d|
          names.add(d.name)
          names.add(d.alias_name) if d.alias_name
        end
        names
      end

      # Return the set of token names as the parser will see them.
      # For definitions with aliases, returns the alias (not the name).
      # Includes names from all pattern groups.
      def effective_token_names
        all_defs = @definitions.dup
        @groups.each_value { |g| all_defs.concat(g.definitions) }
        all_defs.map { |d| d.alias_name || d.name }.to_set
      end
    end

    # Scan a /pattern/ string starting at index 1 and return the index of
    # the closing /. Skips escaped characters (\x) and does not treat /
    # inside [...] character classes as the closing delimiter.
    # Returns -1 if no closing slash is found.
    def self.find_closing_slash(s)
      in_bracket = false
      i = 1
      while i < s.length
        ch = s[i]
        if ch == "\\"
          i += 2 # skip escaped character
          next
        end
        if ch == "[" && !in_bracket
          in_bracket = true
        elsif ch == "]" && in_bracket
          in_bracket = false
        elsif ch == "/" && !in_bracket
          return i
        end
        i += 1
      end
      # Fallback: if bracket-aware scan found nothing (e.g. unclosed [),
      # try the last / as a best-effort parse.
      last = s.rindex("/")
      (last && last > 0) ? last : -1
    end

    # Parse a single token definition's pattern and optional -> ALIAS suffix.
    #
    # Returns a TokenDefinition. The pattern_part may have a "-> ALIAS"
    # suffix after the closing delimiter.
    def self.parse_definition(pattern_part, name_part, line_number)
      alias_name = nil

      if pattern_part.start_with?("/")
        # Regex pattern — find the closing / by scanning character-by-character.
        # We track bracket depth so that / inside [...] character classes is
        # not mistaken for the closing delimiter. We also skip escaped chars.
        last_slash = find_closing_slash(pattern_part)
        if last_slash == -1
          raise TokenGrammarError.new(
            "Unclosed regex pattern for token #{name_part.inspect}",
            line_number
          )
        end
        regex_body = pattern_part[1...last_slash]
        remainder = pattern_part[(last_slash + 1)..].strip

        if regex_body.empty?
          raise TokenGrammarError.new(
            "Empty regex pattern for token #{name_part.inspect}",
            line_number
          )
        end

        if remainder.start_with?("->")
          alias_name = remainder[2..].strip
          if alias_name.empty?
            raise TokenGrammarError.new(
              "Missing alias after '->' for token #{name_part.inspect}",
              line_number
            )
          end
        elsif !remainder.empty?
          raise TokenGrammarError.new(
            "Unexpected text after pattern for token #{name_part.inspect}: #{remainder.inspect}",
            line_number
          )
        end

        TokenDefinition.new(
          name: name_part, pattern: regex_body,
          is_regex: true, line_number: line_number, alias_name: alias_name
        )

      elsif pattern_part.start_with?('"')
        # Literal pattern -- find the closing "
        close_quote = pattern_part.index('"', 1)
        unless close_quote
          raise TokenGrammarError.new(
            "Unclosed literal pattern for token #{name_part.inspect}",
            line_number
          )
        end
        literal_body = pattern_part[1...close_quote]
        remainder = pattern_part[(close_quote + 1)..].strip

        if literal_body.empty?
          raise TokenGrammarError.new(
            "Empty literal pattern for token #{name_part.inspect}",
            line_number
          )
        end

        if remainder.start_with?("->")
          alias_name = remainder[2..].strip
          if alias_name.empty?
            raise TokenGrammarError.new(
              "Missing alias after '->' for token #{name_part.inspect}",
              line_number
            )
          end
        elsif !remainder.empty?
          raise TokenGrammarError.new(
            "Unexpected text after pattern for token #{name_part.inspect}: #{remainder.inspect}",
            line_number
          )
        end

        TokenDefinition.new(
          name: name_part, pattern: literal_body,
          is_regex: false, line_number: line_number, alias_name: alias_name
        )

      else
        raise TokenGrammarError.new(
          "Pattern for token #{name_part.inspect} must be /regex/ or \"literal\", got: #{pattern_part.inspect}",
          line_number
        )
      end
    end

    # Parse the text of a .tokens file into a TokenGrammar.
    #
    # The parser operates line-by-line with several modes:
    #
    # 1. Definition mode (default) -- each line is a comment, blank, section
    #    header, or token definition.
    # 2. Keywords mode -- entered on "keywords:" line.
    # 3. Reserved mode -- entered on "reserved:" line.
    # 4. Skip mode -- entered on "skip:" line. Contains token definitions
    #    for patterns that are consumed without producing tokens.
    # 5. Group mode -- entered on "group NAME:" line. Contains token
    #    definitions that belong to the named pattern group.
    #
    # The "mode:" directive sets the lexer mode (e.g. "indentation") and
    # can appear anywhere outside a section.
    def self.parse_token_grammar(source)
      lines = source.split("\n")
      grammar = TokenGrammar.new
      # Section tracking. We use a string to track which section we're in,
      # since sections are mutually exclusive and we can only be in one at
      # a time (or in no section = definition mode).
      #
      # For pattern groups, current_section is "group:NAME" where NAME is
      # the group name. This distinguishes groups from other sections.
      current_section = nil # "keywords", "reserved", "skip", "group:NAME"

      lines.each_with_index do |raw_line, index|
        line_number = index + 1
        line = raw_line.rstrip
        stripped = line.strip

        # Blank lines are always skipped.
        next if stripped.empty?

        # Comment lines: check for magic comments before skipping.
        #
        # A magic comment has the form:
        #
        #   # @key value
        #
        # where key is an identifier and value is the rest of the line.
        # Known keys:
        #   @version N             -- sets grammar.version to N (integer)
        #   @case_insensitive true/false -- sets grammar.case_insensitive
        #
        # Unknown keys are silently ignored so that future extensions do not
        # break older parsers. Regular comments (no @key) are also ignored.
        if stripped.start_with?("#")
          if (m = stripped.match(/^#\s*@(\w+)\s*(.*)/))
            key = m[1]
            value = m[2].strip
            case key
            when "version"
              grammar.version = value.to_i
            when "case_insensitive"
              grammar.case_insensitive = (value == "true")
            end
            # Unknown keys: silently ignore (fall through to next)
          end
          next
        end

        # mode: directive -- sets the lexer mode.
        if stripped.start_with?("mode:")
          mode_value = stripped[5..].strip
          if mode_value.empty?
            raise TokenGrammarError.new(
              "Missing value after 'mode:'", line_number
            )
          end
          grammar.mode = mode_value
          current_section = nil
          next
        end

        # escapes: directive -- controls how STRING tokens are processed.
        # "none" disables escape processing (quotes are stripped but escape
        # sequences are left as-is). Useful for languages like CSS and TOML
        # where escape semantics differ from JSON.
        if stripped.start_with?("escapes:")
          escape_value = stripped[8..].strip
          if escape_value.empty?
            raise TokenGrammarError.new(
              "Missing value after 'escapes:'", line_number
            )
          end
          grammar.escape_mode = escape_value
          current_section = nil
          next
        end

        # case_sensitive: directive -- controls whether the lexer matches
        # case-sensitively. ``case_sensitive: false`` makes the lexer
        # lowercase the source text before matching. Used by
        # case-insensitive languages like VHDL and SQL.
        if stripped.start_with?("case_sensitive:")
          cs_value = stripped[15..].strip.downcase
          unless %w[true false].include?(cs_value)
            raise TokenGrammarError.new(
              "Invalid value for 'case_sensitive:': #{cs_value.inspect} " \
              "(expected 'true' or 'false')",
              line_number
            )
          end
          grammar.case_sensitive = cs_value == "true"
          current_section = nil
          next
        end

        # Group headers -- "group NAME:" declares a named pattern group.
        # All subsequent indented lines belong to that group, just like
        # skip: or reserved: sections.
        if stripped.start_with?("group ") && stripped.end_with?(":")
          group_name = stripped[6..-2].strip
          if group_name.empty?
            raise TokenGrammarError.new(
              "Missing group name after 'group'", line_number
            )
          end
          unless group_name.match?(/\A[a-z_][a-z0-9_]*\z/)
            raise TokenGrammarError.new(
              "Invalid group name: #{group_name.inspect} " \
              "(must be a lowercase identifier like 'tag' or 'cdata')",
              line_number
            )
          end
          reserved_names = %w[
            default skip keywords reserved errors layout_keywords
            context_keywords soft_keywords
          ].to_set
          if reserved_names.include?(group_name)
            raise TokenGrammarError.new(
              "Reserved group name: #{group_name.inspect} " \
              "(cannot use #{reserved_names.to_a.sort.join(", ")})",
              line_number
            )
          end
          if grammar.groups.key?(group_name)
            raise TokenGrammarError.new(
              "Duplicate group name: #{group_name.inspect}",
              line_number
            )
          end
          grammar.groups[group_name] = PatternGroup.new(
            name: group_name, definitions: []
          )
          current_section = "group:#{group_name}"
          next
        end


        # Section headers.
        if stripped == "keywords:" || stripped == "keywords :"
          current_section = "keywords"
          next
        end

        if stripped == "reserved:" || stripped == "reserved :"
          current_section = "reserved"
          next
        end

        if stripped == "skip:" || stripped == "skip :"
          current_section = "skip"
          next
        end

        # errors: section -- fallback patterns for graceful lexer error
        # recovery. Tried only when no normal token matches. This mirrors
        # the Python grammar_tools behaviour so that CSS/Lattice .tokens
        # files that include an errors: block are accepted without error.
        if stripped == "errors:" || stripped == "errors :"
          current_section = "errors"
          next
        end

        if stripped == "layout_keywords:" || stripped == "layout_keywords :"
          current_section = "layout_keywords"
          next
        end

        # context_keywords: section -- words that are keywords in some
        # syntactic positions but identifiers in others. The lexer emits
        # these as NAME tokens with the TOKEN_CONTEXT_KEYWORD flag set.
        if stripped == "context_keywords:" || stripped == "context_keywords :"
          current_section = "context_keywords"
          next
        end

        # soft_keywords: section -- words that act as keywords only in
        # specific syntactic contexts. Unlike context_keywords, these
        # produce plain NAME tokens with no special flag. The parser
        # handles all disambiguation based on syntactic position.
        if stripped == "soft_keywords:" || stripped == "soft_keywords :"
          current_section = "soft_keywords"
          next
        end

        # Inside a section -- indented lines belong to the section.
        if current_section
          if line.start_with?(" ", "\t")
            case current_section
            when "keywords"
              grammar.keywords << stripped unless stripped.empty?
            when "layout_keywords"
              grammar.layout_keywords << stripped unless stripped.empty?
            when "context_keywords"
              grammar.context_keywords << stripped unless stripped.empty?
            when "soft_keywords"
              grammar.soft_keywords << stripped unless stripped.empty?
            when "reserved"
              grammar.reserved_keywords << stripped unless stripped.empty?
            when "skip"
              unless stripped.include?("=")
                raise TokenGrammarError.new(
                  "Expected skip pattern definition (NAME = pattern), got: #{stripped.inspect}",
                  line_number
                )
              end
              eq_idx = stripped.index("=")
              skip_name = stripped[0...eq_idx].strip
              skip_pattern = stripped[(eq_idx + 1)..].strip
              if skip_name.empty? || skip_pattern.empty?
                raise TokenGrammarError.new(
                  "Incomplete skip pattern definition: #{stripped.inspect}",
                  line_number
                )
              end
              grammar.skip_definitions << parse_definition(
                skip_pattern, skip_name, line_number
              )
            when "errors"
              # Error recovery patterns — stored for completeness and
              # compatibility with .tokens files that include an errors:
              # block (e.g. css.tokens, lattice.tokens). The GrammarLexer
              # does not currently use these, but storing them here keeps
              # the grammar_tools round-trip-compatible with the Python
              # grammar_tools which does support them.
              unless stripped.include?("=")
                raise TokenGrammarError.new(
                  "Expected error pattern definition (NAME = pattern), got: #{stripped.inspect}",
                  line_number
                )
              end
              eq_idx = stripped.index("=")
              err_name = stripped[0...eq_idx].strip
              err_pattern = stripped[(eq_idx + 1)..].strip
              if err_name.empty? || err_pattern.empty?
                raise TokenGrammarError.new(
                  "Incomplete error pattern definition: #{stripped.inspect}",
                  line_number
                )
              end
              grammar.error_definitions << parse_definition(
                err_pattern, err_name, line_number
              )
            else
              # Pattern group section -- current_section is "group:NAME".
              if current_section.start_with?("group:")
                gname = current_section[6..]
                unless stripped.include?("=")
                  raise TokenGrammarError.new(
                    "Expected token definition in group '#{gname}' " \
                    "(NAME = pattern), got: #{stripped.inspect}",
                    line_number
                  )
                end
                eq_idx = stripped.index("=")
                g_name = stripped[0...eq_idx].strip
                g_pattern = stripped[(eq_idx + 1)..].strip
                if g_name.empty? || g_pattern.empty?
                  raise TokenGrammarError.new(
                    "Incomplete definition in group '#{gname}': #{stripped.inspect}",
                    line_number
                  )
                end
                defn = parse_definition(g_pattern, g_name, line_number)
                # PatternGroup is a Data (frozen), so we replace it with
                # a new instance that includes the additional definition.
                old_group = grammar.groups[gname]
                grammar.groups[gname] = PatternGroup.new(
                  name: gname,
                  definitions: [*old_group.definitions, defn]
                )
              end
            end
            next
          else
            # Non-indented line -- exit section, fall through.
            current_section = nil
          end
        end

        # Token definition -- NAME = /pattern/ or NAME = "literal"
        unless line.include?("=")
          raise TokenGrammarError.new(
            "Expected token definition (NAME = pattern), got: #{stripped.inspect}",
            line_number
          )
        end

        eq_index = line.index("=")
        name_part = line[0...eq_index].strip
        pattern_part = line[(eq_index + 1)..].strip

        if name_part.empty?
          raise TokenGrammarError.new("Missing token name before '='", line_number)
        end

        unless name_part.match?(/\A[a-zA-Z_][a-zA-Z0-9_]*\z/)
          raise TokenGrammarError.new(
            "Invalid token name: #{name_part.inspect} (must be an identifier like NAME or PLUS_EQUALS)",
            line_number
          )
        end

        if pattern_part.empty?
          raise TokenGrammarError.new(
            "Missing pattern after '=' for token #{name_part.inspect}",
            line_number
          )
        end

        grammar.definitions << parse_definition(
          pattern_part, name_part, line_number
        )
      end

      grammar
    end

    # Validate a list of token definitions (shared logic for regular and skip).
    def self.validate_definitions(definitions, label)
      issues = []
      seen_names = {}

      definitions.each do |defn|
        # Duplicate check.
        if seen_names.key?(defn.name)
          issues << "Line #{defn.line_number}: Duplicate #{label} name '#{defn.name}' " \
                    "(first defined on line #{seen_names[defn.name]})"
        else
          seen_names[defn.name] = defn.line_number
        end

        # Empty pattern check.
        if defn.pattern.empty?
          issues << "Line #{defn.line_number}: Empty pattern for #{label} '#{defn.name}'"
        end

        # Invalid regex check.
        if defn.is_regex
          begin
            Regexp.new(defn.pattern)
          rescue RegexpError => e
            issues << "Line #{defn.line_number}: Invalid regex for #{label} '#{defn.name}': #{e.message}"
          end
        end

        # Naming convention check.
        unless defn.name == defn.name.upcase
          issues << "Line #{defn.line_number}: Token name '#{defn.name}' should be UPPER_CASE"
        end

        # Alias convention check.
        if defn.alias_name && defn.alias_name != defn.alias_name.upcase
          issues << "Line #{defn.line_number}: Alias '#{defn.alias_name}' for " \
                    "token '#{defn.name}' should be UPPER_CASE"
        end
      end

      issues
    end

    # Check a parsed TokenGrammar for common problems.
    #
    # Validation checks:
    # - Duplicate token names
    # - Invalid regex patterns
    # - Empty patterns (safety net)
    # - Non-UPPER_CASE names (convention warning)
    # - Invalid aliases
    # - Unknown lexer mode
    # - Skip definition issues
    # - Pattern group issues (bad regex, empty groups, bad names)
    def self.validate_token_grammar(grammar)
      issues = []
      issues.concat(validate_definitions(grammar.definitions, "token"))
      issues.concat(validate_definitions(grammar.skip_definitions, "skip pattern"))

      supported_modes = [nil, "indentation", "layout"]
      unless supported_modes.include?(grammar.mode)
        issues << "Unknown lexer mode '#{grammar.mode}' " \
                  "(supported: indentation, layout)"
      end

      if grammar.mode == "layout" && grammar.layout_keywords.empty?
        issues << "Layout mode requires a non-empty layout_keywords: section"
      end

      # Validate pattern groups.
      grammar.groups.each do |group_name, group|
        # Group name format check.
        unless group_name.match?(/\A[a-z_][a-z0-9_]*\z/)
          issues << "Invalid group name '#{group_name}' " \
                    "(must be a lowercase identifier)"
        end

        # Empty group warning.
        if group.definitions.empty?
          issues << "Empty pattern group '#{group_name}' " \
                    "(has no token definitions)"
        end

        # Validate definitions within the group.
        issues.concat(
          validate_definitions(group.definitions, "group '#{group_name}' token")
        )
      end

      issues
    end
  end
end
