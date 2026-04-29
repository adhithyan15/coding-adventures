# frozen_string_literal: true

module CodingAdventures
  module MiniSqlite
    module SQL
      module_function

      def first_keyword(sql)
        sql.lstrip[/\A[A-Za-z]+/].to_s.upcase
      end

      def parse_create(sql)
        match = /\A\s*CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?([A-Za-z_]\w*)\s*\((.*)\)\s*;?\s*\z/im.match(sql)
        raise ProgrammingError, "invalid CREATE TABLE statement" unless match

        columns = split_top_level(match[3], ",").map { |part| part.strip[/\A[A-Za-z_]\w*/] }.compact
        raise ProgrammingError, "CREATE TABLE requires at least one column" if columns.empty?

        {kind: :create, table: match[2], columns:, if_not_exists: !match[1].nil?}
      end

      def parse_drop(sql)
        match = /\A\s*DROP\s+TABLE\s+(IF\s+EXISTS\s+)?([A-Za-z_]\w*)\s*;?\s*\z/im.match(sql)
        raise ProgrammingError, "invalid DROP TABLE statement" unless match

        {kind: :drop, table: match[2], if_exists: !match[1].nil?}
      end

      def parse_insert(sql)
        match = /\A\s*INSERT\s+INTO\s+([A-Za-z_]\w*)(?:\s*\(([^)]*)\))?\s+VALUES\s+(.*?)\s*;?\s*\z/im.match(sql)
        raise ProgrammingError, "invalid INSERT statement" unless match

        columns = match[2] ? split_top_level(match[2], ",").map { |part| identifier(part.strip) } : nil
        {kind: :insert, table: match[1], columns:, rows: parse_value_rows(match[3])}
      end

      def parse_update(sql)
        text = sql.strip.delete_suffix(";").strip
        match = /\A\s*UPDATE\s+([A-Za-z_]\w*)\s+SET\s+(.*)\z/im.match(text)
        raise ProgrammingError, "invalid UPDATE statement" unless match

        assignment_sql, where_sql = split_top_level_keyword(match[2], "WHERE")
        assignments = {}
        split_top_level(assignment_sql, ",").each do |assignment|
          parts = split_top_level(assignment, "=")
          raise ProgrammingError, "invalid assignment: #{assignment.strip}" unless parts.length == 2

          assignments[identifier(parts[0].strip)] = parse_literal(parts[1].strip)
        end
        raise ProgrammingError, "UPDATE requires at least one assignment" if assignments.empty?

        {kind: :update, table: match[1], assignments:, where: where_sql}
      end

      def parse_delete(sql)
        match = /\A\s*DELETE\s+FROM\s+([A-Za-z_]\w*)(?:\s+WHERE\s+(.*?))?\s*;?\s*\z/im.match(sql)
        raise ProgrammingError, "invalid DELETE statement" unless match

        {kind: :delete, table: match[1], where: match[2].to_s.strip}
      end

      def parse_value_rows(sql)
        rest = sql.strip
        rows = []
        until rest.empty?
          raise ProgrammingError, "INSERT VALUES rows must be parenthesized" unless rest.start_with?("(")

          ending = matching_paren(rest, 0)
          raise ProgrammingError, "unterminated INSERT VALUES row" unless ending

          rows << split_top_level(rest[1...ending], ",").map { |part| parse_literal(part.strip) }
          rest = rest[(ending + 1)..].to_s.strip
          rest = rest[1..].strip if rest.start_with?(",")
        end
        raise ProgrammingError, "INSERT requires at least one row" if rows.empty?

        rows
      end

      def parse_literal(text)
        value = text.strip
        return nil if value.match?(/\ANULL\z/i)
        return true if value.match?(/\ATRUE\z/i)
        return false if value.match?(/\AFALSE\z/i)
        return value.to_f if value.match?(/\A-?\d+\.\d+\z/)
        return value.to_i if value.match?(/\A-?\d+\z/)
        return value[1...-1].gsub("''", "'") if value.start_with?("'") && value.end_with?("'")

        raise ProgrammingError, "expected literal value, got: #{text}"
      end

      def split_top_level(text, delimiter)
        parts = []
        start = 0
        depth = 0
        quote = nil
        i = 0
        while i < text.length
          ch = text[i]
          if quote
            if ch == quote && text[i + 1] == quote
              i += 2
              next
            elsif ch == quote
              quote = nil
            end
          elsif ch == "'" || ch == '"'
            quote = ch
          elsif ch == "("
            depth += 1
          elsif ch == ")"
            depth -= 1
          elsif depth.zero? && text[i, delimiter.length] == delimiter
            part = text[start...i].strip
            parts << part unless part.empty?
            i += delimiter.length
            start = i
            next
          end
          i += 1
        end
        part = text[start..].to_s.strip
        parts << part unless part.empty?
        parts
      end

      def split_top_level_keyword(text, keyword)
        depth = 0
        quote = nil
        upper = text.upcase
        needle = keyword.upcase
        i = 0
        while i < text.length
          ch = text[i]
          if quote
            if ch == quote && text[i + 1] == quote
              i += 2
              next
            elsif ch == quote
              quote = nil
            end
          elsif ch == "'" || ch == '"'
            quote = ch
          elsif ch == "("
            depth += 1
          elsif ch == ")"
            depth -= 1
          elsif depth.zero? && upper[i, needle.length] == needle && boundary?(text[i - 1]) && boundary?(text[i + needle.length])
            return [text[0...i].strip, text[(i + needle.length)..].to_s.strip]
          end
          i += 1
        end
        [text.strip, ""]
      end

      def matching_paren(text, open_index)
        depth = 0
        quote = nil
        i = open_index
        while i < text.length
          ch = text[i]
          if quote
            if ch == quote && text[i + 1] == quote
              i += 2
              next
            elsif ch == quote
              quote = nil
            end
          elsif ch == "'" || ch == '"'
            quote = ch
          elsif ch == "("
            depth += 1
          elsif ch == ")"
            depth -= 1
            return i if depth.zero?
          end
          i += 1
        end
        nil
      end

      def identifier(text)
        raise ProgrammingError, "invalid identifier: #{text}" unless text.match?(/\A[A-Za-z_]\w*\z/)

        text
      end

      def boundary?(char)
        char.nil? || !char.match?(/[A-Za-z0-9_]/)
      end
    end
  end
end
