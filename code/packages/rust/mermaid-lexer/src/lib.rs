//! Grammar-driven lexers for Mermaid diagram families.

pub const VERSION: &str = "0.5.0";

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
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Mermaid sequence tokenization failed: {e}"))
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
        assert_eq!(VERSION, "0.5.0");
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
}
