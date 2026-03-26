# frozen_string_literal: true

# ================================================================
# Executor — the relational pipeline for a SELECT statement
# ================================================================
#
# Execution order (logical, not syntactic):
#
#   1. FROM        — identify and scan source table
#   2. JOIN        — combine tables
#   3. WHERE       — filter individual rows
#   4. GROUP BY    — group rows for aggregation
#   5. HAVING      — filter groups post-aggregation
#   6. SELECT      — project and rename columns
#   7. DISTINCT    — deduplicate rows
#   8. ORDER BY    — sort rows
#   9. LIMIT       — paginate
# ================================================================

require "set"

module CodingAdventures
  module SqlExecutionEngine
    # Executes a single select_stmt AST node against a DataSource.
    class Executor
      ASTNode = CodingAdventures::Parser::ASTNode
      Token   = CodingAdventures::Lexer::Token

      def initialize(source)
        @source = source
      end

      # Execute a select_stmt node and return a QueryResult.
      def execute(stmt)
        # Phase 1: FROM
        table_ref = find_child(stmt, "table_ref")
        table_name, table_alias = extract_table_ref(table_ref)
        rows = @source.scan(table_name)
        effective_alias = table_alias.empty? ? table_name : table_alias
        rows = qualify_rows(rows, effective_alias)

        # Phase 2: JOIN
        find_children(stmt, "join_clause").each do |jc|
          rows = process_join(rows, jc)
        end

        # Phase 3: WHERE
        where_clause = find_child(stmt, "where_clause")
        rows = apply_where(rows, where_clause) if where_clause

        # Phase 4 & 5: GROUP BY / HAVING / Aggregates
        group_clause  = find_child(stmt, "group_clause")
        having_clause = find_child(stmt, "having_clause")
        select_list   = find_child(stmt, "select_list")
        all_agg_specs = collect_agg_specs(select_list) + collect_agg_specs(having_clause)
        all_agg_specs.uniq!

        if group_clause || !all_agg_specs.empty?
          rows = apply_group_by_and_aggregate(rows, group_clause, having_clause, all_agg_specs)
        end

        # Phase 6: ORDER BY (before projection so non-selected cols accessible)
        order_clause = find_child(stmt, "order_clause")
        rows = apply_order_by(rows, order_clause) if order_clause

        # Phase 7: SELECT projection
        has_distinct = has_distinct_qualifier?(stmt)
        columns, rows = apply_select(select_list, rows, table_name, effective_alias)

        # Phase 8: DISTINCT
        rows = apply_distinct(rows) if has_distinct

        # Phase 9: LIMIT
        limit_clause = find_child(stmt, "limit_clause")
        rows = apply_limit(rows, limit_clause) if limit_clause

        QueryResult.new(columns: columns, rows: rows)
      end

      private

      # ---- Phase 1: FROM --------------------------------------------------

      def extract_table_ref(table_ref)
        return ["", ""] unless table_ref

        table_name_node = find_child(table_ref, "table_name")
        if table_name_node
          name_token = first_token(table_name_node)
          table_name = name_token ? name_token.value : ""
        else
          name_token = first_token(table_ref)
          table_name = name_token ? name_token.value : ""
        end

        alias_val = ""
        children = table_ref.children
        children.each_with_index do |child, i|
          if child.is_a?(Token) && child.value.upcase == "AS"
            next_child = children[i + 1]
            alias_val = next_child.value if next_child.is_a?(Token)
          end
        end

        [table_name, alias_val]
      end

      def qualify_rows(rows, alias_name)
        rows.map do |row|
          qrow = row.dup
          row.each do |key, val|
            qrow["#{alias_name}.#{key}"] = val unless key.include?(".")
          end
          qrow
        end
      end

      # ---- Phase 2: JOIN --------------------------------------------------

      def process_join(left_rows, join_clause)
        join_type_node = find_child(join_clause, "join_type")
        join_type = extract_join_type(join_type_node)

        table_ref = find_child(join_clause, "table_ref")
        right_table, right_alias = extract_table_ref(table_ref)
        effective_alias = right_alias.empty? ? right_table : right_alias

        right_rows = @source.scan(right_table)
        right_rows = qualify_rows(right_rows, effective_alias)

        on_condition = find_on_condition(join_clause)

        Join.perform(left_rows, "", right_rows, effective_alias, join_type, on_condition)
      end

      def extract_join_type(join_type_node)
        return "INNER" unless join_type_node

        keywords = join_type_node.children
                                 .select { |c| c.is_a?(Token) }
                                 .map { |c| c.value.upcase }
        keywords.empty? ? "INNER" : keywords.join(" ")
      end

      def find_on_condition(join_clause)
        children = join_clause.children
        children.each_with_index do |child, i|
          if child.is_a?(Token) && child.value.upcase == "ON"
            return children[i + 1]
          end
        end
        nil
      end

      # ---- Phase 3: WHERE -------------------------------------------------

      def apply_where(rows, where_clause)
        expr = find_expr_in_clause(where_clause)
        return rows unless expr

        rows.select { |row| truthy?(Expression.eval_expr(expr, row)) }
      end

      # ---- Phase 4 & 5: GROUP BY / HAVING ----------------------------------

      def apply_group_by_and_aggregate(rows, group_clause, having_clause, agg_specs)
        group_keys = group_clause ? extract_group_keys(group_clause) : []

        groups = rows.group_by do |row|
          group_keys.map { |k| safe_get(row, k) }
        end

        result = []
        groups.each do |key_values, group_rows|
          rep_row = group_rows[0].dup
          group_keys.each_with_index { |k, i| rep_row[k] = key_values[i] }
          rep_row.merge!(Aggregate.compute(group_rows, agg_specs)) unless agg_specs.empty?
          rep_row["_group_rows"] = group_rows

          if having_clause
            having_expr = find_expr_in_clause(having_clause)
            next if having_expr && !truthy?(Expression.eval_expr(having_expr, rep_row))
          end

          result << rep_row
        end
        result
      end

      def extract_group_keys(group_clause)
        group_clause.children
                    .reject { |c| c.is_a?(Token) }
                    .map { |c| node_text(c).strip }
      end

      def collect_agg_specs(node)
        return [] unless node
        specs = []
        collect_agg_specs_recursive(node, specs)
        specs
      end

      def collect_agg_specs_recursive(node, specs)
        return if node.is_a?(Token)
        if node.rule_name == "function_call"
          name_token = node.children[0]
          if name_token.is_a?(Token)
            func_name = name_token.value.upcase
            if %w[COUNT SUM AVG MIN MAX].include?(func_name)
              arg_parts = node.children[2..-2].filter_map do |c|
                next if c.is_a?(Token) && c.value == ","
                c.is_a?(Token) ? c.value : node_text(c)
              end
              pair = [func_name, arg_parts.join]
              specs << pair unless specs.include?(pair)
            end
          end
        end
        node.children.each { |c| collect_agg_specs_recursive(c, specs) }
      end

      # ---- Phase 6: SELECT projection --------------------------------------

      def apply_select(select_list, rows, primary_table, primary_alias)
        return [[], rows] unless select_list

        if star_select?(select_list)
          if rows.empty?
            schema_cols = @source.schema(primary_alias.empty? ? primary_table : primary_alias)
            return [schema_cols, []]
          end
          cols = all_bare_columns(rows, primary_alias)
          projected = rows.map { |row| cols.each_with_object({}) { |c, h| h[c] = row[c] } }
          return [cols, projected]
        end

        items = find_children(select_list, "select_item")
        items = [select_list] if items.empty?

        columns = items.map { |item| extract_item_name(item)[0] }
        projected_rows = rows.map do |row|
          projected = {}
          items.each do |item|
            col_name, expr_node = extract_item_name(item)
            if expr_node
              begin
                projected[col_name] = Expression.eval_expr(expr_node, row)
              rescue ColumnNotFoundError
                projected[col_name] = nil
              end
            else
              projected[col_name] = nil
            end
          end
          projected
        end

        [columns, projected_rows]
      end

      def star_select?(select_list)
        select_list.children.any? do |child|
          (child.is_a?(Token) && child.value == "*") ||
            (child.is_a?(ASTNode) && child.rule_name == "select_item" &&
              child.children.any? { |gc| gc.is_a?(Token) && gc.value == "*" })
        end
      end

      def extract_item_name(item)
        children = item.children
        alias_val = nil
        expr_node = nil

        children.each_with_index do |child, i|
          if child.is_a?(Token) && child.value.upcase == "AS"
            next_child = children[i + 1]
            alias_val = next_child.value if next_child.is_a?(Token)
          elsif !(child.is_a?(Token) && child.value.upcase == "AS")
            expr_node ||= child
          end
        end

        name = alias_val || infer_col_name(expr_node)
        [name, expr_node]
      end

      def infer_col_name(node)
        return "?" unless node
        return node.value if node.is_a?(Token)

        if node.rule_name == "column_ref"
          tokens = node.children.select { |c| c.is_a?(Token) }
          return tokens.last&.value || "?"
        end

        node_text(node)
      end

      def all_bare_columns(rows, _alias)
        seen = Set.new
        cols = []
        rows[0].each_key do |key|
          next if key.include?(".") || key.start_with?("_")
          unless seen.include?(key)
            cols << key
            seen.add(key)
          end
        end
        cols
      end

      # ---- Phase 7: DISTINCT ----------------------------------------------

      def apply_distinct(rows)
        seen = []
        rows.select do |row|
          key = row.values
          if seen.include?(key)
            false
          else
            seen << key
            true
          end
        end
      end

      # ---- Phase 8: ORDER BY ----------------------------------------------

      def apply_order_by(rows, order_clause)
        order_items = find_children(order_clause, "order_item")
        return rows if order_items.empty?

        result = rows.dup
        order_items.reverse_each do |item|
          expr_node, ascending = extract_order_item(item)
          result.sort! do |a, b|
            va = begin
                   Expression.eval_expr(expr_node, a)
                 rescue ColumnNotFoundError
                   nil
                 end
            vb = begin
                   Expression.eval_expr(expr_node, b)
                 rescue ColumnNotFoundError
                   nil
                 end
            cmp = compare_nulls_last(va, vb)
            ascending ? cmp : -cmp
          end
        end
        result
      end

      def compare_nulls_last(a, b)
        if a.nil? && b.nil? then 0
        elsif a.nil? then 1
        elsif b.nil? then -1
        else a <=> b
        end
      end

      def extract_order_item(item)
        children = item.children
        ascending = true
        expr_node = children[0]
        children[1..].each do |child|
          if child.is_a?(Token)
            ascending = false if child.value.upcase == "DESC"
            ascending = true  if child.value.upcase == "ASC"
          end
        end
        [expr_node, ascending]
      end

      # ---- Phase 9: LIMIT -------------------------------------------------

      def apply_limit(rows, limit_clause)
        limit  = nil
        offset = 0
        children = limit_clause.children
        i = 0
        while i < children.size
          child = children[i]
          if child.is_a?(Token)
            if child.value.upcase == "LIMIT"
              i += 1
              limit = children[i].value.to_i if i < children.size
            elsif child.value.upcase == "OFFSET"
              i += 1
              offset = children[i].value.to_i if i < children.size
            end
          end
          i += 1
        end

        limit ? rows[offset, limit] || [] : rows[offset..]
      end

      # ---- DISTINCT qualifier ---------------------------------------------

      def has_distinct_qualifier?(stmt)
        stmt.children.any? { |c| c.is_a?(Token) && c.value.upcase == "DISTINCT" }
      end

      # ---- Generic AST helpers --------------------------------------------

      def find_child(node, rule_name)
        node.children.find { |c| c.is_a?(ASTNode) && c.rule_name == rule_name }
      end

      def find_children(node, rule_name)
        node.children.select { |c| c.is_a?(ASTNode) && c.rule_name == rule_name }
      end

      def find_expr_in_clause(clause)
        clause.children.each do |child|
          next if child.is_a?(Token) &&
                  %w[WHERE HAVING ON].include?(child.value.upcase)
          return child
        end
        nil
      end

      def first_token(node)
        node.children.find { |c| c.is_a?(Token) }
      end

      def safe_get(row, key)
        return row[key] if row.key?(key)
        key_down = key.downcase
        row.each { |k, v| return v if k.to_s.downcase == key_down }
        nil
      end

      def truthy?(val)
        val ? true : false
      end

      def node_text(node)
        return node.value if node.is_a?(Token)
        node.children.map { |c| node_text(c) }.join
      end
    end
  end
end
