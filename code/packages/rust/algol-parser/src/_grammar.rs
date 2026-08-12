// AUTO-GENERATED FILE — DO NOT EDIT
// Source: algol60.grammar
// Regenerate with: grammar-tools compile-grammar algol60.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"block"#.to_string() },
            line_number: 47,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                            ] }) },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 53,
        },
        GrammarRule {
            name: r#"declaration"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"type_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"own_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"own_array_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"switch_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"procedure_decl"#.to_string() },
            ] },
            line_number: 60,
        },
        GrammarRule {
            name: r#"type_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::RuleReference { name: r#"ident_list"#.to_string() },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"own_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"own"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::RuleReference { name: r#"ident_list"#.to_string() },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"own_array_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"own"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type"#.to_string() }) },
                GrammarElement::Literal { value: r#"array"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_segment"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"array_segment"#.to_string() },
                    ] }) },
            ] },
            line_number: 81,
        },
        GrammarRule {
            name: r#"type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"integer"#.to_string() },
                GrammarElement::Literal { value: r#"real"#.to_string() },
                GrammarElement::Literal { value: r#"boolean"#.to_string() },
                GrammarElement::Literal { value: r#"string"#.to_string() },
            ] },
            line_number: 83,
        },
        GrammarRule {
            name: r#"ident_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 85,
        },
        GrammarRule {
            name: r#"array_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type"#.to_string() }) },
                GrammarElement::Literal { value: r#"array"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_segment"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"array_segment"#.to_string() },
                    ] }) },
            ] },
            line_number: 93,
        },
        GrammarRule {
            name: r#"array_segment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"ident_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"bound_pair"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bound_pair"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 95,
        },
        GrammarRule {
            name: r#"bound_pair"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
            ] },
            line_number: 99,
        },
        GrammarRule {
            name: r#"switch_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"switch"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"ASSIGN"#.to_string() },
                GrammarElement::RuleReference { name: r#"switch_list"#.to_string() },
            ] },
            line_number: 104,
        },
        GrammarRule {
            name: r#"switch_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 106,
        },
        GrammarRule {
            name: r#"procedure_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type"#.to_string() }) },
                GrammarElement::Literal { value: r#"procedure"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"formal_params"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"value_part"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"spec_part"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"proc_body"#.to_string() },
            ] },
            line_number: 113,
        },
        GrammarRule {
            name: r#"formal_params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"ident_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 118,
        },
        GrammarRule {
            name: r#"value_part"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"value"#.to_string() },
                GrammarElement::RuleReference { name: r#"ident_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 123,
        },
        GrammarRule {
            name: r#"spec_part"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"specifier"#.to_string() },
                GrammarElement::RuleReference { name: r#"ident_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 130,
        },
        GrammarRule {
            name: r#"specifier"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"array"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"procedure"#.to_string() },
                ] },
                GrammarElement::Literal { value: r#"array"#.to_string() },
                GrammarElement::Literal { value: r#"label"#.to_string() },
                GrammarElement::Literal { value: r#"switch"#.to_string() },
                GrammarElement::Literal { value: r#"procedure"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
            ] },
            line_number: 132,
        },
        GrammarRule {
            name: r#"proc_body"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 140,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"label"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unlabeled_stmt"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"label"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"cond_stmt"#.to_string() },
                ] },
            ] },
            line_number: 152,
        },
        GrammarRule {
            name: r#"label"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"INTEGER_LIT"#.to_string() },
            ] },
            line_number: 155,
        },
        GrammarRule {
            name: r#"unlabeled_stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"dummy_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"goto_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"proc_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
            ] },
            line_number: 165,
        },
        GrammarRule {
            name: r#"dummy_stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::PositiveLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() }) },
                GrammarElement::PositiveLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                GrammarElement::PositiveLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
            ] },
            line_number: 175,
        },
        GrammarRule {
            name: r#"cond_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                GrammarElement::Literal { value: r#"then"#.to_string() },
                GrammarElement::RuleReference { name: r#"unlabeled_stmt"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 181,
        },
        GrammarRule {
            name: r#"compound_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                            ] }) },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 185,
        },
        GrammarRule {
            name: r#"assign_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"left_part"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"left_part"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 191,
        },
        GrammarRule {
            name: r#"left_part"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::TokenReference { name: r#"ASSIGN"#.to_string() },
            ] },
            line_number: 193,
        },
        GrammarRule {
            name: r#"goto_stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"goto"#.to_string() },
                    GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"go"#.to_string() },
                    GrammarElement::Literal { value: r#"to"#.to_string() },
                    GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                ] },
            ] },
            line_number: 197,
        },
        GrammarRule {
            name: r#"proc_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"actual_params"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 202,
        },
        GrammarRule {
            name: r#"actual_params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 204,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::TokenReference { name: r#"ASSIGN"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_list"#.to_string() },
                GrammarElement::Literal { value: r#"do"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 212,
        },
        GrammarRule {
            name: r#"for_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"for_elem"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"for_elem"#.to_string() },
                    ] }) },
            ] },
            line_number: 214,
        },
        GrammarRule {
            name: r#"for_elem"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"step"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"until"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
            ] },
            line_number: 218,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"else"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"expr_eqv"#.to_string() },
            ] },
            line_number: 250,
        },
        GrammarRule {
            name: r#"expr_eqv"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_impl"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"eqv"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_impl"#.to_string() },
                    ] }) },
            ] },
            line_number: 253,
        },
        GrammarRule {
            name: r#"expr_impl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_or"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"impl"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_or"#.to_string() },
                    ] }) },
            ] },
            line_number: 254,
        },
        GrammarRule {
            name: r#"expr_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"or"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 255,
        },
        GrammarRule {
            name: r#"expr_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_not"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_not"#.to_string() },
                    ] }) },
            ] },
            line_number: 256,
        },
        GrammarRule {
            name: r#"expr_not"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"not"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_not"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"expr_cmp"#.to_string() },
            ] },
            line_number: 257,
        },
        GrammarRule {
            name: r#"expr_cmp"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_add"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NEQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LEQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr_add"#.to_string() },
                    ] }) },
            ] },
            line_number: 258,
        },
        GrammarRule {
            name: r#"expr_add"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                        GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expr_mul"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr_mul"#.to_string() },
                    ] }) },
            ] },
            line_number: 259,
        },
        GrammarRule {
            name: r#"expr_mul"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_pow"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                GrammarElement::Literal { value: r#"div"#.to_string() },
                                GrammarElement::Literal { value: r#"mod"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr_pow"#.to_string() },
                    ] }) },
            ] },
            line_number: 260,
        },
        GrammarRule {
            name: r#"expr_pow"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr_atom"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                                GrammarElement::TokenReference { name: r#"POWER"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr_atom"#.to_string() },
                    ] }) },
            ] },
            line_number: 261,
        },
        GrammarRule {
            name: r#"expr_atom"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"INTEGER_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"REAL_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING_LIT"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::RuleReference { name: r#"proc_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 262,
        },
        GrammarRule {
            name: r#"arith_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"else"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"simple_arith"#.to_string() },
            ] },
            line_number: 274,
        },
        GrammarRule {
            name: r#"simple_arith"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                        GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
            ] },
            line_number: 278,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                GrammarElement::Literal { value: r#"div"#.to_string() },
                                GrammarElement::Literal { value: r#"mod"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] }) },
            ] },
            line_number: 283,
        },
        GrammarRule {
            name: r#"factor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                                GrammarElement::TokenReference { name: r#"POWER"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 289,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"INTEGER_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"REAL_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING_LIT"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::RuleReference { name: r#"proc_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 291,
        },
        GrammarRule {
            name: r#"bool_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"else"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"simple_bool"#.to_string() },
            ] },
            line_number: 309,
        },
        GrammarRule {
            name: r#"simple_bool"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"implication"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"eqv"#.to_string() },
                        GrammarElement::RuleReference { name: r#"implication"#.to_string() },
                    ] }) },
            ] },
            line_number: 312,
        },
        GrammarRule {
            name: r#"implication"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bool_term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"impl"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bool_term"#.to_string() },
                    ] }) },
            ] },
            line_number: 314,
        },
        GrammarRule {
            name: r#"bool_term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bool_factor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"or"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bool_factor"#.to_string() },
                    ] }) },
            ] },
            line_number: 316,
        },
        GrammarRule {
            name: r#"bool_factor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bool_secondary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bool_secondary"#.to_string() },
                    ] }) },
            ] },
            line_number: 318,
        },
        GrammarRule {
            name: r#"bool_secondary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"not"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_secondary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"bool_primary"#.to_string() },
            ] },
            line_number: 320,
        },
        GrammarRule {
            name: r#"bool_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"relation"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::RuleReference { name: r#"proc_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 322,
        },
        GrammarRule {
            name: r#"relation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"simple_arith"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NEQ"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LEQ"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GEQ"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"simple_arith"#.to_string() },
            ] },
            line_number: 332,
        },
        GrammarRule {
            name: r#"desig_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"bool_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                    GrammarElement::Literal { value: r#"else"#.to_string() },
                    GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"simple_desig"#.to_string() },
            ] },
            line_number: 337,
        },
        GrammarRule {
            name: r#"simple_desig"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"desig_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"label"#.to_string() },
            ] },
            line_number: 340,
        },
        GrammarRule {
            name: r#"variable"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscripts"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] }) },
            ] },
            line_number: 352,
        },
        GrammarRule {
            name: r#"subscripts"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 354,
        },
        GrammarRule {
            name: r#"proc_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"actual_params"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 359,
        },
    ],
        version: 1,
    }
}
