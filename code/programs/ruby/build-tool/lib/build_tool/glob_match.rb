# frozen_string_literal: true

# glob_match.rb -- Bounded Portable Glob Matching
# =================================================
#
# BUILD files declare source patterns that must mean the same thing in every
# build-tool implementation. This matcher therefore avoids File.fnmatch and
# compiles a deliberately small portable language:
#
#   - ** crosses path boundaries only when it is a complete segment.
#   - * matches zero or more Unicode scalars inside one segment.
#   - ? matches exactly one Unicode scalar inside one segment.
#   - [...] matches one scalar from a strict literal/range class.
#
# Compilation validates a complete pattern before matching. Matching then uses
# rolling-row dynamic programs for both path segments and tokens, so adversarial
# near misses visit each state once instead of recursively slicing suffixes.

module BuildTool
  module GlobMatch
    INVALID_PATTERN_MESSAGE = "ambiguous or descending character class in glob pattern"

    class InvalidPatternError < ArgumentError
      def initialize
        super(INVALID_PATTERN_MESSAGE)
      end
    end

    module_function

    # Match one path against one portable glob. Invalid syntax raises the
    # stable typed InvalidPatternError rather than becoming a host-dependent
    # non-match.
    def match_path?(pattern, path)
      match_compiled_path?(compile_pattern(pattern), path)
    end

    # Validate one pattern without matching a candidate path.
    def validate_pattern(pattern)
      compile_pattern(pattern)
      nil
    end

    # Compile the complete declared list before callers enumerate candidates.
    def compile_patterns(patterns)
      patterns.map { |pattern| compile_pattern(pattern) }.freeze
    end

    # Reuse one compiled pattern for every candidate in an operation.
    def match_compiled_path?(compiled_pattern, path)
      match_compiled_path_with_state_count(compiled_pattern, path).first
    end

    # Split a path string into non-empty compatibility segments.
    def split_path(string)
      return [] if string.empty?

      string.split("/").reject(&:empty?)
    end

    def compile_pattern(pattern)
      segments = []
      split_path(pattern.tr("\\", "/")).each do |segment|
        if segment == "**"
          segments << :globstar unless segments.last == :globstar
        else
          segments << parse_segment(segment).freeze
        end
      end
      segments.freeze
    end

    def parse_segment(segment)
      parse_segment_with_state_count(segment).first
    end

    def parse_segment_with_state_count(segment)
      scalars = segment.chars.each(&:freeze)
      next_closing_bracket = Array.new(scalars.length + 1)
      next_closing = nil

      (scalars.length - 1).downto(0) do |index|
        next_closing = index if scalars[index] == "]"
        next_closing_bracket[index] = next_closing
      end

      tokens = []
      index = 0
      visited = scalars.length
      while index < scalars.length
        visited += 1
        case scalars[index]
        when "*"
          tokens << [:star].freeze unless tokens.last&.first == :star
          index += 1
        when "?"
          tokens << [:question].freeze
          index += 1
        when "["
          parsed = parse_character_class(scalars, index, next_closing_bracket)
          if parsed
            token, index = parsed
            tokens << token
          else
            tokens << [:literal, "["].freeze
            index += 1
          end
        else
          tokens << [:literal, scalars[index]].freeze
          index += 1
        end
      end

      [tokens.freeze, visited]
    end

    def parse_character_class(scalars, opening, next_closing_bracket)
      cursor = opening + 1
      negated = cursor < scalars.length && scalars[cursor] == "!"
      cursor += 1 if negated

      closing = next_closing_bracket[cursor]
      closing = next_closing_bracket[cursor + 1] if closing == cursor
      return nil unless closing

      body = scalars[cursor...closing]
      ambiguous = body.each_cons(2).any? do |left, right|
        left == right && ["-", "&", "~", "|"].include?(left)
      end
      raise InvalidPatternError if ambiguous

      members = []
      member_index = 0
      while member_index < body.length
        if member_index + 2 < body.length && body[member_index + 1] == "-"
          range_start = body[member_index]
          range_end = body[member_index + 2]
          raise InvalidPatternError if range_start.ord > range_end.ord

          members << [:range, range_start, range_end].freeze
          member_index += 3
        else
          members << [:literal, body[member_index]].freeze
          member_index += 1
        end
      end

      [[:character_class, negated, members.freeze].freeze, closing + 1]
    end

    def match_path_with_state_count(pattern, path)
      match_compiled_path_with_state_count(compile_pattern(pattern), path)
    end

    def match_compiled_path_with_state_count(compiled_pattern, path)
      path_segments = split_path(path.tr("\\", "/")).map(&:chars)
      path_count = path_segments.length
      next_row = Array.new(path_count + 1, false)
      next_row[path_count] = true
      visited = path_count + 1

      compiled_pattern.reverse_each do |segment|
        row = Array.new(path_count + 1, false)
        visited += path_count + 1
        if segment == :globstar
          row[path_count] = next_row[path_count]
          (path_count - 1).downto(0) do |path_index|
            row[path_index] = next_row[path_index] || row[path_index + 1]
          end
        else
          (path_count - 1).downto(0) do |path_index|
            row[path_index] = next_row[path_index + 1] &&
              match_segment?(segment, path_segments[path_index])
          end
        end
        next_row = row
      end

      [next_row[0], visited]
    end

    def match_segment?(tokens, value)
      match_segment_with_state_count(tokens, value).first
    end

    def match_segment_with_state_count(tokens, value)
      value_count = value.length
      next_row = Array.new(value_count + 1, false)
      next_row[value_count] = true

      tokens.reverse_each do |token|
        row = Array.new(value_count + 1, false)
        case token.first
        when :star
          row[value_count] = next_row[value_count]
          (value_count - 1).downto(0) do |value_index|
            row[value_index] = next_row[value_index] || row[value_index + 1]
          end
        when :question
          value_count.times { |value_index| row[value_index] = next_row[value_index + 1] }
        else
          value_count.times do |value_index|
            row[value_index] = next_row[value_index + 1] && token_matches?(token, value[value_index])
          end
        end
        next_row = row
      end

      [next_row[0], (tokens.length + 1) * (value_count + 1)]
    end

    def token_matches?(token, value)
      case token.first
      when :literal
        token[1] == value
      when :character_class
        included = token[2].any? do |member|
          if member.first == :literal
            member[1] == value
          else
            value.ord.between?(member[1].ord, member[2].ord)
          end
        end
        token[1] ? !included : included
      else
        false
      end
    end
  end
end
