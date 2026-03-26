// AUTO-GENERATED FILE - DO NOT EDIT
use crate::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};
use crate::token_grammar::{PatternGroup, TokenDefinition, TokenGrammar};

pub fn compile_tokens_to_rust(grammar: &TokenGrammar, export_name: &str) -> String {
    let mut lines = Vec::new();
    lines.push("// AUTO-GENERATED FILE - DO NOT EDIT".to_string());
    lines.push("#![allow(clippy::all)]".to_string());
    lines.push("use std::collections::HashMap;".to_string());
    lines.push("use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition, PatternGroup};".to_string());
    lines.push("".to_string());
    lines.push(format!("pub fn {}() -> TokenGrammar {{", export_name));

    if !grammar.groups.is_empty() {
        lines.push("    let mut groups = HashMap::new();".to_string());
        for (gname, group) in &grammar.groups {
            lines.push(format!("    groups.insert({:?}.to_string(), PatternGroup {{", gname));
            lines.push(format!("        name: {:?}.to_string(),", group.name));
            lines.push("        definitions: vec![".to_string());
            for defn in &group.definitions {
                lines.push(format!("            {},", _compile_token_def(defn)));
            }
            lines.push("        ],".to_string());
            lines.push("    });".to_string());
        }
    } else {
        lines.push("    let groups = HashMap::new();".to_string());
    }

    lines.push("    TokenGrammar {".to_string());
    lines.push(format!("        version: {},", grammar.version));
    lines.push(format!("        case_insensitive: {},", grammar.case_insensitive));
    lines.push(format!("        case_sensitive: {},", grammar.case_sensitive));
    
    if let Some(mode) = &grammar.mode {
        lines.push(format!("        mode: Some({:?}.to_string()),", mode));
    } else {
        lines.push("        mode: None,".to_string());
    }
    
    if let Some(escapes) = &grammar.escapes {
        lines.push(format!("        escapes: Some({:?}.to_string()),", escapes));
    } else {
        lines.push("        escapes: None,".to_string());
    }

    lines.push(format!("        keywords: vec![{}],", grammar.keywords.iter().map(|k| format!("{:?}.to_string()", k)).collect::<Vec<_>>().join(", ")));
    lines.push(format!("        reserved_keywords: vec![{}],", grammar.reserved_keywords.iter().map(|k| format!("{:?}.to_string()", k)).collect::<Vec<_>>().join(", ")));

    lines.push("        definitions: vec![".to_string());
    for defn in &grammar.definitions {
        lines.push(format!("            {},", _compile_token_def(defn)));
    }
    lines.push("        ],".to_string());

    lines.push("        skip_definitions: vec![".to_string());
    for defn in &grammar.skip_definitions {
        lines.push(format!("            {},", _compile_token_def(defn)));
    }
    lines.push("        ],".to_string());

    lines.push("        error_definitions: vec![".to_string());
    for defn in &grammar.error_definitions {
        lines.push(format!("            {},", _compile_token_def(defn)));
    }
    lines.push("        ],".to_string());

    lines.push("        groups,".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    
    lines.join("\n") + "\n"
}

fn _compile_token_def(defn: &TokenDefinition) -> String {
    let alias_str = match &defn.alias {
        Some(a) => format!("Some({:?}.to_string())", a),
        None => "None".to_string(),
    };
    format!(
        "TokenDefinition {{ name: {:?}.to_string(), \
         pattern: {:?}.to_string(), \
         is_regex: {}, \
         line_number: {}, \
         alias: {} }}",
        defn.name, defn.pattern, defn.is_regex, defn.line_number, alias_str
    )
}

pub fn compile_parser_to_rust(grammar: &ParserGrammar, export_name: &str) -> String {
    let mut lines = Vec::new();
    lines.push("// AUTO-GENERATED FILE - DO NOT EDIT".to_string());
    lines.push("#![allow(clippy::all)]".to_string());
    lines.push("use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};".to_string());
    lines.push("".to_string());
    lines.push(format!("pub fn {}() -> ParserGrammar {{", export_name));
    lines.push("    ParserGrammar {".to_string());
    lines.push(format!("        version: {},", grammar.version));
    lines.push("        rules: vec![".to_string());
    for rule in &grammar.rules {
        lines.push("            GrammarRule {".to_string());
        lines.push(format!("                name: {:?}.to_string(),", rule.name));
        lines.push(format!("                line_number: {},", rule.line_number));
        lines.push(format!("                body: {},", _compile_grammar_element(&rule.body)));
        lines.push("            },".to_string());
    }
    lines.push("        ],".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.join("\n") + "\n"
}

fn _compile_grammar_element(el: &GrammarElement) -> String {
    match el {
        GrammarElement::RuleReference { name } => {
            format!("GrammarElement::RuleReference {{ name: {:?}.to_string() }}", name)
        }
        GrammarElement::TokenReference { name } => {
            format!("GrammarElement::TokenReference {{ name: {:?}.to_string() }}", name)
        }
        GrammarElement::Literal { value } => {
            format!("GrammarElement::Literal {{ value: {:?}.to_string() }}", value)
        }
        GrammarElement::Sequence { elements } => {
            let elems: Vec<String> = elements.iter().map(_compile_grammar_element).collect();
            format!("GrammarElement::Sequence {{ elements: vec![{}] }}", elems.join(", "))
        }
        GrammarElement::Alternation { choices } => {
            let choices_str: Vec<String> = choices.iter().map(_compile_grammar_element).collect();
            format!("GrammarElement::Alternation {{ choices: vec![{}] }}", choices_str.join(", "))
        }
        GrammarElement::Repetition { element } => {
            format!("GrammarElement::Repetition {{ element: Box::new({}) }}", _compile_grammar_element(element))
        }
        GrammarElement::Optional { element } => {
            format!("GrammarElement::Optional {{ element: Box::new({}) }}", _compile_grammar_element(element))
        }
        GrammarElement::Group { element } => {
            format!("GrammarElement::Group {{ element: Box::new({}) }}", _compile_grammar_element(element))
        }
    }
}
