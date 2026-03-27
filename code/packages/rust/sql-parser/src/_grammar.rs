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
                GrammarElement::Literal { value: r#"FROM"#.to_string() },
                GrammarElement::RuleReference { name: r#"table_ref"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"join_clause"#.to_string() }) },
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
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"AS"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 23,
        },
        GrammarRule {
            name: r#"table_ref"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"table_name"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"AS"#.to_string() },
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
                GrammarElement::RuleReference { name: r#"join_type"#.to_string() },
                GrammarElement::Literal { value: r#"JOIN"#.to_string() },
                GrammarElement::RuleReference { name: r#"table_ref"#.to_string() },
                GrammarElement::Literal { value: r#"ON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
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
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"ASC"#.to_string() },
                        GrammarElement::Literal { value: r#"DESC"#.to_string() },
                    ] }) },
            ] },
            line_number: 36,
        },
        GrammarRule {
            name: r#"limit_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"LIMIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"OFFSET"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
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
            name: r#"col_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
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
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"cmp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"BETWEEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                            GrammarElement::Literal { value: r#"AND"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"BETWEEN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                            GrammarElement::Literal { value: r#"AND"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
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
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"NOT"#.to_string() },
                            GrammarElement::Literal { value: r#"LIKE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"additive"#.to_string() },
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
            name: r#"additive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"+"#.to_string() },
                                GrammarElement::Literal { value: r#"-"#.to_string() },
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
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"-"#.to_string() },
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
            name: r#"function_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"("#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"value_list"#.to_string() }) },
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
