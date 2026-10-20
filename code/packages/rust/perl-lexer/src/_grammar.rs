// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: perl (LANG84 release family; release list from code/grammars/perl/releases.csv)
// Regenerate with: grammar-tools generate-rust-compiled-grammars perl
//
// 117 release(s) have a pair; they compile to 5 distinct TokenGrammar value(s).
// Each distinct grammar is one module named g_<first 12 hex digits of the
// SHA-256 of its canonical form (comments and line numbers dropped).

use grammar_tools::token_grammar::TokenGrammar;

/// Releases that have a pair, in inventory (date) order.
pub const SUPPORTED_RELEASES: &[&str] = &[
    "1.000",
    "1.0.15",
    "1.0_16",
    "2.000",
    "2.001",
    "3.000",
    "3.044",
    "4.000",
    "4.036",
    "5.000",
    "5.001",
    "5.001n",
    "5.002",
    "5.002_01",
    "5.003",
    "5.003_01",
    "5.003_02",
    "5.003_03",
    "5.003_04",
    "5.003_05",
    "5.003_06",
    "5.003_07",
    "5.003_08",
    "5.003_09",
    "5.003_10",
    "5.003_11",
    "5.003_12",
    "5.003_13",
    "5.003_14",
    "5.003_15",
    "5.003_16",
    "5.003_17",
    "5.003_18",
    "5.003_19",
    "5.003_20",
    "5.003_21",
    "5.003_22",
    "5.003_23",
    "5.003_24",
    "5.003_25",
    "5.003_26",
    "5.003_27",
    "5.003_28",
    "5.003_90",
    "5.003_91",
    "5.003_92",
    "5.003_93",
    "5.003_94",
    "5.003_95",
    "5.003_96",
    "5.003_97",
    "5.003_97a",
    "5.003_97b",
    "5.003_97c",
    "5.003_97d",
    "5.003_97e",
    "5.003_97f",
    "5.003_97g",
    "5.003_97h",
    "5.003_97i",
    "5.003_97j",
    "5.003_98",
    "5.003_99",
    "5.003_99a",
    "p54rc1",
    "p54rc2",
    "5.004",
    "5.004_01-t2",
    "5.004_01",
    "5.004_01_01",
    "5.004_01_02",
    "5.004_01_03",
    "5.004_02",
    "5.004_02_01",
    "5.004_03-t2",
    "5.004_03",
    "5.004_04-t1",
    "5.004_04-t2",
    "5.004_04-t3",
    "5.004_04-t4",
    "5.004_04",
    "5.004_04-m1",
    "5.004_04-m2",
    "5.004_04-m3",
    "5.004_04-m4",
    "5.004_05-MT5",
    "5.004_05-MT6",
    "5.004_05-MT7",
    "5.004_05-MT8",
    "5.004_05-MT9",
    "5.004_05",
    "5.004_50",
    "5.004_51",
    "5.004_52",
    "5.004_53",
    "5.004_54",
    "5.004_55",
    "5.004_56",
    "5.004_57",
    "5.004_58",
    "5.004_59",
    "5.004_60",
    "5.004_61",
    "5.004_62",
    "5.004_63",
    "5.004_64",
    "5.004_65",
    "5.004_66",
    "5.004_67",
    "5.004_68",
    "5.004_69",
    "5.004_70",
    "5.004_71",
    "5.004_72",
    "5.38.2",
    "5.44.0",
    "5.45.3",
];

/// The TokenGrammar of exactly `release`, or `None`. There is no default and no
/// nearest-release fallback (LANG84 C4).
pub fn token_grammar(release: &str) -> Option<TokenGrammar> {
    match release {
        "1.000" | "1.0.15" | "1.0_16" | "2.000" | "2.001" | "3.000" | "3.044" | "4.000" | "4.036" | "5.000" | "5.001" | "5.001n" | "5.002" | "5.002_01" | "5.003" | "5.003_01" | "5.003_02" | "5.003_03" | "5.003_04" | "5.003_05" | "5.003_06" | "5.003_07" | "5.003_08" | "5.003_09" | "5.003_10" | "5.003_11" | "5.003_12" | "5.003_13" | "5.003_14" | "5.003_15" | "5.003_16" | "5.003_17" | "5.003_18" | "5.003_19" | "5.003_20" | "5.003_21" | "5.003_22" | "5.003_23" | "5.003_24" | "5.003_25" => Some(g_5a960085add9::token_grammar()),
        "5.003_26" | "5.003_27" | "5.003_28" | "5.003_90" | "5.003_91" | "5.003_92" | "5.003_93" | "5.003_94" | "5.003_95" | "5.003_96" | "5.003_97" | "5.003_97a" | "5.003_97b" | "5.003_97c" | "5.003_97d" | "5.003_97e" | "5.003_97f" | "5.003_97g" | "5.003_97h" => Some(g_c4f61ac43123::token_grammar()),
        "5.003_97i" | "5.003_97j" | "5.003_98" | "5.003_99" | "5.003_99a" | "p54rc1" | "p54rc2" | "5.004" | "5.004_01-t2" | "5.004_01" | "5.004_01_01" | "5.004_01_02" | "5.004_01_03" | "5.004_02" | "5.004_02_01" | "5.004_03-t2" | "5.004_03" | "5.004_04-t1" | "5.004_04-t2" | "5.004_04-t3" | "5.004_04-t4" | "5.004_04" | "5.004_04-m1" | "5.004_04-m2" | "5.004_04-m3" | "5.004_04-m4" | "5.004_05-MT5" | "5.004_05-MT6" | "5.004_05-MT7" | "5.004_05-MT8" | "5.004_05-MT9" | "5.004_05" | "5.004_50" | "5.004_51" | "5.004_52" | "5.004_53" | "5.004_54" | "5.004_55" | "5.004_56" | "5.004_57" | "5.004_58" | "5.004_59" | "5.004_60" | "5.004_61" | "5.004_62" | "5.004_63" | "5.004_64" | "5.004_65" | "5.004_66" | "5.004_67" | "5.004_68" | "5.004_69" | "5.004_70" | "5.004_71" => Some(g_e27cead68fd9::token_grammar()),
        "5.004_72" => Some(g_c8dae440f6c9::token_grammar()),
        "5.38.2" | "5.44.0" | "5.45.3" => Some(g_40885e9aaa81::token_grammar()),
        _ => None,
    }
}

/// The class (module name) of each release, for tests and tooling.
pub fn grammar_class(release: &str) -> Option<&'static str> {
    match release {
        "1.000" | "1.0.15" | "1.0_16" | "2.000" | "2.001" | "3.000" | "3.044" | "4.000" | "4.036" | "5.000" | "5.001" | "5.001n" | "5.002" | "5.002_01" | "5.003" | "5.003_01" | "5.003_02" | "5.003_03" | "5.003_04" | "5.003_05" | "5.003_06" | "5.003_07" | "5.003_08" | "5.003_09" | "5.003_10" | "5.003_11" | "5.003_12" | "5.003_13" | "5.003_14" | "5.003_15" | "5.003_16" | "5.003_17" | "5.003_18" | "5.003_19" | "5.003_20" | "5.003_21" | "5.003_22" | "5.003_23" | "5.003_24" | "5.003_25" => Some("g_5a960085add9"),
        "5.003_26" | "5.003_27" | "5.003_28" | "5.003_90" | "5.003_91" | "5.003_92" | "5.003_93" | "5.003_94" | "5.003_95" | "5.003_96" | "5.003_97" | "5.003_97a" | "5.003_97b" | "5.003_97c" | "5.003_97d" | "5.003_97e" | "5.003_97f" | "5.003_97g" | "5.003_97h" => Some("g_c4f61ac43123"),
        "5.003_97i" | "5.003_97j" | "5.003_98" | "5.003_99" | "5.003_99a" | "p54rc1" | "p54rc2" | "5.004" | "5.004_01-t2" | "5.004_01" | "5.004_01_01" | "5.004_01_02" | "5.004_01_03" | "5.004_02" | "5.004_02_01" | "5.004_03-t2" | "5.004_03" | "5.004_04-t1" | "5.004_04-t2" | "5.004_04-t3" | "5.004_04-t4" | "5.004_04" | "5.004_04-m1" | "5.004_04-m2" | "5.004_04-m3" | "5.004_04-m4" | "5.004_05-MT5" | "5.004_05-MT6" | "5.004_05-MT7" | "5.004_05-MT8" | "5.004_05-MT9" | "5.004_05" | "5.004_50" | "5.004_51" | "5.004_52" | "5.004_53" | "5.004_54" | "5.004_55" | "5.004_56" | "5.004_57" | "5.004_58" | "5.004_59" | "5.004_60" | "5.004_61" | "5.004_62" | "5.004_63" | "5.004_64" | "5.004_65" | "5.004_66" | "5.004_67" | "5.004_68" | "5.004_69" | "5.004_70" | "5.004_71" => Some("g_e27cead68fd9"),
        "5.004_72" => Some("g_c8dae440f6c9"),
        "5.38.2" | "5.44.0" | "5.45.3" => Some("g_40885e9aaa81"),
        _ => None,
    }
}

/// The unversioned pair, kept for consumers that have not moved to a release.
/// It is not reachable through the release selector above.
pub fn unversioned_token_grammar() -> TokenGrammar {
    legacy::token_grammar()
}

mod legacy {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl.tokens
    // Regenerate with: grammar-tools compile-tokens perl.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 5,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 6,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 7,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 16,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

mod g_5a960085add9 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl1.000.tokens
    // Regenerate with: grammar-tools compile-tokens perl1.000.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 18,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 21,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

mod g_c4f61ac43123 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.003_26.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.003_26.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 19,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

mod g_e27cead68fd9 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.003_97i.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.003_97i.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]{0,249}"#.to_string(),
                    is_regex: true,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 20,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

mod g_c8dae440f6c9 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.004_72.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.004_72.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]{0,249}"#.to_string(),
                    is_regex: true,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 18,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 19,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 20,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 21,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

mod g_40885e9aaa81 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.38.2.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.38.2.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 7,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 18,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}
