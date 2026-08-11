//! Grammar-driven lexers for Mermaid diagram families.

pub const VERSION: &str = "0.28.0";

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
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid state tokenization failed: {e}"))
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
        assert_eq!(VERSION, "0.28.0");
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
