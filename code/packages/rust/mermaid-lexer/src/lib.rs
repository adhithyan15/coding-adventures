//! Grammar-driven lexers for Mermaid diagram families.

pub const VERSION: &str = "0.69.0";

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

const TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/mermaid.tokens");
const PIE_TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/pie.tokens");
const SANKEY_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/sankey.tokens");
const GITGRAPH_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/gitgraph.tokens");
const ER_TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/er.tokens");
const C4_TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/c4.tokens");
const SEQUENCE_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/sequence.tokens");
const STATE_TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/state.tokens");
const QUADRANT_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/quadrant.tokens");
const JOURNEY_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/journey.tokens");
const REQUIREMENT_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/requirement.tokens");
const XYCHART_TOKEN_GRAMMAR_SOURCE: &str =
    include_str!("../../../../grammars/mermaid/xychart.tokens");
const GANTT_TOKEN_GRAMMAR_SOURCE: &str = include_str!("../../../../grammars/mermaid/gantt.tokens");

fn create_lexer<'a>(source: &'a str, grammar_source: &str, grammar_name: &str) -> GrammarLexer<'a> {
    let grammar = parse_token_grammar(grammar_source)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_name}: {e}"));
    GrammarLexer::new(source, &grammar)
}

pub fn create_mermaid_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, TOKEN_GRAMMAR_SOURCE, "mermaid.tokens")
}

pub fn create_mermaid_pie_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, PIE_TOKEN_GRAMMAR_SOURCE, "pie.tokens")
}

pub fn create_mermaid_sankey_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, SANKEY_TOKEN_GRAMMAR_SOURCE, "sankey.tokens")
}

pub fn create_mermaid_gitgraph_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, GITGRAPH_TOKEN_GRAMMAR_SOURCE, "gitgraph.tokens")
}

pub fn create_mermaid_er_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, ER_TOKEN_GRAMMAR_SOURCE, "er.tokens")
}

pub fn create_mermaid_c4_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, C4_TOKEN_GRAMMAR_SOURCE, "c4.tokens")
}

pub fn create_mermaid_sequence_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, SEQUENCE_TOKEN_GRAMMAR_SOURCE, "sequence.tokens")
}

pub fn create_mermaid_state_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, STATE_TOKEN_GRAMMAR_SOURCE, "state.tokens")
}

pub fn create_mermaid_quadrant_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, QUADRANT_TOKEN_GRAMMAR_SOURCE, "quadrant.tokens")
}

pub fn create_mermaid_journey_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, JOURNEY_TOKEN_GRAMMAR_SOURCE, "journey.tokens")
}

pub fn create_mermaid_requirement_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(
        source,
        REQUIREMENT_TOKEN_GRAMMAR_SOURCE,
        "requirement.tokens",
    )
}

pub fn create_mermaid_xychart_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, XYCHART_TOKEN_GRAMMAR_SOURCE, "xychart.tokens")
}

pub fn create_mermaid_gantt_lexer(source: &str) -> GrammarLexer<'_> {
    create_lexer(source, GANTT_TOKEN_GRAMMAR_SOURCE, "gantt.tokens")
}

pub fn tokenize_mermaid(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid tokenization failed: {e}"))
}

pub fn tokenize_mermaid_pie(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_pie_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid pie tokenization failed: {e}"))
}

pub fn tokenize_mermaid_sankey(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_sankey_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid Sankey tokenization failed: {e}"))
}

pub fn tokenize_mermaid_gitgraph(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_gitgraph_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid GitGraph tokenization failed: {e}"))
}

pub fn tokenize_mermaid_er(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_er_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid ER tokenization failed: {e}"))
}

pub fn tokenize_mermaid_c4(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_c4_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid C4 tokenization failed: {e}"))
}

pub fn tokenize_mermaid_sequence(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_sequence_lexer(source);
    let mut tokens = lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid sequence tokenization failed: {e}"));
    let grammar = parse_token_grammar(SEQUENCE_TOKEN_GRAMMAR_SOURCE)
        .expect("sequence.tokens was already validated while creating the lexer");
    let statement_keywords = [
        "participant",
        "actor",
        "create",
        "destroy",
        "link",
        "links",
        "properties",
        "details",
        "accTitle",
        "accDescr",
        "box",
        "activate",
        "deactivate",
        "note",
        "title",
        "autonumber",
        "loop",
        "rect",
        "opt",
        "alt",
        "else",
        "par",
        "par_over",
        "and",
        "critical",
        "option",
        "break",
        "end",
    ];
    let mut line_start = true;
    let mut context: Option<String> = None;
    for token in &mut tokens {
        let token_name = token.type_name.as_deref();
        if matches!(
            token.type_,
            lexer::token::TokenType::Newline | lexer::token::TokenType::Semicolon
        ) {
            line_start = true;
            context = None;
            continue;
        }
        if token_name == Some("HEADER") {
            line_start = false;
            continue;
        }
        let keyword = grammar
            .keywords
            .iter()
            .find(|keyword| keyword.eq_ignore_ascii_case(&token.value))
            .cloned();
        if line_start {
            if let Some(keyword) =
                keyword.filter(|value| statement_keywords.contains(&value.as_str()))
            {
                token.value.clone_from(&keyword);
                context = Some(keyword);
            }
            line_start = false;
        } else if let Some(keyword) = keyword {
            let allowed = match context.as_deref() {
                Some("create") => matches!(keyword.as_str(), "participant" | "actor"),
                Some("participant" | "actor") => keyword == "as",
                Some("note") => matches!(keyword.as_str(), "left" | "right" | "over"),
                Some("note-placement") => keyword == "of",
                Some("autonumber") => keyword == "off",
                _ => false,
            };
            if allowed {
                token.value.clone_from(&keyword);
                context = match (context.as_deref(), keyword.as_str()) {
                    (Some("create"), "participant" | "actor") => Some(keyword),
                    (Some("note"), "left" | "right") => Some("note-placement".to_string()),
                    _ => None,
                };
            }
        }
        if token.type_name.as_deref() == Some("WRAP_DIRECTIVE") {
            token.value.make_ascii_lowercase();
        }
    }
    tokens.retain(|token| token.type_name.as_deref() != Some("HASH_COMMENT"));
    tokens
}

pub fn tokenize_mermaid_state(source: &str) -> Vec<Token> {
    let mut lexer = create_mermaid_state_lexer(source);
    let tokens = lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid state tokenization failed: {e}"));
    let mut filtered = Vec::with_capacity(tokens.len());
    let mut semantic_hash_context = false;
    let mut suppress_line = false;
    for token in tokens {
        if matches!(
            token.type_,
            lexer::token::TokenType::Newline
                | lexer::token::TokenType::Semicolon
                | lexer::token::TokenType::Eof
        ) {
            semantic_hash_context = false;
            suppress_line = false;
            filtered.push(token);
            continue;
        }
        if suppress_line {
            continue;
        }

        let token_name = token.type_name.as_deref();
        if token.type_ == lexer::token::TokenType::Colon
            || matches!(token_name, Some("ACC_TITLE" | "ACC_DESCR"))
            || token.value.eq_ignore_ascii_case("title")
        {
            semantic_hash_context = true;
        }
        match token_name {
            Some("HASH_COMMENT") => continue,
            Some("HASH_COLOR" | "ENTITY") if !semantic_hash_context => {
                suppress_line = true;
                continue;
            }
            _ => filtered.push(token),
        }
    }
    filtered
}

pub fn tokenize_mermaid_quadrant(source: &str) -> Vec<Token> {
    try_tokenize_mermaid_quadrant(source)
        .unwrap_or_else(|e| panic!("Mermaid quadrant tokenization failed: {e}"))
}

pub fn try_tokenize_mermaid_quadrant(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_mermaid_quadrant_lexer(source);
    lexer.tokenize().map_err(|error| error.to_string())
}

pub fn try_tokenize_mermaid_journey(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_mermaid_journey_lexer(source);
    lexer.tokenize().map_err(|error| error.to_string())
}

pub fn try_tokenize_mermaid_xychart(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_mermaid_xychart_lexer(source);
    lexer.tokenize().map_err(|error| error.to_string())
}

pub fn try_tokenize_mermaid_gantt(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_mermaid_gantt_lexer(source);
    lexer.tokenize().map_err(|error| error.to_string())
}

pub fn try_tokenize_mermaid_requirement(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = create_mermaid_requirement_lexer(source);
    lexer.tokenize().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    fn custom_name(token: &Token) -> Option<&str> {
        token.type_name.as_deref()
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.69.0");
    }

    #[test]
    fn tokenizes_xychart_statements_case_insensitively() {
        let tokens = try_tokenize_mermaid_xychart(
            "XYCHART horizontal\nX-AXIS \"Quarter\" [Q1, Q2]\nLINE Forecast [1, 2]\n",
        )
        .unwrap();
        let names: Vec<_> = tokens
            .iter()
            .filter_map(|token| token.type_name.as_deref())
            .collect();
        for expected in [
            "HEADER",
            "ORIENTATION",
            "X_AXIS_STATEMENT",
            "LINE_STATEMENT",
        ] {
            assert!(names.contains(&expected));
        }
    }

    #[test]
    fn tokenizes_gantt_core_statements_case_insensitively() {
        let tokens = try_tokenize_mermaid_gantt(
            "GANTT\nTITLE Project\naccTitle: Timeline\nSECTION Build\nTask :done, t1, 2026-01-01, 2d\nCLICK t1 href \"https://example.com\" call inspect(t1)\n",
        )
        .unwrap();
        let names: Vec<_> = tokens
            .iter()
            .filter_map(|token| token.type_name.as_deref())
            .collect();
        assert!(names.contains(&"HEADER"));
        assert!(names.contains(&"TITLE_STATEMENT"));
        assert!(names.contains(&"ACC_TITLE_STATEMENT"));
        assert!(names.contains(&"SECTION_STATEMENT"));
        assert!(names.contains(&"TASK_STATEMENT"));
        assert!(names.contains(&"CLICK_STATEMENT"));
    }

    #[test]
    fn journey_tokenizes_sections_and_scored_tasks() {
        let tokens = try_tokenize_mermaid_journey(
            "JoUrNeY\naccTitle: Checkout\naccDescr {\nNative journey\n}\ntitle Checkout\nsection Payment<br/>Flow\nPay: 2: Alice, Bob",
        )
        .expect("journey tokenization");
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("HEADER")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("SECTION_STATEMENT")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("TASK_STATEMENT")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("ACC_TITLE_STATEMENT")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("ACC_DESCR_BLOCK")));
    }

    #[test]
    fn journey_rejects_scores_outside_the_documented_range() {
        for score in ["0", "6", "15"] {
            let source = format!("journey\nsection Work\nTask: {score}: Me");
            assert!(
                try_tokenize_mermaid_journey(&source).is_err(),
                "score {score}"
            );
        }
    }

    #[test]
    fn quadrant_tokenizes_native_statements() {
        let tokens = tokenize_mermaid_quadrant(
            "quadrantChart\ntitle Native portfolio\nx-axis Low --> High\nquadrant-1 Invest\nMetal: [0.75, 0.8]\n",
        );
        let names: Vec<_> = tokens.iter().filter_map(custom_name).collect();
        assert!(names.contains(&"HEADER"));
        assert!(names.contains(&"TITLE_STATEMENT"));
        assert!(names.contains(&"AXIS_STATEMENT"));
        assert!(names.contains(&"QUADRANT_STATEMENT"));
        assert!(names.contains(&"POINT_STATEMENT"));
    }

    #[test]
    fn quadrant_tokenizes_point_classes_and_styles() {
        let tokens = tokenize_mermaid_quadrant(
            "quadrantChart\nclassDef native color: #ff0000, radius: 10\nMetal:::native: [0.75, 0.8] stroke-width: 3px\n",
        );
        let names: Vec<_> = tokens.iter().filter_map(custom_name).collect();
        assert!(names.contains(&"CLASSDEF_STATEMENT"));
        assert!(names.contains(&"POINT_STATEMENT"));
    }

    #[test]
    fn quadrant_tokenizes_accessibility_metadata() {
        let tokens = tokenize_mermaid_quadrant(
            "quadrantChart\naccTitle: Portfolio matrix\naccDescr {\nNative renderer priorities\nacross backends\n}\n",
        );
        let names: Vec<_> = tokens.iter().filter_map(custom_name).collect();
        assert!(names.contains(&"ACC_TITLE_STATEMENT"));
        assert!(names.contains(&"ACC_DESCR_BLOCK"));
    }

    #[test]
    fn quadrant_tokenizes_keywords_case_insensitively() {
        let tokens = tokenize_mermaid_quadrant(
            "QuAdRaNtChArT\nTiTlE Native portfolio\nX-AxIs Low ---> High\nQuAdRaNt-1 Invest\nClAsSdEf native radius: 10\n",
        );
        let names: Vec<_> = tokens.iter().filter_map(custom_name).collect();
        for expected in [
            "HEADER",
            "TITLE_STATEMENT",
            "AXIS_STATEMENT",
            "QUADRANT_STATEMENT",
            "CLASSDEF_STATEMENT",
        ] {
            assert!(names.contains(&expected), "missing {expected}");
        }
    }

    #[test]
    fn quadrant_skips_leading_and_inline_comments() {
        let tokens = tokenize_mermaid_quadrant(
            "%% chart comment\nquadrantChart\nx-axis Low --> High %% axis comment\nMetal: [0.75, 0.8] %% point comment\n",
        );
        assert!(!tokens.iter().any(|token| token.value.contains("comment")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("AXIS_STATEMENT")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("POINT_STATEMENT")));
    }

    #[test]
    fn tokenizes_state_transitions_and_aliases() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\ndirection LR\nstate \"Still waiting\" as Still\n[*] --> Still\nStill --> [*]: done\n",
        );
        let names: Vec<_> = tokens.iter().filter_map(custom_name).collect();
        assert!(names.contains(&"HEADER"));
        assert_eq!(names.iter().filter(|name| **name == "ARROW").count(), 2);
        assert_eq!(
            names.iter().filter(|name| **name == "EDGE_STATE").count(),
            2
        );
        assert!(tokens
            .iter()
            .any(|token| token.value == "Still waiting" || token.value == "\"Still waiting\""));
    }

    #[test]
    fn tokenizes_state_choice_markers() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nstate First <<choice>>\nstate Second [[choice]]\n",
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("CHOICE"))
                .count(),
            2
        );
    }

    #[test]
    fn tokenizes_state_fork_and_join_markers() {
        let tokens =
            tokenize_mermaid_state("stateDiagram-v2\nstate Fork <<fork>>\nstate Join [[join]]\n");
        let markers: Vec<_> = tokens
            .iter()
            .filter(|token| token.type_name.as_deref() == Some("FORK_JOIN"))
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(markers, vec!["<<fork>>", "[[join]]"]);
    }

    #[test]
    fn tokenizes_state_inline_styles() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nstyle Ready fill:#fee2e2,stroke:#991b1b,color:#111827\n",
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("HASH_COLOR"))
                .count(),
            3
        );
        assert_eq!(tokens.iter().filter(|token| token.value == ",").count(), 2);
    }

    #[test]
    fn tokenizes_state_class_styles() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nclassDef warning fill:#fef3c7,stroke:#92400e\nclass Ready,Waiting warning\n",
        );
        assert!(tokens.iter().any(|token| token.value == "classDef"));
        assert!(tokens.iter().any(|token| token.value == "class"));
        assert_eq!(tokens.iter().filter(|token| token.value == ",").count(), 2);
    }

    #[test]
    fn tokenizes_state_inline_class_shorthand() {
        let tokens = tokenize_mermaid_state("stateDiagram-v2\nStill:::quiet --> Moving:::active\n");
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("STYLE_SEPARATOR"))
                .count(),
            2
        );
    }

    #[test]
    fn tokenizes_state_attached_notes() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nnote left of Ready: Waiting for work\nnote right of Running: Active\n",
        );
        assert_eq!(
            tokens.iter().filter(|token| token.value == "note").count(),
            2
        );
        assert_eq!(tokens.iter().filter(|token| token.value == "of").count(), 2);
        assert_eq!(tokens.iter().filter(|token| token.value == ":").count(), 2);
    }

    #[test]
    fn tokenizes_state_multiline_and_floating_notes() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nnote left of Ready\nFirst line\nSecond line\nend note\nnote \"Floating\" as N1\n",
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("END_NOTE"))
                .count(),
            1
        );
        assert!(tokens
            .iter()
            .any(|token| token.type_ == TokenType::String && token.value == "Floating"));
    }

    #[test]
    fn tokenizes_state_line_break_variants_as_single_tokens() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nReady: One<br>Two<br/>Three<br />Four<br\t/>Five\n",
        );
        let breaks: Vec<_> = tokens
            .iter()
            .filter(|token| token.type_name.as_deref() == Some("LINE_BREAK"))
            .map(|token| token.value.as_str())
            .collect();

        assert_eq!(breaks, ["<br>", "<br/>", "<br />", "<br\t/>"]);
    }

    #[test]
    fn tokenizes_internal_percent_state_identifiers_without_splitting() {
        let tokens =
            tokenize_mermaid_state("stateDiagram-v2\nMoving --> Still%Active\n% standalone\n");
        let values: Vec<_> = tokens.iter().map(|token| token.value.as_str()).collect();

        assert!(values.contains(&"Still%Active"));
        assert!(values.contains(&"%"));
    }

    #[test]
    fn tokenizes_state_accessibility_metadata() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\naccTitle: State lifecycle\naccDescr: Ready to running\naccDescr {\nMultiline description\n}\n",
        );
        for name in ["ACC_TITLE", "ACC_DESCR", "ACC_DESCR_START"] {
            assert!(
                tokens
                    .iter()
                    .any(|token| token.type_name.as_deref() == Some(name)),
                "missing {name} token"
            );
        }
        assert!(tokens.iter().any(|token| token.value == "}"));
    }

    #[test]
    fn tokenizes_state_click_links() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nclick Ready \"https://example.com\" \"Open ready state\"\nclick Running href \"https://example.com/run\"\n",
        );
        assert_eq!(
            tokens.iter().filter(|token| token.value == "click").count(),
            2
        );
        assert!(tokens.iter().any(|token| token.value == "href"));
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_ == TokenType::String)
                .count(),
            3
        );
    }

    #[test]
    fn tokenizes_state_composite_braces() {
        let tokens =
            tokenize_mermaid_state("stateDiagram-v2\nstate Processing {\nQueued --> Running\n}\n");
        assert!(tokens.iter().any(|token| token.value == "{"));
        assert!(tokens.iter().any(|token| token.value == "}"));
    }

    #[test]
    fn tokenizes_state_concurrent_region_divider() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nstate Active {\nOff --> On\n--\nIdle --> Busy\n}\n",
        );

        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("CONCURRENT")));
    }

    #[test]
    fn tokenizes_state_hide_empty_description_directive() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nhide empty description\nstate Junction <<choice>>\n",
        );

        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("HIDE_EMPTY")));
    }

    #[test]
    fn tokenizes_state_entities_before_hash_colors() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\nReady: Metal #9829; native\nstyle Ready fill:#dbeafe\n",
        );

        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("ENTITY")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("HASH_COLOR")));
    }

    #[test]
    fn skips_state_hash_comments_without_consuming_entities_or_colors() {
        let tokens = tokenize_mermaid_state(
            "stateDiagram-v2\n# standalone comment\n#abc color-looking comment\nReady: Metal #9829; native\nReady --> Running # inline comment\nRunning --> Done #123; entity-looking comment\nstyle Ready fill:#dbeafe\n# final comment",
        );

        assert!(!tokens.iter().any(|token| token.value.contains("comment")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("ENTITY")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("HASH_COLOR")));
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    #[test]
    fn tokenizes_header_shapes_and_edges() {
        let tokens = tokenize_mermaid("flowchart LR\nA[Start] -->|yes| B{Ship?}\n");

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "flowchart");
        assert_eq!(custom_name(&tokens[1]), Some("DIRECTION"));
        assert_eq!(tokens[1].value, "LR");
        assert_eq!(tokens[2].type_, TokenType::Newline);

        assert_eq!(tokens[3].type_, TokenType::Name);
        assert_eq!(tokens[3].value, "A");
        assert_eq!(custom_name(&tokens[4]), Some("RECT"));
        assert_eq!(tokens[4].value, "[Start]");

        assert_eq!(custom_name(&tokens[5]), Some("ARROW"));
        assert_eq!(custom_name(&tokens[6]), Some("EDGE_LABEL"));
        assert_eq!(tokens[6].value, "|yes|");

        assert_eq!(tokens[7].type_, TokenType::Name);
        assert_eq!(tokens[7].value, "B");
        assert_eq!(custom_name(&tokens[8]), Some("DIAMOND"));
        assert_eq!(tokens[8].value, "{Ship?}");
    }

    #[test]
    fn comments_are_skipped() {
        let tokens =
            tokenize_mermaid("%% heading comment\nflowchart TD\n%% edge comment\nA --- B\n");
        let values: Vec<&str> = tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();

        assert!(!values.iter().any(|v| v.contains("comment")));
        assert!(values.contains(&"flowchart"));
        assert!(values.contains(&"TD"));
        assert!(values.contains(&"A"));
        assert!(values.contains(&"B"));
    }

    #[test]
    fn shape_tokens_preserve_delimiters() {
        let tokens =
            tokenize_mermaid("flowchart TB\nA((Circle)); B(Round); C[Rect]; D{Decision}\n");
        let custom_tokens: Vec<(&str, &str)> = tokens
            .iter()
            .filter_map(|t| t.type_name.as_deref().map(|name| (name, t.value.as_str())))
            .collect();
        let semicolon_count = tokens
            .iter()
            .filter(|t| t.type_ == TokenType::Semicolon)
            .count();

        assert!(custom_tokens.contains(&("CIRCLE", "((Circle))")));
        assert!(custom_tokens.contains(&("ROUND", "(Round)")));
        assert!(custom_tokens.contains(&("RECT", "[Rect]")));
        assert!(custom_tokens.contains(&("DIAMOND", "{Decision}")));
        assert_eq!(semicolon_count, 3);
    }

    #[test]
    fn tokenizes_pie_sections() {
        let tokens = tokenize_mermaid_pie("pie showData\n\"Dogs\" : 60\n\"Cats\" : 40.5\n");
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert_eq!(
            values,
            vec!["pie", "showData", "\\n", "Dogs", ":", "60", "\\n", "Cats", ":", "40.5", "\\n"]
        );
    }

    #[test]
    fn tokenizes_sankey_csv_rows() {
        let tokens = tokenize_mermaid_sankey(
            "sankey-beta\nGrid,\"Heating, homes\",113.726\nGrid,Losses,56\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert_eq!(
            values,
            vec![
                "sankey-beta",
                "\\n",
                "Grid",
                ",",
                "Heating, homes",
                ",",
                "113.726",
                "\\n",
                "Grid",
                ",",
                "Losses",
                ",",
                "56",
                "\\n"
            ]
        );
    }

    #[test]
    fn tokenizes_gitgraph_commands_and_attributes() {
        let tokens = tokenize_mermaid_gitgraph(
            "gitGraph LR:\ncommit id: \"c1\" msg: \"Start\"\nbranch develop\ncheckout develop\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert_eq!(values[0], "gitGraph");
        assert!(values.contains(&"LR"));
        assert!(values.contains(&"commit"));
        assert!(values.contains(&"c1"));
        assert!(values.contains(&"develop"));
    }

    #[test]
    fn tokenizes_er_relationships_and_attributes() {
        let tokens = tokenize_mermaid_er(
            "erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring name PK\n}\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert!(values.contains(&"erDiagram"));
        assert!(values.contains(&"||"));
        assert!(values.contains(&"--"));
        assert!(values.contains(&"o{"));
        assert!(values.contains(&"PK"));
    }

    #[test]
    fn tokenizes_c4_macros_and_keyed_arguments() {
        let tokens = tokenize_mermaid_c4(
            "C4Context\nPerson(user, \"Customer\", $sprite=\"person\")\nRel(user, bank, \"Uses\")\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert!(values.contains(&"C4Context"));
        assert!(values.contains(&"Person"));
        assert!(values.contains(&"$sprite"));
        assert!(values.contains(&"Rel"));
    }

    #[test]
    fn tokenizes_sequence_participants_messages_and_notes() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nparticipant A as Alice\nA->>+Bob: Hello Bob\nnote right of Bob: Ready\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();

        assert!(values.contains(&"sequenceDiagram"));
        assert!(values.contains(&"->>"));
        assert!(values.contains(&"+"));
        assert!(values.contains(&"note"));
        assert!(values.contains(&"Ready"));
    }

    #[test]
    fn tokenizes_sequence_keywords_case_insensitively() {
        let tokens = tokenize_mermaid_sequence(
            "SeQuEnCeDiAgRaM\nPaRtIcIpAnT A As Alice\nNoTe RiGhT Of A: WRAP: Ready\n",
        );
        let values: Vec<&str> = tokens.iter().map(|token| token.value.as_str()).collect();

        assert!(values.contains(&"participant"));
        assert!(values.contains(&"as"));
        assert!(values.contains(&"note"));
        assert!(values.contains(&"right"));
        assert!(values.contains(&"of"));
        assert!(values.contains(&"wrap:"));
    }

    #[test]
    fn tokenizes_sequence_control_blocks() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nalt Ready\nA->>B: Go\nelse Waiting\nloop Retry\nB-->>A: Wait\nend\nend\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();
        assert!(values.contains(&"alt"));
        assert!(values.contains(&"else"));
        assert!(values.contains(&"loop"));
        assert_eq!(values.iter().filter(|value| **value == "end").count(), 2);
    }

    #[test]
    fn tokenizes_sequence_participant_lifecycle() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\ncreate participant Worker as Background Worker\ndestroy Worker\n",
        );
        let values: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .map(|token| token.value.as_str())
            .collect();
        assert!(values.contains(&"create"));
        assert!(values.contains(&"destroy"));
        assert!(values.contains(&"Worker"));
    }

    #[test]
    fn tokenizes_sequence_box_color_as_one_token() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nbox rgba(33,66,99,0.5) Services\nparticipant API\nend\n",
        );
        assert!(tokens
            .iter()
            .any(|token| token.value == "rgba(33,66,99,0.5)"));
    }

    #[test]
    fn tokenizes_sequence_hsl_colors_as_one_token() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nbox hsl(180, 100%, 50%) Services\nparticipant API\nend\nrect hsla(30, 100%, 50%, 0.25)\nAPI->>DB: Query\nend\n",
        );
        let colors: Vec<_> = tokens
            .iter()
            .filter(|token| token.type_name.as_deref() == Some("COLOR"))
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(
            colors,
            vec!["hsl(180, 100%, 50%)", "hsla(30, 100%, 50%, 0.25)"]
        );
    }

    #[test]
    fn tokenizes_sequence_participant_configuration() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nparticipant API@{ \"type\": \"boundary\", \"alias\": \"Public API\" }\n",
        );
        assert!(tokens.iter().any(|token| token.value == "API"));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("CONFIG")));
    }

    #[test]
    fn tokenizes_sequence_half_arrows() {
        let tokens = tokenize_mermaid_sequence(
            r#"sequenceDiagram
A-|\B: filled top
A-|/B: filled bottom
A-\\B: stick top
A-//B: stick bottom
A/|-B: reverse filled top
A\\-B: reverse stick bottom
"#,
        );
        let names: Vec<&str> = tokens
            .iter()
            .filter_map(|token| token.type_name.as_deref())
            .collect();
        assert!(names.contains(&"SOLID_FILLED_TOP"));
        assert!(names.contains(&"SOLID_FILLED_BOTTOM"));
        assert!(names.contains(&"SOLID_STICK_TOP"));
        assert!(names.contains(&"SOLID_STICK_BOTTOM"));
        assert!(names.contains(&"SOLID_REVERSE_FILLED_TOP"));
        assert!(names.contains(&"SOLID_REVERSE_STICK_BOTTOM"));
    }

    #[test]
    fn tokenizes_sequence_central_connections() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nAlice->>()John: destination\nAlice()->>John: source\nJohn()->>()Alice: both\n",
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("CENTRAL"))
                .count(),
            4
        );
    }

    #[test]
    fn tokenizes_sequence_autonumber_decimals() {
        let tokens = tokenize_mermaid_sequence("sequenceDiagram\nautonumber 10.5 2.25\n");
        let numbers: Vec<&str> = tokens
            .iter()
            .filter(|token| token.type_ == TokenType::Number)
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(numbers, vec!["10.5", "2.25"]);
    }

    #[test]
    fn tokenizes_sequence_actor_links() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nlink Alice: Dashboard @ https://example.com/alice\nlinks Bob: {\"Wiki\": \"https://example.com/wiki\"}\n",
        );
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("URL")));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("JSON_OBJECT")));
    }

    #[test]
    fn tokenizes_sequence_actor_properties() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nproperties Alice: {\"role\": \"admin\", \"active\": true}\n",
        );
        assert!(tokens.iter().any(|token| token.value == "properties"));
        assert!(tokens
            .iter()
            .any(|token| token.type_name.as_deref() == Some("JSON_OBJECT")));
    }

    #[test]
    fn tokenizes_sequence_actor_details_reference() {
        let tokens = tokenize_mermaid_sequence("sequenceDiagram\ndetails Alice: alice-info\n");
        assert!(tokens.iter().any(|token| token.value == "details"));
        assert!(tokens.iter().any(|token| token.value == "alice"));
        assert!(tokens.iter().any(|token| token.value == "-"));
        assert!(tokens.iter().any(|token| token.value == "info"));
    }

    #[test]
    fn tokenizes_sequence_accessibility_statements() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\naccTitle: Transfer flow\naccDescr: Banking interaction\n",
        );
        assert!(tokens.iter().any(|token| token.value == "accTitle"));
        assert!(tokens.iter().any(|token| token.value == "accDescr"));
    }

    #[test]
    fn tokenizes_multiline_sequence_accessibility_description() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\naccDescr {\n  Transfers funds\n  between accounts\n}\n",
        );
        assert!(tokens.iter().any(|token| {
            token.type_name.as_deref() == Some("ACC_DESCR_BLOCK")
                && token.value.contains("between accounts")
        }));
    }

    #[test]
    fn tokenizes_sequence_semicolon_terminators() {
        let tokens =
            tokenize_mermaid_sequence("sequenceDiagram;participant Alice;Alice->>Bob: Hello;");
        assert_eq!(tokens.iter().filter(|token| token.value == ";").count(), 3);
    }

    #[test]
    fn tokenizes_sequence_entities_without_splitting_the_semicolon() {
        let tokens =
            tokenize_mermaid_sequence("sequenceDiagram\nAlice->>Bob: I #9829; you #infin; times\n");
        let entities: Vec<_> = tokens
            .iter()
            .filter(|token| token.type_name.as_deref() == Some("ENTITY"))
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(entities, vec!["#9829;", "#infin;"]);
        assert!(!tokens.iter().any(|token| token.value == ";"));
    }

    #[test]
    fn skips_sequence_hash_comments_without_dropping_entities() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\n# heading comment\nA->>B: I #9829; diagrams # trailing comment\n",
        );
        assert!(!tokens.iter().any(|token| token.value.contains("comment")));
        assert!(tokens.iter().any(|token| token.value == "#9829;"));
    }

    #[test]
    fn tokenizes_sequence_line_break_variants() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nAlice->>Bob: One<br>Two<br/>Three<br />Four\n",
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.type_name.as_deref() == Some("LINE_BREAK"))
                .count(),
            3
        );
    }

    #[test]
    fn tokenizes_sequence_wrap_directives() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nAlice->>Bob: wrap: A long message\nnote over Alice,Bob: nowrap: A note\n",
        );
        let directives: Vec<_> = tokens
            .iter()
            .filter(|token| token.type_name.as_deref() == Some("WRAP_DIRECTIVE"))
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(directives, vec!["wrap:", "nowrap:"]);
    }

    #[test]
    fn tokenizes_multiword_sequence_actor_references() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nparticipant Customer Portal\nCustomer Portal->>Order Service: Submit\n",
        );
        let values: Vec<_> = tokens.iter().map(|token| token.value.as_str()).collect();
        assert!(values.windows(2).any(|pair| pair == ["Customer", "Portal"]));
        assert!(values.windows(2).any(|pair| pair == ["Order", "Service"]));
    }

    #[test]
    fn tokenizes_hyphenated_sequence_actor_references() {
        let tokens = tokenize_mermaid_sequence(
            "sequenceDiagram\nparticipant Customer-Portal\nCustomer-Portal->>Order-Service: Submit\n",
        );
        assert_eq!(tokens.iter().filter(|token| token.value == "-").count(), 3);
    }
}
