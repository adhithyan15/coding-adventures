// AUTO-GENERATED FILE — DO NOT EDIT
// Source: sql.grammar
// Regenerate with: grammar-tools compile-grammar sql.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#";"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#";"#.to_string() }) },
            ] },
            line_number: 10,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"select_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"insert_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"update_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"delete_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"create_table_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"drop_table_stmt"#.to_string() },
            ] },
            line_number: 12,
        },
        GrammarRule {
            name: r#"select_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"SELECT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"DISTINCT"#.to_string() },
                        GrammarElement::Literal { value: r#"ALL"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"select_list"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"FROM"#.to_string() },
                    GrammarElement::RuleReference { name: r#"table_ref"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"join_clause"#.to_string() }) },
                ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"where_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"group_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"having_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"order_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"limit_clause"#.to_string() }) },
            ] },
            line_number: 17,
        },
        GrammarRule {
            name: r#"select_list"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"select_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#","#.to_string() },
                            GrammarElement::RuleReference { name: r#"select_item"#.to_string() },
                        ] }) },
                ] },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"select_item"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                // `select_item = expr [ [ "AS" ] NAME ]` — the AS keyword is
                // OPTIONAL, so `SELECT a col1` (bare alias) parses the same as
                // `SELECT a AS col1`. The alias NAME can never eat a following
                // keyword (FROM/WHERE/…) because NAME only matches Name-type
                // tokens, and it can't eat a comma, so `SELECT a, b` is safe.
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"AS"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 23,
        },
        GrammarRule {
            name: r#"table_ref"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"table_name"#.to_string() },
                // `table_ref = table_name [ [ "AS" ] NAME ]` — the AS keyword is
                // OPTIONAL, so `FROM users u` aliases the table the same as
                // `FROM users AS u` (SQLite accepts both). The alias NAME cannot
                // eat a following keyword (JOIN/WHERE/ON/…) because NAME only
                // matches Name-type tokens, so `FROM a JOIN b` is unaffected.
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"AS"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 25,
        },
        GrammarRule {
            name: r#"table_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"."#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 26,
        },
        GrammarRule {
            name: r#"join_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                // `join_type` is OPTIONAL: a bare `JOIN` (no INNER/LEFT/… prefix)
                // is an INNER join, which the planner already defaults to when the
                // `join_type` node is absent. Matches `sql.grammar` (`[ join_type ]`).
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"join_type"#.to_string() }) },
                GrammarElement::Literal { value: r#"JOIN"#.to_string() },
                GrammarElement::RuleReference { name: r#"table_ref"#.to_string() },
                // `ON expr` is OPTIONAL: a join with no condition is a Cartesian
                // (cross) product — `FROM a JOIN b` and `FROM a CROSS JOIN b`. The
                // planner returns `None` for a missing ON, and codegen emits every
                // pair (no condition check) for an INNER join with no condition.
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"ON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                ] }) },
            ] },
            line_number: 28,
        },
        GrammarRule {
            name: r#"join_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"CROSS"#.to_string() },
                GrammarElement::Literal { value: r#"INNER"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"LEFT"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"OUTER"#.to_string() }) },
                    ] }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"RIGHT"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"OUTER"#.to_string() }) },
                    ] }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"FULL"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"OUTER"#.to_string() }) },
                    ] }) },
            ] },
            line_number: 29,
        },
        GrammarRule {
            name: r#"where_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"WHERE"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 32,
        },
        GrammarRule {
            name: r#"group_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"GROUP"#.to_string() },
                GrammarElement::Literal { value: r#"BY"#.to_string() },
                GrammarElement::RuleReference { name: r#"column_ref"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"column_ref"#.to_string() },
                    ] }) },
            ] },
            line_number: 33,
        },
        GrammarRule {
            name: r#"having_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"HAVING"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 34,
        },
        GrammarRule {
            name: r#"order_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ORDER"#.to_string() },
                GrammarElement::Literal { value: r#"BY"#.to_string() },
                GrammarElement::RuleReference { name: r#"order_item"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"order_item"#.to_string() },
                    ] }) },
            ] },
            line_number: 35,
        },
        GrammarRule {
            name: r#"order_item"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                // Optional `COLLATE name` clause, BEFORE the ASC/DESC direction
                // (per SQLite grammar: `expr COLLATE name ASC`). `COLLATE` is
                // matched by literal text (it is not in the lexer keyword list,
                // so it arrives as a NAME token that the literal matcher accepts
                // case-insensitively); the name that follows (BINARY / NOCASE /
                // RTRIM, or a user collation) is accepted as a generic NAME and
                // validated in the planner. Absent → BINARY (byte order).
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"COLLATE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"ASC"#.to_string() },
                        GrammarElement::Literal { value: r#"DESC"#.to_string() },
                    ] }) },
                // Optional `NULLS FIRST` / `NULLS LAST`. FIRST/LAST are NOT
                // reserved keywords (they are extremely common column names), so
                // we accept a generic NAME here; the planner validates it is
                // literally FIRST or LAST. Omitting the clause falls back to
                // SQLite's defaults (NULLs first for ASC, last for DESC).
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"NULLS"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 36,
        },
        GrammarRule {
            name: r#"limit_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"LIMIT"#.to_string() },
                // Allow optional "-" before NUMBER to support LIMIT -1 (all rows).
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"-"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                // The tail is either the standard `OFFSET n` or MySQL's
                // `, n` shorthand. NOTE the argument order flips between them:
                // `LIMIT count OFFSET off` vs `LIMIT off , count` — in the comma
                // form the FIRST number is the OFFSET and the SECOND is the
                // count (plan_limit does the swap). This matches SQLite, which
                // accepts the comma form purely for MySQL compatibility.
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"OFFSET"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#","#.to_string() },
                            GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 37,
        },
        GrammarRule {
            name: r#"insert_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"INSERT"#.to_string() },
                GrammarElement::Literal { value: r#"INTO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"("#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#","#.to_string() },
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            ] }) },
                        GrammarElement::Literal { value: r#")"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"VALUES"#.to_string() },
                GrammarElement::RuleReference { name: r#"row_value"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"row_value"#.to_string() },
                    ] }) },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"row_value"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"("#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#")"#.to_string() },
            ] },
            line_number: 44,
        },
        GrammarRule {
            name: r#"update_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"UPDATE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"SET"#.to_string() },
                GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"where_clause"#.to_string() }) },
            ] },
            line_number: 46,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"="#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 48,
        },
        GrammarRule {
            name: r#"delete_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DELETE"#.to_string() },
                GrammarElement::Literal { value: r#"FROM"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"where_clause"#.to_string() }) },
            ] },
            line_number: 50,
        },
        GrammarRule {
            name: r#"create_table_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"CREATE"#.to_string() },
                GrammarElement::Literal { value: r#"TABLE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"IF"#.to_string() },
                        GrammarElement::Literal { value: r#"NOT"#.to_string() },
                        GrammarElement::Literal { value: r#"EXISTS"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"("#.to_string() },
                GrammarElement::RuleReference { name: r#"col_def"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"col_def"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#")"#.to_string() },
            ] },
            line_number: 54,
        },
        GrammarRule {
            // col_def = NAME [ col_type ] col_constraint*
            // The type name is OPTIONAL — SQLite allows typeless columns such as
            //   CREATE TABLE t (id, name, age)
            // When omitted the engine applies TEXT affinity (the SQLite default).
            name: r#"col_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"col_constraint"#.to_string() }) },
            ] },
            line_number: 56,
        },
        GrammarRule {
            name: r#"col_constraint"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"NOT"#.to_string() },
                        GrammarElement::Literal { value: r#"NULL"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"NULL"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"PRIMARY"#.to_string() },
                        GrammarElement::Literal { value: r#"KEY"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"UNIQUE"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"DEFAULT"#.to_string() },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"drop_table_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DROP"#.to_string() },
                GrammarElement::Literal { value: r#"TABLE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"IF"#.to_string() },
                        GrammarElement::Literal { value: r#"EXISTS"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 60,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            line_number: 64,
        },
        GrammarRule {
            name: r#"or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 65,
        },
        GrammarRule {
            name: r#"and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 66,
        },
        GrammarRule {
            name: r#"not_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"NOT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
            ] },
            line_number: 67,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            // Optional `COLLATE name` on the LEFT operand
                            // (`col COLLATE NOCASE = 'x'`). It lives at the START
                            // of THIS cmp_op alternative — not before the whole
                            // alternation — so that a trailing `COLLATE` with no
                            // following `cmp_op` (e.g. `ORDER BY name COLLATE
                            // NOCASE`, where the order_item owns the COLLATE)
                            // fails this alternative and backtracks, leaving the
                            // COLLATE for the caller. The planner takes the FIRST
                            // COLLATE token, so a left collation wins over a right
                            // one — matching SQLite — and applies it to both sides.
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"COLLATE"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"cmp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                            // Optional `COLLATE name` on the right operand of a
                            // comparison (`col = 'x' COLLATE NOCASE`). The planner
                            // applies the collation to BOTH sides of the compare.
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"COLLATE"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"BETWEEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                            GrammarElement::Literal { value: r#"AND"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"BETWEEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                            GrammarElement::Literal { value: r#"AND"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IN"#.to_string() },
                            GrammarElement::Literal { value: r#"("#.to_string() },
                            GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                            GrammarElement::Literal { value: r#")"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"IN"#.to_string() },
                            GrammarElement::Literal { value: r#"("#.to_string() },
                            GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                            GrammarElement::Literal { value: r#")"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"LIKE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"LIKE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        // `x GLOB pattern` — case-sensitive Unix-glob matching.
                        // SQLite defines `X GLOB Y` as `glob(Y, X)`, so the
                        // planner lowers this to the existing `glob` builtin
                        // (args swapped); `NOT GLOB` wraps it in a logical NOT.
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"GLOB"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"GLOB"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::Literal { value: r#"NULL"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"NULL"#.to_string() },
                        ] },
                        // `x IS [NOT] DISTINCT FROM <expr>` — the standard-SQL
                        // spelling of the null-safe compare. `IS NOT DISTINCT
                        // FROM` is the null-safe *equality* (`x IS y`) and `IS
                        // DISTINCT FROM` is its negation. Placed BEFORE the plain
                        // `IS [NOT] <expr>` forms so ordered choice matches the
                        // DISTINCT keyword first (the planner inverts the sense).
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"DISTINCT"#.to_string() },
                            GrammarElement::Literal { value: r#"FROM"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::Literal { value: r#"DISTINCT"#.to_string() },
                            GrammarElement::Literal { value: r#"FROM"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        // `x IS NOT <expr>` / `x IS <expr>` — null-safe (in)equality.
                        // These come AFTER the IS NULL / IS NOT NULL sequences so
                        // ordered-choice matches the NULL forms first (NULL is
                        // itself a valid `additive`); and `IS NOT <expr>` before
                        // `IS <expr>` so the NOT form is tried first.
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"IS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 68,
        },
        GrammarRule {
            name: r#"cmp_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"="#.to_string() },
                GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                GrammarElement::Literal { value: r#"<"#.to_string() },
                GrammarElement::Literal { value: r#">"#.to_string() },
                GrammarElement::Literal { value: r#"<="#.to_string() },
                GrammarElement::Literal { value: r#">="#.to_string() },
            ] },
            line_number: 78,
        },
        GrammarRule {
            // Bitwise operators `& | << >>` — one precedence level, left-
            // associative, sitting BETWEEN additive and comparison (SQLite
            // groups all four here). `5 | 3 & 2` = `(5|3)&2` = 2; `3+1<<2` =
            // `(3+1)<<2` = 16. The generated rule mirrors the grammar source
            // `bitwise = additive { ("&"|"|"|"<<"|">>") additive }`.
            name: r#"bitwise"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"&"#.to_string() },
                                GrammarElement::Literal { value: r#"|"#.to_string() },
                                GrammarElement::Literal { value: r#"<<"#.to_string() },
                                GrammarElement::Literal { value: r#">>"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                    ] }) },
            ] },
            line_number: 79,
        },
        GrammarRule {
            name: r#"additive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"+"#.to_string() },
                                GrammarElement::Literal { value: r#"-"#.to_string() },
                                GrammarElement::Literal { value: r#"||"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                    ] }) },
            ] },
            line_number: 79,
        },
        GrammarRule {
            name: r#"multiplicative"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::Literal { value: r#"/"#.to_string() },
                                GrammarElement::Literal { value: r#"%"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 80,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            // Prefix operators, per the grammar source `( "-" | "~" | "+" ) unary`:
            // arithmetic negation `-`, bitwise complement `~`, and the no-op
            // unary plus `+`. The planner maps `-`→Neg, `~`→BitNot, and treats
            // `+x` as `x`.
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"-"#.to_string() },
                            GrammarElement::Literal { value: r#"~"#.to_string() },
                            GrammarElement::Literal { value: r#"+"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
            ] },
            line_number: 81,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Literal { value: r#"NULL"#.to_string() },
                GrammarElement::Literal { value: r#"TRUE"#.to_string() },
                GrammarElement::Literal { value: r#"FALSE"#.to_string() },
                // `CAST(expr AS type)` — placed before `function_call` so the
                // ordered-choice parser matches the AS-typed form first (a bare
                // `foo(...)` fails the leading `CAST` literal and falls through
                // to function_call). The type is a NAME token (INTEGER/REAL/…).
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"CAST"#.to_string() },
                    GrammarElement::Literal { value: r#"("#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::Literal { value: r#"AS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Literal { value: r#")"#.to_string() },
                ] },
                // `CASE [operand] WHEN cond THEN val … [ELSE val] END`. Two
                // forms share one rule: the *searched* form (`CASE WHEN cond …`)
                // and the *simple* form (`CASE operand WHEN value …`), told apart
                // by the optional `operand` expr between `CASE` and the first
                // `WHEN`. Because `WHEN` is a keyword and cannot start an `expr`,
                // the optional operand never swallows the searched form's `WHEN`.
                // One WHEN/THEN is mandatory; further ones repeat; ELSE is
                // optional. All slots are full `expr`s. The planner desugars the
                // simple form to the searched form (`operand = value`). Placed
                // before function_call so the leading `CASE` literal matches here.
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"CASE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                    GrammarElement::Literal { value: r#"WHEN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::Literal { value: r#"THEN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"WHEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                            GrammarElement::Literal { value: r#"THEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"ELSE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"END"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"column_ref"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"("#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::Literal { value: r#")"#.to_string() },
                ] },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"column_ref"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"."#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 85,
        },
        GrammarRule {
            // function_call = NAME "(" ( STAR | [ DISTINCT ] value_list? ) ")"
            // This supports COUNT(*), COUNT(col), COUNT(DISTINCT col),
            // SUM(expr), and all other aggregate/scalar function calls.
            name: r#"function_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"("#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"DISTINCT"#.to_string() }) },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"value_list"#.to_string() }) },
                        ] },
                    ] }) },
                GrammarElement::Literal { value: r#")"#.to_string() },
            ] },
            line_number: 86,
        },
        GrammarRule {
            name: r#"value_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#","#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 87,
        },
    ],
        version: 1,
    }
}
