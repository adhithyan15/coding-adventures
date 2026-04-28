use coding_adventures_html_lexer::{Attribute, HtmlLexer, Token};
use state_machine::EffectfulStateMachine;
use state_machine_markup_deserializer::from_states_toml;

const HTML1_LEXER_TOML: &str = include_str!("../html1.lexer.states.toml");

#[test]
fn html1_authoring_artifact_parses_as_mosaic_era_floor() {
    let definition = from_states_toml(HTML1_LEXER_TOML).unwrap();

    assert_eq!(definition.name, "html1-lexer");
    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(definition.fixtures.len(), 6);
    assert!(definition.states.iter().any(|state| state.id == "comment"));
    assert!(definition
        .states
        .iter()
        .any(|state| state.id == "doctype_name"));
    assert!(definition.states.iter().any(|state| state.id == "rcdata"));
    assert!(definition.states.iter().any(|state| state.id == "rawtext"));
}

#[test]
fn html1_authoring_fixtures_execute_through_generic_runtime() {
    let definition = from_states_toml(HTML1_LEXER_TOML).unwrap();

    for fixture in &definition.fixtures {
        let machine = EffectfulStateMachine::from_definition(&definition).unwrap();
        let mut lexer = HtmlLexer::new(machine);
        lexer.push(&fixture.input).unwrap();
        lexer.finish().unwrap();
        let actual = lexer
            .drain_tokens()
            .into_iter()
            .map(token_summary)
            .collect::<Vec<_>>();
        assert_eq!(
            actual, fixture.tokens,
            "fixture `{}` should match",
            fixture.name
        );
    }
}

#[test]
fn html1_authoring_reports_comment_eof_diagnostic() {
    let definition = from_states_toml(HTML1_LEXER_TOML).unwrap();
    let machine = EffectfulStateMachine::from_definition(&definition).unwrap();
    let mut lexer = HtmlLexer::new(machine);

    lexer.push("<!--open").unwrap();
    lexer.finish().unwrap();

    assert_eq!(
        lexer.drain_tokens(),
        vec![Token::Comment("open".to_string()), Token::Eof]
    );
    assert!(lexer
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "eof-in-comment"));
}

fn token_summary(token: Token) -> String {
    match token {
        Token::Text(data) => format!("Text(data={data})"),
        Token::StartTag {
            name,
            attributes,
            self_closing,
        } => format!(
            "StartTag(name={name}, attributes={}, self_closing={self_closing})",
            attribute_summary(&attributes)
        ),
        Token::EndTag { name } => format!("EndTag(name={name})"),
        Token::Comment(data) => format!("Comment(data={data})"),
        Token::Doctype { name, force_quirks } => match name {
            Some(name) => format!("Doctype(name={name}, force_quirks={force_quirks})"),
            None => format!("Doctype(name=null, force_quirks={force_quirks})"),
        },
        Token::Eof => "EOF".to_string(),
    }
}

fn attribute_summary(attributes: &[Attribute]) -> String {
    if attributes.is_empty() {
        "[]".to_string()
    } else {
        let joined = attributes
            .iter()
            .map(|attribute| format!("{}={}", attribute.name, attribute.value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{joined}]")
    }
}
