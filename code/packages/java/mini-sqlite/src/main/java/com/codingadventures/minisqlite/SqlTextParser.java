package com.codingadventures.minisqlite;

// SqlTextParser.java — hand-written recursive-descent SQL parser for mini-sqlite Level 1.
//
// Purpose
// ───────
// The Java sql-parser package is currently a stub (only exposes ping()).
// mini-sqlite Level 1 therefore includes its own text-to-AST parser so that
// execute(String sql) can feed real SqlPlanner.Statement objects into the
// full pipeline:
//
//   SqlTextParser.parse(sql) → Statement
//      ↓
//   SqlPlanner.plan(stmt)   → LogicalPlan
//      ↓
//   SqlOptimizer.optimize() → OptimizedPlan
//      ↓
//   SqlCodegen.compile()    → Program
//      ↓
//   SqlVm.execute()         → QueryResult
//
// Supported grammar
// ─────────────────
//   SELECT [DISTINCT] (* | col [AS alias] [, ...])
//          FROM table [AS alias]
//          [WHERE expr]
//          [GROUP BY expr [, ...]]
//          [HAVING expr]
//          [ORDER BY expr [ASC|DESC] [NULLS FIRST|LAST] [, ...]]
//          [LIMIT n [OFFSET m]]
//   INSERT INTO table [(col, ...)] VALUES (v, ...) [, ...]
//   UPDATE table SET col = expr [, ...] [WHERE expr]
//   DELETE FROM table [WHERE expr]
//   CREATE TABLE [IF NOT EXISTS] table (colDef, ...)
//   DROP TABLE [IF EXISTS] table
//
// Scalar expressions include:
//   literals (NULL, TRUE, FALSE, integer, real, 'string')
//   column references (col or table.col)
//   arithmetic: +, -, *, /, %
//   comparisons: =, <>, !=, <, <=, >, >=
//   logical: AND, OR, NOT
//   IS NULL / IS NOT NULL
//   BETWEEN … AND …
//   IN (list)
//   LIKE pattern
//   aggregate calls: COUNT(*), SUM(e), AVG(e), MIN(e), MAX(e), COUNT(DISTINCT e)
//   scalar function calls: upper(e), lower(e), length(e), trim(e), etc.
//
// Three-valued SQL NULL logic is handled at runtime by the VM;
// the parser just builds AST nodes.

import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqlplanner.SqlPlanner.AggArg;
import com.codingadventures.sqlplanner.SqlPlanner.AggFunction;
import com.codingadventures.sqlplanner.SqlPlanner.Assignment;
import com.codingadventures.sqlplanner.SqlPlanner.BinaryOperator;
import com.codingadventures.sqlplanner.SqlPlanner.ColumnDef;
import com.codingadventures.sqlplanner.SqlPlanner.JoinClause;
import com.codingadventures.sqlplanner.SqlPlanner.JoinKind;
import com.codingadventures.sqlplanner.SqlPlanner.LimitClause;
import com.codingadventures.sqlplanner.SqlPlanner.NullOrder;
import com.codingadventures.sqlplanner.SqlPlanner.OutputColumn;
import com.codingadventures.sqlplanner.SqlPlanner.SortDir;
import com.codingadventures.sqlplanner.SqlPlanner.SortKey;
import com.codingadventures.sqlplanner.SqlPlanner.SqlExpr;
import com.codingadventures.sqlplanner.SqlPlanner.Statement;
import com.codingadventures.sqlplanner.SqlPlanner.TableRef;
import com.codingadventures.sqlplanner.SqlPlanner.UnaryOperator;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * Recursive-descent parser that converts a SQL string into a
 * {@link Statement} ready for the sql-planner pipeline.
 *
 * <p>This parser is intentionally limited to the SQL subset used by the
 * mini-sqlite conformance suite and the Level-1 feature matrix described in
 * {@code mini-sqlite-porting.md}.  Unknown syntax is reported as a
 * {@link MiniSqliteConnection.MiniSqliteException} with kind {@code OperationalError}.
 */
final class SqlTextParser {

    // ── Public entry point ────────────────────────────────────────────────────

    /**
     * Parse a single SQL statement from {@code sql} and return the AST.
     *
     * @param sql the SQL text to parse (may end with an optional semicolon)
     * @return the parsed {@link Statement}
     * @throws MiniSqliteConnection.MiniSqliteException with kind {@code OperationalError}
     *         if the input is syntactically invalid or uses unsupported syntax
     */
    static Statement parse(String sql) {
        SqlTextParser p = new SqlTextParser(sql);
        Statement stmt = p.parseStatement();
        p.expectEnd();
        return stmt;
    }

    // ── Tokeniser ─────────────────────────────────────────────────────────────
    //
    // We tokenise lazily: `advance()` advances the position and fills `tok`
    // and `tokRaw`.  Case-insensitive keyword matching is always done on the
    // upper-cased `tok`; `tokRaw` preserves the original text for literals.

    private final String src;
    private int pos = 0;

    // Current token (upper-cased for keyword matching).
    private String tok = "";
    // Raw (un-upper-cased) text of the current token — needed for string literals.
    private String tokRaw = "";

    // Token kinds (rough categories used by the parser).
    private enum TokKind { KEYWORD, IDENT, INTEGER, REAL, STRING, PUNCT, EOF }
    private TokKind kind = TokKind.EOF;

    private SqlTextParser(String sql) {
        this.src = sql == null ? "" : sql;
        advance();  // prime the lookahead
    }

    // Skip whitespace and advance to the next token.
    private void advance() {
        skipWs();
        if (pos >= src.length()) {
            tok = "";
            tokRaw = "";
            kind = TokKind.EOF;
            return;
        }
        char c = src.charAt(pos);

        // ── String literals ───────────────────────────────────────────────
        if (c == '\'' || c == '"') {
            int start = pos;
            char quote = c;
            pos++;
            StringBuilder sb = new StringBuilder();
            while (pos < src.length()) {
                char ch = src.charAt(pos);
                if (ch == quote) {
                    pos++;
                    // doubled quote → literal quote character
                    if (pos < src.length() && src.charAt(pos) == quote) {
                        sb.append(quote);
                        pos++;
                    } else {
                        break;
                    }
                } else {
                    sb.append(ch);
                    pos++;
                }
            }
            tokRaw = sb.toString();   // value without surrounding quotes
            tok    = tokRaw;
            kind   = TokKind.STRING;
            return;
        }

        // ── Numeric literals ──────────────────────────────────────────────
        if (Character.isDigit(c) || (c == '-' && pos + 1 < src.length() && Character.isDigit(src.charAt(pos + 1)) && !isIdentChar(peekBefore()))) {
            int start = pos;
            if (c == '-') pos++;
            while (pos < src.length() && Character.isDigit(src.charAt(pos))) pos++;
            boolean isReal = false;
            if (pos < src.length() && src.charAt(pos) == '.') {
                isReal = true;
                pos++;
                while (pos < src.length() && Character.isDigit(src.charAt(pos))) pos++;
            }
            tokRaw = src.substring(start, pos);
            tok    = tokRaw;
            kind   = isReal ? TokKind.REAL : TokKind.INTEGER;
            return;
        }

        // ── Identifiers and keywords ──────────────────────────────────────
        if (Character.isLetter(c) || c == '_') {
            int start = pos;
            while (pos < src.length() && isIdentChar(src.charAt(pos))) pos++;
            tokRaw = src.substring(start, pos);
            tok    = tokRaw.toUpperCase(Locale.ROOT);
            kind   = KEYWORDS.contains(tok) ? TokKind.KEYWORD : TokKind.IDENT;
            return;
        }

        // ── Back-tick quoted identifiers ──────────────────────────────────
        if (c == '`') {
            pos++;
            int start = pos;
            while (pos < src.length() && src.charAt(pos) != '`') pos++;
            tokRaw = src.substring(start, pos);
            if (pos < src.length()) pos++; // consume closing `
            tok    = tokRaw.toUpperCase(Locale.ROOT);
            kind   = TokKind.IDENT;
            return;
        }

        // ── Two-char punctuation ──────────────────────────────────────────
        if (pos + 1 < src.length()) {
            String two = src.substring(pos, pos + 2);
            switch (two) {
                case "<>", "!=", "<=", ">=" -> {
                    tok = tokRaw = two;
                    kind = TokKind.PUNCT;
                    pos += 2;
                    return;
                }
            }
        }

        // ── Single-char punctuation ───────────────────────────────────────
        tok = tokRaw = String.valueOf(c);
        kind = TokKind.PUNCT;
        pos++;
    }

    private char peekBefore() {
        if (pos == 0) return 0;
        // Look at the last non-whitespace char before pos.
        int i = pos - 1;
        while (i >= 0 && src.charAt(i) == ' ') i--;
        return i >= 0 ? src.charAt(i) : 0;
    }

    private void skipWs() {
        while (pos < src.length()) {
            char c = src.charAt(pos);
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
                pos++;
            } else if (c == '-' && pos + 1 < src.length() && src.charAt(pos + 1) == '-') {
                // SQL line comment
                while (pos < src.length() && src.charAt(pos) != '\n') pos++;
            } else if (c == '/' && pos + 1 < src.length() && src.charAt(pos + 1) == '*') {
                // block comment
                pos += 2;
                while (pos + 1 < src.length() && !(src.charAt(pos) == '*' && src.charAt(pos + 1) == '/')) pos++;
                if (pos + 1 < src.length()) pos += 2;
            } else {
                break;
            }
        }
    }

    private static boolean isIdentChar(char c) {
        return Character.isLetterOrDigit(c) || c == '_';
    }

    // ── Keyword set ────────────────────────────────────────────────────────────

    private static final java.util.Set<String> KEYWORDS = new java.util.HashSet<>(java.util.Arrays.asList(
        "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "ASC", "DESC",
        "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
        "CREATE", "TABLE", "IF", "NOT", "EXISTS", "DROP", "DISTINCT",
        "AND", "OR", "NOT", "IS", "NULL", "BETWEEN", "IN", "LIKE",
        "TRUE", "FALSE", "AS", "INNER", "LEFT", "RIGHT", "FULL", "CROSS",
        "JOIN", "ON", "NULLS", "FIRST", "LAST", "COUNT", "SUM", "AVG", "MIN", "MAX",
        "CONCAT", "COLLATE"
    ));

    // ── Token consumption helpers ─────────────────────────────────────────────

    /** Return true if current token equals {@code kw} (case-insensitive). */
    private boolean at(String kw) {
        return tok.equalsIgnoreCase(kw);
    }

    /** Return true if current token equals any of the given options. */
    private boolean atAny(String... opts) {
        for (String o : opts) if (at(o)) return true;
        return false;
    }

    /**
     * Consume the current token if it matches {@code kw} and advance.
     *
     * @return true if consumed, false otherwise
     */
    private boolean eat(String kw) {
        if (at(kw)) { advance(); return true; }
        return false;
    }

    /**
     * Consume the current token if it matches any of the given options.
     *
     * @return the matched token in upper-case, or null
     */
    private String eatAny(String... opts) {
        for (String o : opts) {
            if (at(o)) { String t = tok; advance(); return t; }
        }
        return null;
    }

    /** Consume {@code kw} or throw a parse error. */
    private void expect(String kw) {
        if (!eat(kw)) {
            throw parseError("expected '" + kw + "' but got '" + tok + "'");
        }
    }

    /** Assert that we've consumed all input (ignoring trailing semicolons). */
    private void expectEnd() {
        // skip optional trailing semicolons
        while (eat(";")) {}
        if (kind != TokKind.EOF) {
            throw parseError("unexpected token '" + tok + "' after statement");
        }
    }

    /** Read an identifier (keyword or plain identifier) and return it. */
    private String readIdent() {
        if (kind == TokKind.EOF || at(";")) {
            throw parseError("expected identifier but got '" + tok + "'");
        }
        // Allow keywords to be used as identifiers when unambiguous.
        String name = tokRaw;
        advance();
        return name;
    }

    private MiniSqliteConnection.MiniSqliteException parseError(String msg) {
        return new MiniSqliteConnection.MiniSqliteException("OperationalError",
            "SQL parse error near position " + pos + ": " + msg);
    }

    // ── Statement dispatch ────────────────────────────────────────────────────

    private Statement parseStatement() {
        // strip leading semicolons from prior statements
        while (eat(";")) {}
        return switch (tok) {
            case "SELECT" -> parseSelect();
            case "INSERT" -> parseInsert();
            case "UPDATE" -> parseUpdate();
            case "DELETE" -> parseDelete();
            case "CREATE" -> parseCreate();
            case "DROP"   -> parseDrop();
            default -> throw parseError("unsupported statement keyword '" + tok + "'");
        };
    }

    // ── SELECT ────────────────────────────────────────────────────────────────

    private Statement.Select parseSelect() {
        expect("SELECT");
        boolean distinct = eat("DISTINCT");

        // Column list
        List<OutputColumn> columns = parseSelectColumns();

        // FROM clause (optional in some engines but required for table queries)
        List<TableRef> from = new ArrayList<>();
        List<JoinClause> joins = new ArrayList<>();
        if (eat("FROM")) {
            from.add(parseTableRef());
            // JOIN clauses
            while (atAny("INNER", "LEFT", "RIGHT", "FULL", "CROSS", "JOIN")) {
                joins.add(parseJoinClause());
            }
        }

        SqlExpr where = null;
        if (eat("WHERE")) {
            where = parseExpr();
        }

        List<SqlExpr> groupBy = new ArrayList<>();
        if (eat("GROUP")) {
            expect("BY");
            groupBy.add(parseExpr());
            while (eat(",")) groupBy.add(parseExpr());
        }

        SqlExpr having = null;
        if (eat("HAVING")) {
            having = parseExpr();
        }

        List<SortKey> orderBy = new ArrayList<>();
        if (eat("ORDER")) {
            expect("BY");
            orderBy.add(parseSortKey());
            while (eat(",")) orderBy.add(parseSortKey());
        }

        LimitClause limit = null;
        if (eat("LIMIT")) {
            long count = readLong("LIMIT count");
            Long offset = null;
            if (eat("OFFSET")) {
                offset = readLong("OFFSET value");
            } else if (eat(",")) {
                // MySQL-style: LIMIT offset, count (swap them)
                offset = count;
                count  = readLong("LIMIT second value");
            }
            limit = new LimitClause(count, offset);
        }

        return new Statement.Select(distinct, columns, from, joins, where, groupBy, having, orderBy, limit);
    }

    private long readLong(String context) {
        if (kind != TokKind.INTEGER) {
            throw parseError(context + " must be an integer, got '" + tok + "'");
        }
        long v = Long.parseLong(tok);
        advance();
        return v;
    }

    private List<OutputColumn> parseSelectColumns() {
        List<OutputColumn> cols = new ArrayList<>();
        if (at("*")) {
            advance();
            cols.add(new OutputColumn.Star());
            return cols;
        }
        cols.add(parseSelectColumn());
        while (eat(",")) {
            cols.add(parseSelectColumn());
        }
        return cols;
    }

    private OutputColumn parseSelectColumn() {
        // COUNT(*) — special case before general expr to avoid consuming '*' as wildcard
        if ((at("COUNT") || at("SUM") || at("AVG") || at("MIN") || at("MAX")) && peekIsLParen()) {
            SqlExpr aggExpr = parseAggOrFunc();
            String alias = null;
            if (eat("AS")) alias = readIdent();
            else if (kind == TokKind.IDENT && !atAny("FROM", "WHERE", "GROUP", "HAVING", "ORDER", "LIMIT", "UNION")) {
                alias = readIdent();
            }
            return new OutputColumn.Expr(aggExpr, alias);
        }
        SqlExpr expr = parseExpr();
        String alias = null;
        if (eat("AS")) alias = readIdent();
        else if (kind == TokKind.IDENT && !atAny("FROM", "WHERE", "GROUP", "HAVING", "ORDER", "LIMIT", "UNION")) {
            alias = readIdent();
        }
        return new OutputColumn.Expr(expr, alias);
    }

    private boolean peekIsLParen() {
        // Save position and check what comes after consuming the current keyword.
        int savedPos = pos;
        String savedTok = tok;
        String savedRaw = tokRaw;
        TokKind savedKind = kind;
        advance();
        boolean result = at("(");
        // Restore
        pos  = savedPos;
        tok  = savedTok;
        tokRaw = savedRaw;
        kind = savedKind;
        return result;
    }

    private TableRef parseTableRef() {
        String table = readIdent();
        String alias = null;
        if (eat("AS")) alias = readIdent();
        else if (kind == TokKind.IDENT && !atAny("WHERE", "GROUP", "HAVING", "ORDER", "LIMIT", "INNER", "LEFT", "RIGHT", "FULL", "CROSS", "JOIN", "ON", "UNION")) {
            alias = readIdent();
        }
        return new TableRef(table, alias);
    }

    private JoinClause parseJoinClause() {
        JoinKind jk;
        if (eat("INNER")) { expect("JOIN"); jk = JoinKind.INNER; }
        else if (eat("LEFT"))  { eat("OUTER"); expect("JOIN"); jk = JoinKind.LEFT; }
        else if (eat("RIGHT")) { eat("OUTER"); expect("JOIN"); jk = JoinKind.RIGHT; }
        else if (eat("FULL"))  { eat("OUTER"); expect("JOIN"); jk = JoinKind.FULL; }
        else if (eat("CROSS")) { expect("JOIN"); jk = JoinKind.CROSS; }
        else { expect("JOIN"); jk = JoinKind.INNER; }

        String table = readIdent();
        String alias = null;
        if (eat("AS")) alias = readIdent();
        SqlExpr on = null;
        if (eat("ON")) on = parseExpr();
        return new JoinClause(jk, table, alias, on);
    }

    private SortKey parseSortKey() {
        SqlExpr expr = parseExpr();
        SortDir dir = SortDir.ASC;
        if (eat("DESC")) dir = SortDir.DESC;
        else eat("ASC");

        // Default null ordering matches SqlVm's sort semantics:
        //   ASC  → NULLS FIRST (null rank = 0, sorts before all values)
        //   DESC → NULLS LAST  (null rank = 2, sorts before reversed values)
        NullOrder nulls = (dir == SortDir.ASC) ? NullOrder.NULLS_FIRST : NullOrder.NULLS_LAST;
        if (eat("NULLS")) {
            if (eat("FIRST")) nulls = NullOrder.NULLS_FIRST;
            else { expect("LAST"); nulls = NullOrder.NULLS_LAST; }
        }
        return new SortKey(expr, dir, nulls);
    }

    // ── INSERT ────────────────────────────────────────────────────────────────

    private Statement.Insert parseInsert() {
        expect("INSERT");
        expect("INTO");
        String table = readIdent();

        List<String> cols = new ArrayList<>();
        if (eat("(")) {
            cols.add(readIdent());
            while (eat(",")) cols.add(readIdent());
            expect(")");
        }

        expect("VALUES");
        List<List<SqlExpr>> rows = new ArrayList<>();
        rows.add(parseValueRow());
        while (eat(",")) rows.add(parseValueRow());

        return new Statement.Insert(table, cols, rows);
    }

    private List<SqlExpr> parseValueRow() {
        expect("(");
        List<SqlExpr> vals = new ArrayList<>();
        vals.add(parsePrimary()); // values may be just literals
        while (eat(",")) vals.add(parsePrimary());
        expect(")");
        return vals;
    }

    // ── UPDATE ────────────────────────────────────────────────────────────────

    private Statement.Update parseUpdate() {
        expect("UPDATE");
        String table = readIdent();
        expect("SET");
        List<Assignment> assignments = new ArrayList<>();
        assignments.add(parseAssignment());
        while (eat(",")) assignments.add(parseAssignment());
        SqlExpr where = null;
        if (eat("WHERE")) where = parseExpr();
        return new Statement.Update(table, assignments, where);
    }

    private Assignment parseAssignment() {
        String col = readIdent();
        expect("=");
        SqlExpr val = parseExpr();
        return new Assignment(col, val);
    }

    // ── DELETE ────────────────────────────────────────────────────────────────

    private Statement.Delete parseDelete() {
        expect("DELETE");
        expect("FROM");
        String table = readIdent();
        SqlExpr where = null;
        if (eat("WHERE")) where = parseExpr();
        return new Statement.Delete(table, where);
    }

    // ── CREATE TABLE ──────────────────────────────────────────────────────────

    private Statement.CreateTable parseCreate() {
        expect("CREATE");
        expect("TABLE");
        boolean ifNotExists = false;
        if (eat("IF")) { expect("NOT"); expect("EXISTS"); ifNotExists = true; }
        String table = readIdent();
        expect("(");
        List<ColumnDef> cols = new ArrayList<>();
        cols.add(parseColumnDef());
        while (eat(",")) cols.add(parseColumnDef());
        expect(")");
        return new Statement.CreateTable(table, ifNotExists, cols);
    }

    private ColumnDef parseColumnDef() {
        String name = readIdent();
        // Optional type name (may be absent, or multi-word like VARYING CHARACTER)
        String typeName = "";
        if (kind == TokKind.IDENT || kind == TokKind.KEYWORD) {
            // Consume type words until we hit a constraint keyword or delimiter
            StringBuilder tb = new StringBuilder();
            while (!atAny("NOT", "PRIMARY", "UNIQUE", "DEFAULT", "CHECK", "REFERENCES", "COLLATE", ",", ")") && kind != TokKind.EOF) {
                if (tb.length() > 0) tb.append(' ');
                tb.append(tokRaw);
                advance();
            }
            typeName = tb.toString();
        }
        boolean notNull = false;
        boolean primaryKey = false;
        boolean unique = false;
        SqlExpr defaultValue = null;
        // Parse column constraints (simplified)
        boolean moreConstraints = true;
        while (moreConstraints) {
            if (eat("NOT")) { expect("NULL"); notNull = true; }
            else if (eat("PRIMARY")) { expect("KEY"); primaryKey = true; notNull = true; }
            else if (eat("UNIQUE")) { unique = true; }
            else if (eat("DEFAULT")) { defaultValue = parsePrimary(); }
            else { moreConstraints = false; }
        }
        return new ColumnDef(name, typeName, notNull, primaryKey, unique, defaultValue);
    }

    // ── DROP TABLE ────────────────────────────────────────────────────────────

    private Statement.DropTable parseDrop() {
        expect("DROP");
        expect("TABLE");
        boolean ifExists = false;
        if (eat("IF")) { expect("EXISTS"); ifExists = true; }
        String table = readIdent();
        return new Statement.DropTable(table, ifExists);
    }

    // ── Expression parser ─────────────────────────────────────────────────────
    //
    // Standard recursive-descent with precedence climbing:
    //
    //   parseExpr      → OR expressions
    //   parseCmpExpr   → AND expressions
    //   parseAndExpr   → NOT, IS NULL, BETWEEN, IN, LIKE, comparisons
    //   parseAddExpr   → + -  (additive)
    //   parseMulExpr   → * / % (multiplicative)
    //   parseUnary     → unary -
    //   parsePrimary   → literal | column | function | (expr)

    private SqlExpr parseExpr() {
        return parseOrExpr();
    }

    private SqlExpr parseOrExpr() {
        SqlExpr left = parseAndExpr();
        while (eat("OR")) {
            left = new SqlExpr.BinaryOp(BinaryOperator.OR, left, parseAndExpr());
        }
        return left;
    }

    private SqlExpr parseAndExpr() {
        SqlExpr left = parseNotExpr();
        while (eat("AND")) {
            left = new SqlExpr.BinaryOp(BinaryOperator.AND, left, parseNotExpr());
        }
        return left;
    }

    private SqlExpr parseNotExpr() {
        if (eat("NOT")) {
            return new SqlExpr.UnaryOp(UnaryOperator.NOT, parseNotExpr());
        }
        return parseCmpExpr();
    }

    private SqlExpr parseCmpExpr() {
        SqlExpr left = parseAddExpr();

        // IS [NOT] NULL
        if (eat("IS")) {
            if (eat("NOT")) {
                expect("NULL");
                return new SqlExpr.IsNotNull(left);
            }
            expect("NULL");
            return new SqlExpr.IsNull(left);
        }

        // [NOT] BETWEEN
        if (eat("BETWEEN")) {
            SqlExpr lo = parseAddExpr();
            expect("AND");
            SqlExpr hi = parseAddExpr();
            return new SqlExpr.Between(left, lo, hi);
        }
        if (at("NOT") && peekAfterNext("BETWEEN")) {
            advance(); // consume NOT
            expect("BETWEEN");
            SqlExpr lo = parseAddExpr();
            expect("AND");
            SqlExpr hi = parseAddExpr();
            // NOT BETWEEN = NOT(BETWEEN)
            return new SqlExpr.UnaryOp(UnaryOperator.NOT, new SqlExpr.Between(left, lo, hi));
        }

        // [NOT] LIKE
        if (eat("LIKE")) {
            String pat = readStringLiteral("LIKE pattern");
            return new SqlExpr.Like(left, pat);
        }
        if (at("NOT") && peekAfterNext("LIKE")) {
            advance();
            expect("LIKE");
            String pat = readStringLiteral("NOT LIKE pattern");
            return new SqlExpr.NotLike(left, pat);
        }

        // [NOT] IN (list)
        if (eat("IN")) {
            List<SqlExpr> items = parseInList();
            return new SqlExpr.In(left, items);
        }
        if (at("NOT") && peekAfterNext("IN")) {
            advance();
            expect("IN");
            List<SqlExpr> items = parseInList();
            return new SqlExpr.NotIn(left, items);
        }

        // Comparison operators
        String op = eatAny("=", "<>", "!=", "<", "<=", ">", ">=");
        if (op != null) {
            SqlExpr right = parseAddExpr();
            BinaryOperator binOp = switch (op) {
                case "="         -> BinaryOperator.EQ;
                case "<>", "!="  -> BinaryOperator.NOT_EQ;
                case "<"         -> BinaryOperator.LT;
                case "<="        -> BinaryOperator.LTE;
                case ">"         -> BinaryOperator.GT;
                case ">="        -> BinaryOperator.GTE;
                default          -> throw parseError("unknown comparison operator: " + op);
            };
            return new SqlExpr.BinaryOp(binOp, left, right);
        }

        return left;
    }

    /** Check whether the token AFTER the next one equals {@code expected}. */
    private boolean peekAfterNext(String expected) {
        // Save state.
        int savedPos = pos;
        String savedTok = tok;
        String savedRaw = tokRaw;
        TokKind savedKind = kind;
        advance();
        boolean result = at(expected);
        // Restore.
        pos  = savedPos;
        tok  = savedTok;
        tokRaw = savedRaw;
        kind = savedKind;
        return result;
    }

    private List<SqlExpr> parseInList() {
        expect("(");
        List<SqlExpr> items = new ArrayList<>();
        if (!at(")")) {
            items.add(parseExpr());
            while (eat(",")) items.add(parseExpr());
        }
        expect(")");
        return items;
    }

    private String readStringLiteral(String context) {
        if (kind != TokKind.STRING) {
            throw parseError(context + " must be a string literal, got '" + tok + "'");
        }
        String val = tokRaw;
        advance();
        return val;
    }

    private SqlExpr parseAddExpr() {
        SqlExpr left = parseMulExpr();
        while (true) {
            if (eat("+")) {
                left = new SqlExpr.BinaryOp(BinaryOperator.ADD, left, parseMulExpr());
            } else if (eat("-")) {
                left = new SqlExpr.BinaryOp(BinaryOperator.SUB, left, parseMulExpr());
            } else if (eat("||")) {
                // String concatenation — map to a FuncCall for now; the VM handles CONCAT
                SqlExpr right = parseMulExpr();
                left = new SqlExpr.FuncCall("CONCAT", List.of(left, right));
            } else {
                break;
            }
        }
        return left;
    }

    private SqlExpr parseMulExpr() {
        SqlExpr left = parseUnary();
        while (true) {
            if (eat("*")) {
                left = new SqlExpr.BinaryOp(BinaryOperator.MUL, left, parseUnary());
            } else if (eat("/")) {
                left = new SqlExpr.BinaryOp(BinaryOperator.DIV, left, parseUnary());
            } else if (eat("%")) {
                left = new SqlExpr.BinaryOp(BinaryOperator.MOD, left, parseUnary());
            } else {
                break;
            }
        }
        return left;
    }

    private SqlExpr parseUnary() {
        if (eat("-")) return new SqlExpr.UnaryOp(UnaryOperator.NEG, parsePrimary());
        if (eat("+")) return parsePrimary();
        return parsePrimary();
    }

    // ── Aggregate / scalar function helpers ───────────────────────────────────

    /** True if the current token is an aggregate function name. */
    private boolean isAggFunc() {
        return atAny("COUNT", "SUM", "AVG", "MIN", "MAX");
    }

    private SqlExpr parseAggOrFunc() {
        String fname = tok.toUpperCase(Locale.ROOT);
        advance(); // consume function name
        expect("(");

        if (fname.equals("COUNT") && eat("*")) {
            expect(")");
            return new SqlExpr.AggExpr(AggFunction.COUNT, new AggArg.Star(), false);
        }

        boolean distinct = eat("DISTINCT");
        SqlExpr arg = parseExpr();
        expect(")");

        AggFunction aggFunc = switch (fname) {
            case "COUNT" -> AggFunction.COUNT;
            case "SUM"   -> AggFunction.SUM;
            case "AVG"   -> AggFunction.AVG;
            case "MIN"   -> AggFunction.MIN;
            case "MAX"   -> AggFunction.MAX;
            default      -> null;
        };
        if (aggFunc != null) {
            return new SqlExpr.AggExpr(aggFunc, new AggArg.Expr(arg), distinct);
        }

        // Scalar function
        return new SqlExpr.FuncCall(fname, List.of(arg));
    }

    private SqlExpr parsePrimary() {
        // ── NULL / TRUE / FALSE ───────────────────────────────────────────
        if (eat("NULL"))  return new SqlExpr.Literal(null);
        if (eat("TRUE"))  return new SqlExpr.Literal(Boolean.TRUE);
        if (eat("FALSE")) return new SqlExpr.Literal(Boolean.FALSE);

        // ── String literal ────────────────────────────────────────────────
        if (kind == TokKind.STRING) {
            String val = tokRaw;
            advance();
            return new SqlExpr.Literal(val);
        }

        // ── Integer literal ───────────────────────────────────────────────
        if (kind == TokKind.INTEGER) {
            long val = Long.parseLong(tok);
            advance();
            return new SqlExpr.Literal(val);
        }

        // ── Real literal ──────────────────────────────────────────────────
        if (kind == TokKind.REAL) {
            double val = Double.parseDouble(tok);
            advance();
            return new SqlExpr.Literal(val);
        }

        // ── Aggregate or scalar function ──────────────────────────────────
        if (isAggFunc() && peekIsLParen()) {
            return parseAggOrFunc();
        }

        // ── Parenthesised expression ──────────────────────────────────────
        if (eat("(")) {
            SqlExpr inner = parseExpr();
            expect(")");
            return inner;
        }

        // ── Scalar function call or column reference ───────────────────────
        if (kind == TokKind.IDENT || kind == TokKind.KEYWORD) {
            String name = tokRaw;
            advance();

            // Function call: name(
            if (eat("(")) {
                List<SqlExpr> args = new ArrayList<>();
                if (!at(")")) {
                    // DISTINCT modifier for functions like COUNT(DISTINCT x)
                    eat("DISTINCT");
                    args.add(parseExpr());
                    while (eat(",")) args.add(parseExpr());
                }
                expect(")");
                return new SqlExpr.FuncCall(name.toUpperCase(Locale.ROOT), args);
            }

            // Table-qualified column: table.col
            if (eat(".")) {
                String col = readIdent();
                return new SqlExpr.Column(name, col);
            }

            // Plain column reference
            return new SqlExpr.Column(null, name);
        }

        throw parseError("unexpected token '" + tok + "' in expression");
    }
}
