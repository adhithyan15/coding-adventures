# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the SQL Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when loaded
# with sql.grammar, correctly builds Abstract Syntax Trees from
# ANSI SQL text.
#
# The grammar-driven parser produces generic ASTNode objects:
#
#   ASTNode(rule_name: "program", children: [...])
#
# Each node records which grammar rule produced it and its matched
# children (which can be tokens or other ASTNodes). This is different
# from a hand-written parser's typed nodes (SelectStatement, etc.),
# but it captures the same structural information.
#
# SQL's grammar (sql.grammar) has the following key rules:
#
#   program           = statement { ";" statement } [ ";" ]
#   statement         = select_stmt | insert_stmt | update_stmt
#                     | delete_stmt | create_table_stmt | drop_table_stmt
#   select_stmt       = "SELECT" [...] select_list "FROM" table_ref [...]
#   insert_stmt       = "INSERT" "INTO" NAME [...] "VALUES" row_value {...}
#   update_stmt       = "UPDATE" NAME "SET" assignment [...] [where_clause]
#   delete_stmt       = "DELETE" "FROM" NAME [where_clause]
#   create_table_stmt = "CREATE" "TABLE" [...] NAME "(" col_def {...} ")"
#   drop_table_stmt   = "DROP" "TABLE" [...] NAME
#
# Case insensitivity:
#   Because sql.tokens uses @case_insensitive true, keywords are
#   normalized to uppercase by the sql_lexer. So "select", "SELECT",
#   and "Select" all parse identically.
# ================================================================

class TestSqlParser < Minitest::Test
  ASTNode = CodingAdventures::Parser::ASTNode
  TT = CodingAdventures::Lexer::TokenType

  # SQL-specific token type strings
  NOT_EQUALS_TYPE    = "NOT_EQUALS"
  LESS_EQUALS_TYPE   = "LESS_EQUALS"
  GREATER_EQUALS_TYPE = "GREATER_EQUALS"

  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::SqlParser.parse_sql(source)
  end

  # Recursively collect all Token objects from an AST.
  def collect_tokens(node)
    tokens = []
    return tokens unless node.is_a?(ASTNode)

    node.children.each do |child|
      if child.is_a?(CodingAdventures::Lexer::Token)
        tokens << child
      elsif child.is_a?(ASTNode)
        tokens.concat(collect_tokens(child))
      end
    end
    tokens
  end

  # Find all ASTNode descendants with a given rule_name.
  def find_nodes_by_rule(node, rule_name)
    results = []
    return results unless node.is_a?(ASTNode)

    results << node if node.rule_name == rule_name
    node.children.each do |child|
      results.concat(find_nodes_by_rule(child, rule_name)) if child.is_a?(ASTNode)
    end
    results
  end

  # Collect all keyword token values from an AST.
  def keyword_values(node)
    collect_tokens(node).select { |t| t.type == TT::KEYWORD }.map(&:value)
  end

  # ------------------------------------------------------------------
  # create_sql_parser
  # ------------------------------------------------------------------

  def test_create_sql_parser_returns_non_nil
    parser = CodingAdventures::SqlParser.create_sql_parser("SELECT 1 FROM t")
    refute_nil parser
  end

  def test_create_sql_parser_returns_grammar_driven_parser
    parser = CodingAdventures::SqlParser.create_sql_parser("SELECT 1 FROM t")
    assert_instance_of CodingAdventures::Parser::GrammarDrivenParser, parser
  end

  # ------------------------------------------------------------------
  # Grammar path
  # ------------------------------------------------------------------

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::SqlParser::SQL_GRAMMAR_PATH),
      "sql.grammar file should exist at #{CodingAdventures::SqlParser::SQL_GRAMMAR_PATH}"
  end

  # ------------------------------------------------------------------
  # Root node is always 'program'
  # ------------------------------------------------------------------
  # Every SQL parse tree starts with the 'program' rule (see sql.grammar
  # first line: program = statement { ";" statement } [ ";" ])

  def test_root_is_program
    ast = parse("SELECT 1 FROM t")
    assert_equal "program", ast.rule_name
  end

  # ------------------------------------------------------------------
  # SELECT statements
  # ------------------------------------------------------------------

  def test_simple_select
    # SELECT * FROM users
    ast = parse("SELECT * FROM users")
    assert_equal "program", ast.rule_name

    # Should contain a select_stmt node
    select_nodes = find_nodes_by_rule(ast, "select_stmt")
    assert select_nodes.length >= 1, "Expected a select_stmt node"

    # Should have SELECT and FROM keywords
    kw = keyword_values(ast)
    assert_includes kw, "SELECT"
    assert_includes kw, "FROM"
  end

  def test_select_with_column_list
    # SELECT id, name FROM users
    ast = parse("SELECT id, name FROM users")
    kw = keyword_values(ast)
    assert_includes kw, "SELECT"
    assert_includes kw, "FROM"

    all_tokens = collect_tokens(ast)
    name_tokens = all_tokens.select { |t| t.type == TT::NAME }
    name_values = name_tokens.map(&:value)
    assert_includes name_values, "id"
    assert_includes name_values, "name"
    assert_includes name_values, "users"
  end

  def test_select_with_where_clause
    # SELECT id FROM users WHERE id = 1
    ast = parse("SELECT id FROM users WHERE id = 1")
    kw = keyword_values(ast)
    assert_includes kw, "WHERE"

    where_nodes = find_nodes_by_rule(ast, "where_clause")
    assert where_nodes.length >= 1, "Expected a where_clause node"
  end

  def test_select_with_string_literal
    # SELECT * FROM users WHERE name = 'Alice'
    ast = parse("SELECT * FROM users WHERE name = 'Alice'")

    all_tokens = collect_tokens(ast)
    string_tokens = all_tokens.select { |t| t.type == TT::STRING }
    assert string_tokens.any? { |t| t.value == "Alice" }, "Expected STRING 'Alice'"
  end

  def test_select_with_number_literal
    ast = parse("SELECT * FROM orders WHERE id = 42")

    all_tokens = collect_tokens(ast)
    number_tokens = all_tokens.select { |t| t.type == TT::NUMBER }
    assert number_tokens.any? { |t| t.value == "42" }, "Expected NUMBER 42"
  end

  def test_select_star
    ast = parse("SELECT * FROM orders")
    select_list_nodes = find_nodes_by_rule(ast, "select_list")
    assert select_list_nodes.length >= 1, "Expected select_list node"

    all_tokens = collect_tokens(ast)
    star_tokens = all_tokens.select { |t| t.type == TT::STAR }
    assert star_tokens.any?, "Expected STAR token for SELECT *"
  end

  def test_select_with_limit
    ast = parse("SELECT id FROM users LIMIT 10")
    kw = keyword_values(ast)
    assert_includes kw, "LIMIT"

    limit_nodes = find_nodes_by_rule(ast, "limit_clause")
    assert limit_nodes.length >= 1, "Expected limit_clause node"
  end

  def test_select_with_order_by
    ast = parse("SELECT id FROM users ORDER BY id ASC")
    kw = keyword_values(ast)
    assert_includes kw, "ORDER"
    assert_includes kw, "BY"
    assert_includes kw, "ASC"
  end

  def test_select_with_group_by
    ast = parse("SELECT dept, COUNT(id) FROM employees GROUP BY dept")
    kw = keyword_values(ast)
    assert_includes kw, "GROUP"
    assert_includes kw, "BY"
  end

  def test_select_distinct
    ast = parse("SELECT DISTINCT name FROM users")
    kw = keyword_values(ast)
    assert_includes kw, "DISTINCT"
  end

  def test_select_with_as_alias
    ast = parse("SELECT id AS user_id FROM users")
    kw = keyword_values(ast)
    assert_includes kw, "AS"
  end

  def test_select_with_and_condition
    ast = parse("SELECT * FROM t WHERE a = 1 AND b = 2")
    kw = keyword_values(ast)
    assert_includes kw, "AND"
  end

  def test_select_with_or_condition
    ast = parse("SELECT * FROM t WHERE a = 1 OR b = 2")
    kw = keyword_values(ast)
    assert_includes kw, "OR"
  end

  def test_select_with_not_equals
    ast = parse("SELECT * FROM t WHERE id != 0")
    all_tokens = collect_tokens(ast)
    ne_tokens = all_tokens.select { |t| t.type == NOT_EQUALS_TYPE }
    assert ne_tokens.any?, "Expected NOT_EQUALS token"
  end

  def test_select_with_less_equals
    ast = parse("SELECT * FROM t WHERE price <= 100")
    all_tokens = collect_tokens(ast)
    le_tokens = all_tokens.select { |t| t.type == LESS_EQUALS_TYPE }
    assert le_tokens.any?, "Expected LESS_EQUALS token"
  end

  def test_select_with_greater_equals
    ast = parse("SELECT * FROM t WHERE score >= 90")
    all_tokens = collect_tokens(ast)
    ge_tokens = all_tokens.select { |t| t.type == GREATER_EQUALS_TYPE }
    assert ge_tokens.any?, "Expected GREATER_EQUALS token"
  end

  def test_select_with_is_null
    ast = parse("SELECT * FROM users WHERE email IS NULL")
    kw = keyword_values(ast)
    assert_includes kw, "IS"
    assert_includes kw, "NULL"
  end

  def test_select_with_is_not_null
    ast = parse("SELECT * FROM users WHERE email IS NOT NULL")
    kw = keyword_values(ast)
    assert_includes kw, "IS"
    assert_includes kw, "NOT"
    assert_includes kw, "NULL"
  end

  def test_select_with_in_clause
    ast = parse("SELECT * FROM t WHERE id IN (1, 2, 3)")
    kw = keyword_values(ast)
    assert_includes kw, "IN"
  end

  def test_select_with_between
    ast = parse("SELECT * FROM products WHERE price BETWEEN 10 AND 100")
    kw = keyword_values(ast)
    assert_includes kw, "BETWEEN"
    assert_includes kw, "AND"
  end

  def test_select_with_like
    ast = parse("SELECT * FROM users WHERE name LIKE 'A%'")
    kw = keyword_values(ast)
    assert_includes kw, "LIKE"
  end

  def test_select_with_qualified_column
    # SELECT t.id FROM t
    ast = parse("SELECT t.id FROM t")
    all_tokens = collect_tokens(ast)
    dot_tokens = all_tokens.select { |t| t.type == TT::DOT }
    assert dot_tokens.any?, "Expected DOT token for qualified column t.id"
  end

  def test_select_function_call
    # SELECT COUNT(*) FROM t
    ast = parse("SELECT COUNT(id) FROM t")

    function_nodes = find_nodes_by_rule(ast, "function_call")
    assert function_nodes.length >= 1, "Expected function_call node for COUNT(id)"
  end

  def test_select_with_join
    # The grammar requires join_type before JOIN: INNER JOIN, LEFT JOIN, etc.
    # Plain JOIN without a qualifier is not in the grammar.
    ast = parse("SELECT u.id, o.total FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id")
    kw = keyword_values(ast)
    assert_includes kw, "INNER"
    assert_includes kw, "JOIN"
    assert_includes kw, "ON"
    assert_includes kw, "AS"
  end

  def test_select_with_left_join
    ast = parse("SELECT * FROM users LEFT JOIN orders ON users.id = orders.user_id")
    kw = keyword_values(ast)
    assert_includes kw, "LEFT"
    assert_includes kw, "JOIN"
    assert_includes kw, "ON"
  end

  def test_select_with_true_false
    ast = parse("SELECT * FROM users WHERE active = TRUE")
    kw = keyword_values(ast)
    assert_includes kw, "TRUE"
  end

  # ------------------------------------------------------------------
  # INSERT statements
  # ------------------------------------------------------------------

  def test_insert_statement
    # INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')
    ast = parse("INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')")
    assert_equal "program", ast.rule_name

    insert_nodes = find_nodes_by_rule(ast, "insert_stmt")
    assert insert_nodes.length >= 1, "Expected an insert_stmt node"

    kw = keyword_values(ast)
    assert_includes kw, "INSERT"
    assert_includes kw, "INTO"
    assert_includes kw, "VALUES"
  end

  def test_insert_values_contain_strings
    ast = parse("INSERT INTO users VALUES ('Bob')")
    all_tokens = collect_tokens(ast)
    string_tokens = all_tokens.select { |t| t.type == TT::STRING }
    assert string_tokens.any? { |t| t.value == "Bob" }
  end

  def test_insert_without_column_list
    # INSERT INTO ... VALUES (...) without specifying column names
    ast = parse("INSERT INTO orders VALUES (1, 99.99)")
    kw = keyword_values(ast)
    assert_includes kw, "INSERT"
    assert_includes kw, "VALUES"
  end

  # ------------------------------------------------------------------
  # UPDATE statements
  # ------------------------------------------------------------------

  def test_update_statement
    # UPDATE users SET name = 'Bob' WHERE id = 1
    ast = parse("UPDATE users SET name = 'Bob' WHERE id = 1")
    assert_equal "program", ast.rule_name

    update_nodes = find_nodes_by_rule(ast, "update_stmt")
    assert update_nodes.length >= 1, "Expected an update_stmt node"

    kw = keyword_values(ast)
    assert_includes kw, "UPDATE"
    assert_includes kw, "SET"
    assert_includes kw, "WHERE"
  end

  def test_update_multiple_assignments
    ast = parse("UPDATE users SET name = 'Bob', age = 30 WHERE id = 1")
    assignment_nodes = find_nodes_by_rule(ast, "assignment")
    assert_equal 2, assignment_nodes.length, "Expected 2 assignment nodes"
  end

  def test_update_without_where
    ast = parse("UPDATE counters SET value = 0")
    kw = keyword_values(ast)
    assert_includes kw, "UPDATE"
    assert_includes kw, "SET"
    refute_includes kw, "WHERE", "Should have no WHERE keyword"
  end

  # ------------------------------------------------------------------
  # DELETE statements
  # ------------------------------------------------------------------

  def test_delete_statement
    # DELETE FROM orders WHERE id = 42
    ast = parse("DELETE FROM orders WHERE id = 42")
    assert_equal "program", ast.rule_name

    delete_nodes = find_nodes_by_rule(ast, "delete_stmt")
    assert delete_nodes.length >= 1, "Expected a delete_stmt node"

    kw = keyword_values(ast)
    assert_includes kw, "DELETE"
    assert_includes kw, "FROM"
    assert_includes kw, "WHERE"
  end

  def test_delete_without_where
    ast = parse("DELETE FROM temp_data")
    kw = keyword_values(ast)
    assert_includes kw, "DELETE"
    assert_includes kw, "FROM"
  end

  # ------------------------------------------------------------------
  # CREATE TABLE statements
  # ------------------------------------------------------------------

  def test_create_table_statement
    sql = "CREATE TABLE users (id INTEGER, name VARCHAR)"
    ast = parse(sql)
    assert_equal "program", ast.rule_name

    create_nodes = find_nodes_by_rule(ast, "create_table_stmt")
    assert create_nodes.length >= 1, "Expected a create_table_stmt node"

    kw = keyword_values(ast)
    assert_includes kw, "CREATE"
    assert_includes kw, "TABLE"
  end

  def test_create_table_with_column_constraints
    sql = "CREATE TABLE products (id INTEGER NOT NULL, name TEXT NOT NULL, price DECIMAL DEFAULT 0)"
    ast = parse(sql)
    kw = keyword_values(ast)
    assert_includes kw, "CREATE"
    assert_includes kw, "TABLE"
    assert_includes kw, "NOT"
    assert_includes kw, "NULL"
    assert_includes kw, "DEFAULT"
  end

  def test_create_table_if_not_exists
    sql = "CREATE TABLE IF NOT EXISTS sessions (id INTEGER)"
    ast = parse(sql)
    kw = keyword_values(ast)
    assert_includes kw, "IF"
    assert_includes kw, "NOT"
    assert_includes kw, "EXISTS"
  end

  def test_create_table_with_primary_key
    sql = "CREATE TABLE users (id INTEGER PRIMARY KEY)"
    ast = parse(sql)
    kw = keyword_values(ast)
    assert_includes kw, "PRIMARY"
    assert_includes kw, "KEY"
  end

  # ------------------------------------------------------------------
  # DROP TABLE statements
  # ------------------------------------------------------------------

  def test_drop_table_statement
    ast = parse("DROP TABLE orders")
    assert_equal "program", ast.rule_name

    drop_nodes = find_nodes_by_rule(ast, "drop_table_stmt")
    assert drop_nodes.length >= 1, "Expected a drop_table_stmt node"

    kw = keyword_values(ast)
    assert_includes kw, "DROP"
    assert_includes kw, "TABLE"
  end

  def test_drop_table_if_exists
    ast = parse("DROP TABLE IF EXISTS temp_table")
    kw = keyword_values(ast)
    assert_includes kw, "IF"
    assert_includes kw, "EXISTS"
  end

  # ------------------------------------------------------------------
  # Multiple statements (separated by semicolons)
  # ------------------------------------------------------------------

  def test_multiple_statements
    ast = parse("SELECT 1 FROM a; SELECT 2 FROM b")
    assert_equal "program", ast.rule_name

    select_nodes = find_nodes_by_rule(ast, "select_stmt")
    assert_equal 2, select_nodes.length, "Expected 2 select_stmt nodes"
  end

  def test_multiple_statements_with_trailing_semicolon
    ast = parse("SELECT 1 FROM a;")
    assert_equal "program", ast.rule_name
    # The trailing semicolon should parse without error
  end

  # ------------------------------------------------------------------
  # Case-insensitive keywords
  # ------------------------------------------------------------------
  # Because sql.tokens has @case_insensitive true, the sql_lexer
  # normalizes keyword values to uppercase. The grammar matches
  # uppercase keyword literals, so lowercase input parses identically.

  def test_lowercase_select_parses
    ast = parse("select id from users")
    assert_equal "program", ast.rule_name
    kw = keyword_values(ast)
    assert_includes kw, "SELECT", "lowercase 'select' should be normalized to 'SELECT'"
    assert_includes kw, "FROM"
  end

  def test_mixed_case_keywords_parse
    ast = parse("Select Id From Users Where Id = 1")
    assert_equal "program", ast.rule_name
    kw = keyword_values(ast)
    assert_includes kw, "SELECT"
    assert_includes kw, "FROM"
    assert_includes kw, "WHERE"
  end

  def test_all_lowercase_insert_parses
    ast = parse("insert into users values ('Alice')")
    assert_equal "program", ast.rule_name
    kw = keyword_values(ast)
    assert_includes kw, "INSERT"
    assert_includes kw, "INTO"
    assert_includes kw, "VALUES"
  end

  # ------------------------------------------------------------------
  # Expressions
  # ------------------------------------------------------------------

  def test_arithmetic_expression_in_select
    ast = parse("SELECT price * 1.1 FROM products")
    all_tokens = collect_tokens(ast)
    star_tokens = all_tokens.select { |t| t.type == TT::STAR }
    assert star_tokens.any?, "Expected STAR token for multiplication"
  end

  def test_nested_expression_with_parens
    ast = parse("SELECT * FROM t WHERE (a = 1 OR b = 2) AND c = 3")
    kw = keyword_values(ast)
    assert_includes kw, "OR"
    assert_includes kw, "AND"
  end

  def test_not_expression
    ast = parse("SELECT * FROM t WHERE NOT active = TRUE")
    kw = keyword_values(ast)
    assert_includes kw, "NOT"
  end

  # ------------------------------------------------------------------
  # Error cases
  # ------------------------------------------------------------------

  def test_error_on_empty_input
    assert_raises(StandardError) { parse("") }
  end

  def test_error_on_select_without_from
    # "SELECT *" alone without FROM is incomplete per the grammar
    assert_raises(StandardError) { parse("SELECT *") }
  end

  def test_error_on_unknown_token
    assert_raises(StandardError) { parse("@@invalid") }
  end
end
