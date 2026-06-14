# frozen_string_literal: true

module CodingAdventures
  module MiniSqlite
    module Binding
      module_function

      def substitute(sql, params = [])
        index = 0
        out = +""
        i = 0
        while i < sql.length
          ch = sql[i]
          nxt = sql[i + 1]
          if ch == "'" || ch == '"'
            literal, i = read_quoted(sql, i, ch)
            out << literal
            next
          end
          if ch == "-" && nxt == "-"
            j = sql.index("\n", i) || sql.length
            out << sql[i...j]
            i = j
            next
          end
          if ch == "/" && nxt == "*"
            j = sql.index("*/", i + 2)
            j = j ? j + 2 : sql.length
            out << sql[i...j]
            i = j
            next
          end
          if ch == "?"
            raise ProgrammingError, "not enough parameters for SQL statement" if index >= params.length

            out << to_sql_literal(params[index])
            index += 1
            i += 1
            next
          end
          out << ch
          i += 1
        end
        raise ProgrammingError, "too many parameters for SQL statement" unless index == params.length

        out
      end

      def read_quoted(sql, start, quote)
        i = start + 1
        while i < sql.length
          if sql[i] == quote
            if sql[i + 1] == quote
              i += 2
              next
            end
            return [sql[start..i], i + 1]
          end
          i += 1
        end
        [sql[start..], sql.length]
      end

      def to_sql_literal(value)
        case value
        when nil then "NULL"
        when true then "TRUE"
        when false then "FALSE"
        when Integer, Float
          raise ProgrammingError, "non-finite numeric parameter is not supported" unless value.finite?

          value.to_s
        when String
          "'#{value.gsub("'", "''")}'"
        else
          raise ProgrammingError, "unsupported parameter type: #{value.class}"
        end
      end
    end
  end
end
