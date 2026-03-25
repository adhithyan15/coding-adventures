defmodule CodingAdventures.SqlParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.SqlParser
  alias CodingAdventures.Parser.ASTNode

  # ---------------------------------------------------------------------------
  # create_sql_parser/1
  # ---------------------------------------------------------------------------
  #
  # The function parses sql.grammar and returns a ParserGrammar.  We verify
  # that the grammar contains the expected top-level rules.

  describe "create_sql_parser/1" do
    test "returns {:ok, grammar} for the default grammar" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      assert grammar != nil
    end

    test "grammar contains the program rule (entry point)" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "program" in names
    end

    test "grammar contains statement rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "statement" in names
    end

    test "grammar contains select_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "select_stmt" in names
    end

    test "grammar contains insert_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "insert_stmt" in names
    end

    test "grammar contains update_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "update_stmt" in names
    end

    test "grammar contains delete_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "delete_stmt" in names
    end

    test "grammar contains create_table_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "create_table_stmt" in names
    end

    test "grammar contains drop_table_stmt rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "drop_table_stmt" in names
    end

    test "grammar contains expr rule" do
      {:ok, grammar} = SqlParser.create_sql_parser()
      names = Enum.map(grammar.rules, & &1.name)
      assert "expr" in names
    end

    test "returns {:error, _} for a non-existent grammar path" do
      {:error, _msg} = SqlParser.create_sql_parser("/tmp/does_not_exist_xyz/")
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — SELECT statements
  # ---------------------------------------------------------------------------
  #
  # SELECT is the most commonly used statement.  We test basic selects, star
  # selects, WHERE clauses, column lists, aliases, and ORDER/LIMIT clauses.

  describe "parse_sql/1 — SELECT" do
    test "SELECT * FROM table" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t")
      assert node.rule_name == "program"
    end

    test "SELECT with column list" do
      {:ok, node} = SqlParser.parse_sql("SELECT id, name FROM users")
      assert node.rule_name == "program"
    end

    test "SELECT with WHERE clause" do
      {:ok, node} = SqlParser.parse_sql("SELECT id FROM users WHERE id = 1")
      assert node.rule_name == "program"
    end

    test "SELECT with LIMIT" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t LIMIT 10")
      assert node.rule_name == "program"
    end

    test "SELECT with LIMIT OFFSET" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t LIMIT 10 OFFSET 20")
      assert node.rule_name == "program"
    end

    test "SELECT with ORDER BY ASC" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t ORDER BY name ASC")
      assert node.rule_name == "program"
    end

    test "SELECT with ORDER BY DESC" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t ORDER BY name DESC")
      assert node.rule_name == "program"
    end

    test "SELECT with GROUP BY" do
      {:ok, node} = SqlParser.parse_sql("SELECT dept, COUNT(*) FROM employees GROUP BY dept")
      assert node.rule_name == "program"
    end

    test "SELECT with HAVING" do
      {:ok, node} = SqlParser.parse_sql(
        "SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 5"
      )
      assert node.rule_name == "program"
    end

    test "SELECT DISTINCT" do
      {:ok, node} = SqlParser.parse_sql("SELECT DISTINCT name FROM users")
      assert node.rule_name == "program"
    end

    test "SELECT with column alias" do
      {:ok, node} = SqlParser.parse_sql("SELECT id AS user_id FROM users")
      assert node.rule_name == "program"
    end

    test "SELECT with table alias" do
      {:ok, node} = SqlParser.parse_sql("SELECT u.id FROM users AS u")
      assert node.rule_name == "program"
    end

    test "SELECT with qualified column (table.column)" do
      # table_ref = table_name [ "AS" NAME ], so aliases require the AS keyword
      {:ok, node} = SqlParser.parse_sql("SELECT u.name FROM users AS u")
      assert node.rule_name == "program"
    end

    test "SELECT with number literal" do
      {:ok, node} = SqlParser.parse_sql("SELECT 42 FROM t")
      assert node.rule_name == "program"
    end

    test "SELECT with string literal" do
      {:ok, node} = SqlParser.parse_sql("SELECT 'hello' FROM t")
      assert node.rule_name == "program"
    end

    test "SELECT with NULL literal" do
      {:ok, node} = SqlParser.parse_sql("SELECT NULL FROM t")
      assert node.rule_name == "program"
    end

    test "SELECT with TRUE literal" do
      {:ok, node} = SqlParser.parse_sql("SELECT TRUE FROM t")
      assert node.rule_name == "program"
    end

    test "SELECT with FALSE literal" do
      {:ok, node} = SqlParser.parse_sql("SELECT FALSE FROM t")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — case-insensitive keywords
  # ---------------------------------------------------------------------------
  #
  # Because the lexer normalizes keywords to uppercase and the grammar matches
  # uppercase literals, every valid case variation should parse identically.

  describe "parse_sql/1 — case-insensitive keywords" do
    test "lowercase select from" do
      {:ok, node} = SqlParser.parse_sql("select * from t")
      assert node.rule_name == "program"
    end

    test "mixed-case keywords" do
      {:ok, node} = SqlParser.parse_sql("Select Id From Users Where Id = 1")
      assert node.rule_name == "program"
    end

    test "ALL CAPS multi-keyword statement" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE id > 0 ORDER BY id ASC LIMIT 5")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — WHERE expressions
  # ---------------------------------------------------------------------------
  #
  # The grammar supports a rich expression language with comparison, boolean
  # logic, BETWEEN, IN, LIKE, and IS NULL.

  describe "parse_sql/1 — WHERE expressions" do
    test "WHERE with equality" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a = 1")
      assert node.rule_name == "program"
    end

    test "WHERE with NOT EQUALS" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a != 1")
      assert node.rule_name == "program"
    end

    test "WHERE with ANSI NOT EQUALS <>" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a <> 1")
      assert node.rule_name == "program"
    end

    test "WHERE with LESS THAN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a < 10")
      assert node.rule_name == "program"
    end

    test "WHERE with GREATER THAN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a > 10")
      assert node.rule_name == "program"
    end

    test "WHERE with LESS_EQUALS" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a <= 10")
      assert node.rule_name == "program"
    end

    test "WHERE with GREATER_EQUALS" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a >= 10")
      assert node.rule_name == "program"
    end

    test "WHERE with AND" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a = 1 AND b = 2")
      assert node.rule_name == "program"
    end

    test "WHERE with OR" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE a = 1 OR b = 2")
      assert node.rule_name == "program"
    end

    test "WHERE with NOT" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE NOT a = 1")
      assert node.rule_name == "program"
    end

    test "WHERE with BETWEEN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE age BETWEEN 18 AND 65")
      assert node.rule_name == "program"
    end

    test "WHERE with NOT BETWEEN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE age NOT BETWEEN 18 AND 65")
      assert node.rule_name == "program"
    end

    test "WHERE with IN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE id IN (1, 2, 3)")
      assert node.rule_name == "program"
    end

    test "WHERE with NOT IN" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE id NOT IN (1, 2, 3)")
      assert node.rule_name == "program"
    end

    test "WHERE with LIKE" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE name LIKE 'A%'")
      assert node.rule_name == "program"
    end

    test "WHERE with NOT LIKE" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE name NOT LIKE 'A%'")
      assert node.rule_name == "program"
    end

    test "WHERE with IS NULL" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE email IS NULL")
      assert node.rule_name == "program"
    end

    test "WHERE with IS NOT NULL" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE email IS NOT NULL")
      assert node.rule_name == "program"
    end

    test "parenthesized expression" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE (a = 1 OR b = 2) AND c = 3")
      assert node.rule_name == "program"
    end

    test "arithmetic in WHERE" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t WHERE price * 1 > 100")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — INSERT statements
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — INSERT" do
    test "INSERT INTO … VALUES" do
      {:ok, node} = SqlParser.parse_sql("INSERT INTO users VALUES (1, 'Alice')")
      assert node.rule_name == "program"
    end

    test "INSERT INTO with column list" do
      {:ok, node} = SqlParser.parse_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')")
      assert node.rule_name == "program"
    end

    test "INSERT with multiple rows" do
      {:ok, node} = SqlParser.parse_sql(
        "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')"
      )
      assert node.rule_name == "program"
    end

    test "INSERT with NULL value" do
      {:ok, node} = SqlParser.parse_sql("INSERT INTO t (id, email) VALUES (1, NULL)")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — UPDATE statements
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — UPDATE" do
    test "UPDATE … SET … WHERE" do
      {:ok, node} = SqlParser.parse_sql("UPDATE users SET name = 'Bob' WHERE id = 1")
      assert node.rule_name == "program"
    end

    test "UPDATE with multiple assignments" do
      {:ok, node} = SqlParser.parse_sql(
        "UPDATE users SET name = 'Bob', age = 30 WHERE id = 1"
      )
      assert node.rule_name == "program"
    end

    test "UPDATE without WHERE" do
      {:ok, node} = SqlParser.parse_sql("UPDATE t SET active = 0")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — DELETE statements
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — DELETE" do
    test "DELETE FROM … WHERE" do
      {:ok, node} = SqlParser.parse_sql("DELETE FROM users WHERE id = 1")
      assert node.rule_name == "program"
    end

    test "DELETE without WHERE (delete all rows)" do
      {:ok, node} = SqlParser.parse_sql("DELETE FROM t")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — CREATE TABLE
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — CREATE TABLE" do
    test "CREATE TABLE with columns" do
      {:ok, node} = SqlParser.parse_sql("""
        CREATE TABLE users (
          id   INTEGER,
          name VARCHAR
        )
      """)
      assert node.rule_name == "program"
    end

    test "CREATE TABLE IF NOT EXISTS" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE IF NOT EXISTS logs (id INTEGER, msg TEXT)"
      )
      assert node.rule_name == "program"
    end

    test "CREATE TABLE with NOT NULL constraint" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE t (id INTEGER NOT NULL, name VARCHAR NOT NULL)"
      )
      assert node.rule_name == "program"
    end

    test "CREATE TABLE with NULL constraint" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE t (id INTEGER, email VARCHAR NULL)"
      )
      assert node.rule_name == "program"
    end

    test "CREATE TABLE with PRIMARY KEY constraint" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name VARCHAR)"
      )
      assert node.rule_name == "program"
    end

    test "CREATE TABLE with UNIQUE constraint" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE t (id INTEGER, email VARCHAR UNIQUE)"
      )
      assert node.rule_name == "program"
    end

    test "CREATE TABLE with DEFAULT value" do
      {:ok, node} = SqlParser.parse_sql(
        "CREATE TABLE t (id INTEGER, active INTEGER DEFAULT 1)"
      )
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — DROP TABLE
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — DROP TABLE" do
    test "DROP TABLE table_name" do
      {:ok, node} = SqlParser.parse_sql("DROP TABLE users")
      assert node.rule_name == "program"
    end

    test "DROP TABLE IF EXISTS" do
      {:ok, node} = SqlParser.parse_sql("DROP TABLE IF EXISTS temp_data")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — multiple statements
  # ---------------------------------------------------------------------------
  #
  # The grammar's program rule: statement { ";" statement } [ ";" ]
  # so multiple semicolon-separated statements are valid.

  describe "parse_sql/1 — multiple statements" do
    test "two statements separated by semicolon" do
      {:ok, node} = SqlParser.parse_sql("SELECT 1 FROM t; SELECT 2 FROM t")
      assert node.rule_name == "program"
    end

    test "trailing semicolon is accepted" do
      {:ok, node} = SqlParser.parse_sql("SELECT * FROM t;")
      assert node.rule_name == "program"
    end

    test "multiple statements with trailing semicolons" do
      {:ok, node} = SqlParser.parse_sql(
        "INSERT INTO t VALUES (1); DELETE FROM t WHERE id = 2;"
      )
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — function calls
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — function calls" do
    test "COUNT(*)" do
      {:ok, node} = SqlParser.parse_sql("SELECT COUNT(*) FROM t")
      assert node.rule_name == "program"
    end

    test "function with argument" do
      {:ok, node} = SqlParser.parse_sql("SELECT MAX(age) FROM users")
      assert node.rule_name == "program"
    end

    test "function with multiple arguments" do
      {:ok, node} = SqlParser.parse_sql("SELECT COALESCE(a, b, 0) FROM t")
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — JOIN clauses
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — JOINs" do
    test "INNER JOIN" do
      # table_ref requires AS for aliases: "users AS u" not "users u"
      {:ok, node} = SqlParser.parse_sql(
        "SELECT u.id, o.total FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id"
      )
      assert node.rule_name == "program"
    end

    test "LEFT JOIN" do
      {:ok, node} = SqlParser.parse_sql(
        "SELECT * FROM users AS u LEFT JOIN orders AS o ON u.id = o.user_id"
      )
      assert node.rule_name == "program"
    end

    test "LEFT OUTER JOIN" do
      {:ok, node} = SqlParser.parse_sql(
        "SELECT * FROM a LEFT OUTER JOIN b ON a.id = b.aid"
      )
      assert node.rule_name == "program"
    end

    test "CROSS JOIN" do
      {:ok, node} = SqlParser.parse_sql(
        "SELECT * FROM a CROSS JOIN b ON a.id = b.id"
      )
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — whitespace and formatting
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — whitespace" do
    test "multiline query with indentation" do
      source = """
      SELECT
        id,
        name,
        email
      FROM
        users
      WHERE
        active = 1
      ORDER BY
        name ASC
      LIMIT
        100
      """

      {:ok, node} = SqlParser.parse_sql(source)
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — comments are ignored
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — comments" do
    test "line comments are ignored" do
      source = """
      SELECT id -- primary key
      FROM users -- the user table
      WHERE active = 1
      """

      {:ok, node} = SqlParser.parse_sql(source)
      assert node.rule_name == "program"
    end

    test "block comments are ignored" do
      {:ok, node} = SqlParser.parse_sql(
        "SELECT /* all */ * FROM /* the */ t"
      )
      assert node.rule_name == "program"
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — ASTNode helpers
  # ---------------------------------------------------------------------------

  describe "ASTNode helpers" do
    test "leaf? is false for program node" do
      {:ok, node} = SqlParser.parse_sql("SELECT 1 FROM t")
      refute ASTNode.leaf?(node)
    end

    test "program node has children" do
      {:ok, node} = SqlParser.parse_sql("SELECT 1 FROM t")
      assert length(node.children) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # parse_sql/1 — error cases
  # ---------------------------------------------------------------------------

  describe "parse_sql/1 — errors" do
    test "incomplete SELECT (no FROM) returns error" do
      {:error, msg} = SqlParser.parse_sql("SELECT *")
      assert msg =~ "Parse error" or msg =~ "Unexpected" or msg =~ "Expected"
    end

    test "totally invalid SQL returns error" do
      {:error, _msg} = SqlParser.parse_sql("NOT VALID SQL !!!")
    end

    test "unexpected character returns error" do
      {:error, _msg} = SqlParser.parse_sql("@@@")
    end

    test "empty string returns error (no statement)" do
      # The grammar's entry is `program = statement …`, so empty input fails
      # because there's no statement at all.
      {:error, _msg} = SqlParser.parse_sql("")
    end
  end
end
