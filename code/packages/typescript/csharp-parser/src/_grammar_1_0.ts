// AUTO-GENERATED FILE - DO NOT EDIT
// Source: csharp1.0.grammar
// Regenerate with: grammar-tools compile-grammar csharp1.0.grammar
//
// This file embeds a ParserGrammar as native TypeScript object literals.
// Import it directly instead of reading and parsing the .grammar file at
// runtime.

import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PARSER_GRAMMAR: ParserGrammar = {
  version: 1,
  rules: [
  {
    name: "compilation_unit",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "global_attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
    ] },
    lineNumber: 96,
  },
  {
    name: "extern_alias_directive",
    body: { type: "sequence", elements: [
      { type: "literal", value: "extern" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 117,
  },
  {
    name: "using_directive",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "using" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "using" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "EQUALS" },
        { type: "rule_reference", name: "qualified_name" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 143,
  },
  {
    name: "qualified_name",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "DOT" },
          { type: "token_reference", name: "NAME" },
        ] } },
    ] },
    lineNumber: 160,
  },
  {
    name: "global_attribute_section",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "global_attribute_target" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "attribute_list" },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 179,
  },
  {
    name: "global_attribute_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "assembly" },
      { type: "literal", value: "module" },
    ] },
    lineNumber: 181,
  },
  {
    name: "namespace_declaration",
    body: { type: "sequence", elements: [
      { type: "literal", value: "namespace" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "extern_alias_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "using_directive" } },
      { type: "repetition", element: { type: "rule_reference", name: "namespace_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 212,
  },
  {
    name: "namespace_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "namespace_declaration" },
      { type: "rule_reference", name: "type_declaration" },
    ] },
    lineNumber: 226,
  },
  {
    name: "type_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "class_declaration" },
      { type: "rule_reference", name: "struct_declaration" },
      { type: "rule_reference", name: "interface_declaration" },
      { type: "rule_reference", name: "enum_declaration" },
      { type: "rule_reference", name: "delegate_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 229,
  },
  {
    name: "attribute_section",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "attribute_target" },
          { type: "token_reference", name: "COLON" },
        ] } },
      { type: "rule_reference", name: "attribute_list" },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 259,
  },
  {
    name: "attribute_target",
    body: { type: "alternation", choices: [
      { type: "literal", value: "field" },
      { type: "literal", value: "event" },
      { type: "literal", value: "method" },
      { type: "literal", value: "param" },
      { type: "literal", value: "property" },
      { type: "literal", value: "return" },
      { type: "literal", value: "type" },
    ] },
    lineNumber: 261,
  },
  {
    name: "attribute_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "attribute" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "attribute" },
        ] } },
    ] },
    lineNumber: 269,
  },
  {
    name: "attribute",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "optional", element: { type: "rule_reference", name: "attribute_arguments" } },
          { type: "token_reference", name: "RPAREN" },
        ] } },
    ] },
    lineNumber: 271,
  },
  {
    name: "attribute_arguments",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "attribute_argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "attribute_argument" },
        ] } },
    ] },
    lineNumber: 273,
  },
  {
    name: "attribute_argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "EQUALS" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 277,
  },
  {
    name: "class_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "class_modifier" } },
      { type: "literal", value: "class" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "class_base_list" },
        ] } },
      { type: "rule_reference", name: "class_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 302,
  },
  {
    name: "class_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "sealed" },
    ] },
    lineNumber: 306,
  },
  {
    name: "class_base_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "qualified_name" },
        ] } },
    ] },
    lineNumber: 324,
  },
  {
    name: "class_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "class_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 326,
  },
  {
    name: "class_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "property_declaration" },
      { type: "rule_reference", name: "event_declaration" },
      { type: "rule_reference", name: "indexer_declaration" },
      { type: "rule_reference", name: "operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "destructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 348,
  },
  {
    name: "constant_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "constant_modifier" } },
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 379,
  },
  {
    name: "constant_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 382,
  },
  {
    name: "constant_declarators",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "constant_declarator" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "constant_declarator" },
        ] } },
    ] },
    lineNumber: 388,
  },
  {
    name: "constant_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "EQUALS" },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 390,
  },
  {
    name: "field_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "field_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 413,
  },
  {
    name: "field_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "readonly" },
      { type: "literal", value: "volatile" },
    ] },
    lineNumber: 416,
  },
  {
    name: "variable_declarators",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "variable_declarator" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "variable_declarator" },
        ] } },
    ] },
    lineNumber: 432,
  },
  {
    name: "variable_declarator",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "variable_initializer" },
        ] } },
    ] },
    lineNumber: 434,
  },
  {
    name: "variable_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "expression" },
      { type: "rule_reference", name: "array_initializer" },
    ] },
    lineNumber: 436,
  },
  {
    name: "array_initializer",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "rule_reference", name: "variable_initializer" },
          { type: "repetition", element: { type: "sequence", elements: [
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "variable_initializer" },
            ] } },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 448,
  },
  {
    name: "method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "method_modifier" } },
      { type: "rule_reference", name: "return_type" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 479,
  },
  {
    name: "method_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 483,
  },
  {
    name: "return_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "void" },
      { type: "rule_reference", name: "type" },
    ] },
    lineNumber: 495,
  },
  {
    name: "method_body",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 501,
  },
  {
    name: "formal_parameter_list",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "fixed_parameters" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "parameter_array" },
          ] } },
      ] },
      { type: "rule_reference", name: "parameter_array" },
    ] },
    lineNumber: 538,
  },
  {
    name: "fixed_parameters",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "fixed_parameter" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "fixed_parameter" },
        ] } },
    ] },
    lineNumber: 541,
  },
  {
    name: "fixed_parameter",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "parameter_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 543,
  },
  {
    name: "parameter_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "ref" },
      { type: "literal", value: "out" },
    ] },
    lineNumber: 545,
  },
  {
    name: "parameter_array",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "params" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 551,
  },
  {
    name: "property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "property_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 584,
  },
  {
    name: "property_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 587,
  },
  {
    name: "accessor_declarations",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "get_accessor_declaration" },
        { type: "optional", element: { type: "rule_reference", name: "set_accessor_declaration" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "set_accessor_declaration" },
        { type: "optional", element: { type: "rule_reference", name: "get_accessor_declaration" } },
      ] },
    ] },
    lineNumber: 603,
  },
  {
    name: "get_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "literal", value: "get" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 606,
  },
  {
    name: "set_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "rule_reference", name: "accessor_modifier" } },
      { type: "literal", value: "set" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "rule_reference", name: "block" },
          { type: "token_reference", name: "SEMICOLON" },
        ] } },
    ] },
    lineNumber: 609,
  },
  {
    name: "accessor_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "sequence", elements: [
        { type: "literal", value: "protected" },
        { type: "literal", value: "internal" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "internal" },
        { type: "literal", value: "protected" },
      ] },
    ] },
    lineNumber: 615,
  },
  {
    name: "event_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "event_modifier" } },
      { type: "literal", value: "event" },
      { type: "rule_reference", name: "type" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "variable_declarators" },
            { type: "token_reference", name: "SEMICOLON" },
          ] },
          { type: "sequence", elements: [
            { type: "rule_reference", name: "qualified_name" },
            { type: "token_reference", name: "LBRACE" },
            { type: "rule_reference", name: "event_accessor_declarations" },
            { type: "token_reference", name: "RBRACE" },
          ] },
        ] } },
    ] },
    lineNumber: 651,
  },
  {
    name: "event_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "static" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 655,
  },
  {
    name: "event_accessor_declarations",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "add_accessor_declaration" },
        { type: "rule_reference", name: "remove_accessor_declaration" },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "remove_accessor_declaration" },
        { type: "rule_reference", name: "add_accessor_declaration" },
      ] },
    ] },
    lineNumber: 667,
  },
  {
    name: "add_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "add" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 670,
  },
  {
    name: "remove_accessor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "literal", value: "remove" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 672,
  },
  {
    name: "indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "indexer_modifier" } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "accessor_declarations" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 703,
  },
  {
    name: "indexer_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
      { type: "literal", value: "virtual" },
      { type: "literal", value: "sealed" },
      { type: "literal", value: "override" },
      { type: "literal", value: "abstract" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 707,
  },
  {
    name: "operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "operator_modifiers" },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "operator" },
      { type: "rule_reference", name: "overloadable_operator" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "type" },
          { type: "token_reference", name: "NAME" },
        ] } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 765,
  },
  {
    name: "operator_modifiers",
    body: { type: "sequence", elements: [
      { type: "literal", value: "public" },
      { type: "literal", value: "static" },
      { type: "optional", element: { type: "literal", value: "extern" } },
    ] },
    lineNumber: 770,
  },
  {
    name: "overloadable_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "PLUS" },
      { type: "token_reference", name: "MINUS" },
      { type: "token_reference", name: "BANG" },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "PLUS_PLUS" },
      { type: "token_reference", name: "MINUS_MINUS" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "token_reference", name: "STAR" },
      { type: "token_reference", name: "SLASH" },
      { type: "token_reference", name: "PERCENT" },
      { type: "token_reference", name: "AMPERSAND" },
      { type: "token_reference", name: "PIPE" },
      { type: "token_reference", name: "CARET" },
      { type: "token_reference", name: "LEFT_SHIFT" },
      { type: "token_reference", name: "RIGHT_SHIFT" },
      { type: "token_reference", name: "EQUALS_EQUALS" },
      { type: "token_reference", name: "NOT_EQUALS" },
      { type: "token_reference", name: "LESS_THAN" },
      { type: "token_reference", name: "GREATER_THAN" },
      { type: "token_reference", name: "LESS_EQUALS" },
      { type: "token_reference", name: "GREATER_EQUALS" },
    ] },
    lineNumber: 776,
  },
  {
    name: "conversion_operator_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "operator_modifiers" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "literal", value: "implicit" },
          { type: "literal", value: "explicit" },
        ] } },
      { type: "literal", value: "operator" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 826,
  },
  {
    name: "constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "constructor_modifier" } },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "constructor_initializer" } },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 861,
  },
  {
    name: "constructor_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "extern" },
    ] },
    lineNumber: 865,
  },
  {
    name: "constructor_initializer",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "literal", value: "base" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "COLON" },
        { type: "literal", value: "this" },
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
    ] },
    lineNumber: 874,
  },
  {
    name: "destructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "extern" } },
      { type: "token_reference", name: "TILDE" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 907,
  },
  {
    name: "static_constructor_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "rule_reference", name: "static_constructor_modifiers" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "method_body" },
    ] },
    lineNumber: 937,
  },
  {
    name: "static_constructor_modifiers",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "static" },
        { type: "optional", element: { type: "literal", value: "extern" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "extern" },
        { type: "literal", value: "static" },
      ] },
    ] },
    lineNumber: 940,
  },
  {
    name: "struct_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "struct_modifier" } },
      { type: "literal", value: "struct" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "struct_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 985,
  },
  {
    name: "struct_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 989,
  },
  {
    name: "interface_type_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "qualified_name" },
        ] } },
    ] },
    lineNumber: 997,
  },
  {
    name: "struct_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "struct_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 999,
  },
  {
    name: "struct_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "constant_declaration" },
      { type: "rule_reference", name: "field_declaration" },
      { type: "rule_reference", name: "method_declaration" },
      { type: "rule_reference", name: "property_declaration" },
      { type: "rule_reference", name: "event_declaration" },
      { type: "rule_reference", name: "indexer_declaration" },
      { type: "rule_reference", name: "operator_declaration" },
      { type: "rule_reference", name: "conversion_operator_declaration" },
      { type: "rule_reference", name: "constructor_declaration" },
      { type: "rule_reference", name: "static_constructor_declaration" },
      { type: "rule_reference", name: "type_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1004,
  },
  {
    name: "interface_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "interface_modifier" } },
      { type: "literal", value: "interface" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "interface_type_list" },
        ] } },
      { type: "rule_reference", name: "interface_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 1055,
  },
  {
    name: "interface_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 1059,
  },
  {
    name: "interface_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "interface_member_declaration" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1065,
  },
  {
    name: "interface_member_declaration",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "interface_method_declaration" },
      { type: "rule_reference", name: "interface_property_declaration" },
      { type: "rule_reference", name: "interface_event_declaration" },
      { type: "rule_reference", name: "interface_indexer_declaration" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1067,
  },
  {
    name: "interface_method_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1076,
  },
  {
    name: "interface_property_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "interface_accessors" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1082,
  },
  {
    name: "interface_accessors",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "get" },
        { type: "token_reference", name: "SEMICOLON" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "set" },
            { type: "token_reference", name: "SEMICOLON" },
          ] } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "set" },
        { type: "token_reference", name: "SEMICOLON" },
        { type: "optional", element: { type: "sequence", elements: [
            { type: "literal", value: "get" },
            { type: "token_reference", name: "SEMICOLON" },
          ] } },
      ] },
    ] },
    lineNumber: 1085,
  },
  {
    name: "interface_event_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "literal", value: "event" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1090,
  },
  {
    name: "interface_indexer_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "optional", element: { type: "literal", value: "new" } },
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "this" },
      { type: "token_reference", name: "LBRACKET" },
      { type: "rule_reference", name: "formal_parameter_list" },
      { type: "token_reference", name: "RBRACKET" },
      { type: "token_reference", name: "LBRACE" },
      { type: "rule_reference", name: "interface_accessors" },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1094,
  },
  {
    name: "enum_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "enum_modifier" } },
      { type: "literal", value: "enum" },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "integral_type" },
        ] } },
      { type: "rule_reference", name: "enum_body" },
      { type: "optional", element: { type: "token_reference", name: "SEMICOLON" } },
    ] },
    lineNumber: 1129,
  },
  {
    name: "enum_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 1133,
  },
  {
    name: "integral_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "byte" },
      { type: "literal", value: "sbyte" },
      { type: "literal", value: "short" },
      { type: "literal", value: "ushort" },
      { type: "literal", value: "int" },
      { type: "literal", value: "uint" },
      { type: "literal", value: "long" },
      { type: "literal", value: "ulong" },
    ] },
    lineNumber: 1139,
  },
  {
    name: "enum_body",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "optional", element: { type: "rule_reference", name: "enum_member_declarations" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1148,
  },
  {
    name: "enum_member_declarations",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "enum_member_declaration" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "enum_member_declaration" },
        ] } },
      { type: "optional", element: { type: "token_reference", name: "COMMA" } },
    ] },
    lineNumber: 1150,
  },
  {
    name: "enum_member_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "token_reference", name: "NAME" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "EQUALS" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1153,
  },
  {
    name: "delegate_declaration",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "attribute_section" } },
      { type: "repetition", element: { type: "rule_reference", name: "delegate_modifier" } },
      { type: "literal", value: "delegate" },
      { type: "rule_reference", name: "return_type" },
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "formal_parameter_list" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1184,
  },
  {
    name: "delegate_modifier",
    body: { type: "alternation", choices: [
      { type: "literal", value: "public" },
      { type: "literal", value: "protected" },
      { type: "literal", value: "internal" },
      { type: "literal", value: "private" },
      { type: "literal", value: "new" },
    ] },
    lineNumber: 1188,
  },
  {
    name: "type",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "value_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "reference_type" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "void" },
        { type: "token_reference", name: "STAR" },
      ] },
    ] },
    lineNumber: 1233,
  },
  {
    name: "value_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "primitive_type" },
      { type: "rule_reference", name: "qualified_name" },
    ] },
    lineNumber: 1237,
  },
  {
    name: "reference_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "literal", value: "object" },
      { type: "literal", value: "string" },
    ] },
    lineNumber: 1240,
  },
  {
    name: "primitive_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "numeric_type" },
      { type: "literal", value: "bool" },
    ] },
    lineNumber: 1244,
  },
  {
    name: "numeric_type",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "integral_type" },
      { type: "rule_reference", name: "floating_point_type" },
      { type: "literal", value: "decimal" },
    ] },
    lineNumber: 1247,
  },
  {
    name: "floating_point_type",
    body: { type: "alternation", choices: [
      { type: "literal", value: "float" },
      { type: "literal", value: "double" },
    ] },
    lineNumber: 1251,
  },
  {
    name: "rank_specifier",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACKET" },
      { type: "repetition", element: { type: "token_reference", name: "COMMA" } },
      { type: "token_reference", name: "RBRACKET" },
    ] },
    lineNumber: 1267,
  },
  {
    name: "pointer_type",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "STAR" },
    ] },
    lineNumber: 1278,
  },
  {
    name: "block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1305,
  },
  {
    name: "statement",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "block" },
      { type: "rule_reference", name: "local_variable_declaration_statement" },
      { type: "rule_reference", name: "local_constant_declaration_statement" },
      { type: "rule_reference", name: "empty_statement" },
      { type: "rule_reference", name: "expression_statement" },
      { type: "rule_reference", name: "if_statement" },
      { type: "rule_reference", name: "while_statement" },
      { type: "rule_reference", name: "do_while_statement" },
      { type: "rule_reference", name: "for_statement" },
      { type: "rule_reference", name: "foreach_statement" },
      { type: "rule_reference", name: "switch_statement" },
      { type: "rule_reference", name: "try_statement" },
      { type: "rule_reference", name: "throw_statement" },
      { type: "rule_reference", name: "return_statement" },
      { type: "rule_reference", name: "break_statement" },
      { type: "rule_reference", name: "continue_statement" },
      { type: "rule_reference", name: "goto_statement" },
      { type: "rule_reference", name: "lock_statement" },
      { type: "rule_reference", name: "using_statement" },
      { type: "rule_reference", name: "checked_statement" },
      { type: "rule_reference", name: "unchecked_statement" },
      { type: "rule_reference", name: "labelled_statement" },
      { type: "rule_reference", name: "unsafe_statement" },
      { type: "rule_reference", name: "fixed_statement" },
    ] },
    lineNumber: 1307,
  },
  {
    name: "local_variable_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1343,
  },
  {
    name: "local_constant_declaration_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "const" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "constant_declarators" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1353,
  },
  {
    name: "empty_statement",
    body: { type: "token_reference", name: "SEMICOLON" },
    lineNumber: 1360,
  },
  {
    name: "expression_statement",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1372,
  },
  {
    name: "if_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "if" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "literal", value: "else" },
          { type: "rule_reference", name: "statement" },
        ] } },
    ] },
    lineNumber: 1389,
  },
  {
    name: "while_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "while" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1399,
  },
  {
    name: "do_while_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "do" },
      { type: "rule_reference", name: "statement" },
      { type: "literal", value: "while" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1410,
  },
  {
    name: "for_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "for" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "for_initializer" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
      { type: "optional", element: { type: "rule_reference", name: "for_iterator" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1425,
  },
  {
    name: "for_initializer",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "local_variable_declaration" },
      { type: "rule_reference", name: "expression_list" },
    ] },
    lineNumber: 1428,
  },
  {
    name: "for_iterator",
    body: { type: "rule_reference", name: "expression_list" },
    lineNumber: 1431,
  },
  {
    name: "local_variable_declaration",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
    ] },
    lineNumber: 1433,
  },
  {
    name: "expression_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1435,
  },
  {
    name: "foreach_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "foreach" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "NAME" },
      { type: "literal", value: "in" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1463,
  },
  {
    name: "switch_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "switch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "switch_block" },
    ] },
    lineNumber: 1494,
  },
  {
    name: "switch_block",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LBRACE" },
      { type: "repetition", element: { type: "rule_reference", name: "switch_section" } },
      { type: "token_reference", name: "RBRACE" },
    ] },
    lineNumber: 1496,
  },
  {
    name: "switch_section",
    body: { type: "sequence", elements: [
      { type: "repetition", element: { type: "rule_reference", name: "switch_label" } },
      { type: "repetition", element: { type: "rule_reference", name: "statement" } },
    ] },
    lineNumber: 1498,
  },
  {
    name: "switch_label",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "COLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "default" },
        { type: "token_reference", name: "COLON" },
      ] },
    ] },
    lineNumber: 1500,
  },
  {
    name: "try_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "try" },
      { type: "rule_reference", name: "block" },
      { type: "group", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "catch_clauses" },
            { type: "optional", element: { type: "rule_reference", name: "finally_clause" } },
          ] },
          { type: "rule_reference", name: "finally_clause" },
        ] } },
    ] },
    lineNumber: 1523,
  },
  {
    name: "catch_clauses",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "specific_catch_clause" },
        { type: "repetition", element: { type: "rule_reference", name: "specific_catch_clause" } },
        { type: "optional", element: { type: "rule_reference", name: "general_catch_clause" } },
      ] },
      { type: "rule_reference", name: "general_catch_clause" },
    ] },
    lineNumber: 1526,
  },
  {
    name: "specific_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "qualified_name" },
      { type: "optional", element: { type: "token_reference", name: "NAME" } },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1529,
  },
  {
    name: "general_catch_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "catch" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1531,
  },
  {
    name: "finally_clause",
    body: { type: "sequence", elements: [
      { type: "literal", value: "finally" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1533,
  },
  {
    name: "throw_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "throw" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1550,
  },
  {
    name: "return_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "return" },
      { type: "optional", element: { type: "rule_reference", name: "expression" } },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1554,
  },
  {
    name: "break_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "break" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1565,
  },
  {
    name: "continue_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "continue" },
      { type: "token_reference", name: "SEMICOLON" },
    ] },
    lineNumber: 1567,
  },
  {
    name: "goto_statement",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "token_reference", name: "NAME" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "literal", value: "case" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "goto" },
        { type: "literal", value: "default" },
        { type: "token_reference", name: "SEMICOLON" },
      ] },
    ] },
    lineNumber: 1599,
  },
  {
    name: "lock_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "lock" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1620,
  },
  {
    name: "using_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "using" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "resource_acquisition" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1643,
  },
  {
    name: "resource_acquisition",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "type" },
        { type: "rule_reference", name: "variable_declarators" },
      ] },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 1645,
  },
  {
    name: "checked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1674,
  },
  {
    name: "unchecked_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1676,
  },
  {
    name: "labelled_statement",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "NAME" },
      { type: "token_reference", name: "COLON" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1688,
  },
  {
    name: "unsafe_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unsafe" },
      { type: "rule_reference", name: "block" },
    ] },
    lineNumber: 1714,
  },
  {
    name: "fixed_statement",
    body: { type: "sequence", elements: [
      { type: "literal", value: "fixed" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "rule_reference", name: "variable_declarators" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "statement" },
    ] },
    lineNumber: 1739,
  },
  {
    name: "expression",
    body: { type: "rule_reference", name: "assignment_expression" },
    lineNumber: 1786,
  },
  {
    name: "assignment_expression",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "conditional_expression" },
      { type: "sequence", elements: [
        { type: "rule_reference", name: "unary_expression" },
        { type: "rule_reference", name: "assignment_operator" },
        { type: "rule_reference", name: "assignment_expression" },
      ] },
    ] },
    lineNumber: 1788,
  },
  {
    name: "assignment_operator",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "EQUALS" },
      { type: "token_reference", name: "PLUS_EQUALS" },
      { type: "token_reference", name: "MINUS_EQUALS" },
      { type: "token_reference", name: "STAR_EQUALS" },
      { type: "token_reference", name: "SLASH_EQUALS" },
      { type: "token_reference", name: "PERCENT_EQUALS" },
      { type: "token_reference", name: "AMPERSAND_EQUALS" },
      { type: "token_reference", name: "PIPE_EQUALS" },
      { type: "token_reference", name: "CARET_EQUALS" },
      { type: "token_reference", name: "LEFT_SHIFT_EQUALS" },
      { type: "token_reference", name: "RIGHT_SHIFT_EQUALS" },
    ] },
    lineNumber: 1791,
  },
  {
    name: "conditional_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_or_expression" },
      { type: "optional", element: { type: "sequence", elements: [
          { type: "token_reference", name: "QUESTION" },
          { type: "rule_reference", name: "expression" },
          { type: "token_reference", name: "COLON" },
          { type: "rule_reference", name: "expression" },
        ] } },
    ] },
    lineNumber: 1809,
  },
  {
    name: "logical_or_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "logical_and_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "OR_OR" },
          { type: "rule_reference", name: "logical_and_expression" },
        ] } },
    ] },
    lineNumber: 1818,
  },
  {
    name: "logical_and_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_or_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AND_AND" },
          { type: "rule_reference", name: "bitwise_or_expression" },
        ] } },
    ] },
    lineNumber: 1826,
  },
  {
    name: "bitwise_or_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_xor_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "PIPE" },
          { type: "rule_reference", name: "bitwise_xor_expression" },
        ] } },
    ] },
    lineNumber: 1834,
  },
  {
    name: "bitwise_xor_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "bitwise_and_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "CARET" },
          { type: "rule_reference", name: "bitwise_and_expression" },
        ] } },
    ] },
    lineNumber: 1842,
  },
  {
    name: "bitwise_and_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "equality_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "AMPERSAND" },
          { type: "rule_reference", name: "equality_expression" },
        ] } },
    ] },
    lineNumber: 1850,
  },
  {
    name: "equality_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "relational_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "EQUALS_EQUALS" },
              { type: "token_reference", name: "NOT_EQUALS" },
            ] } },
          { type: "rule_reference", name: "relational_expression" },
        ] } },
    ] },
    lineNumber: 1862,
  },
  {
    name: "relational_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "shift_expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "group", element: { type: "alternation", choices: [
                { type: "token_reference", name: "LESS_THAN" },
                { type: "token_reference", name: "GREATER_THAN" },
                { type: "token_reference", name: "LESS_EQUALS" },
                { type: "token_reference", name: "GREATER_EQUALS" },
              ] } },
            { type: "rule_reference", name: "shift_expression" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "is" },
            { type: "rule_reference", name: "type" },
          ] },
          { type: "sequence", elements: [
            { type: "literal", value: "as" },
            { type: "rule_reference", name: "type" },
          ] },
        ] } },
    ] },
    lineNumber: 1885,
  },
  {
    name: "shift_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "additive_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "LEFT_SHIFT" },
              { type: "token_reference", name: "RIGHT_SHIFT" },
            ] } },
          { type: "rule_reference", name: "additive_expression" },
        ] } },
    ] },
    lineNumber: 1904,
  },
  {
    name: "additive_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "multiplicative_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" },
            ] } },
          { type: "rule_reference", name: "multiplicative_expression" },
        ] } },
    ] },
    lineNumber: 1919,
  },
  {
    name: "multiplicative_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "unary_expression" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" },
              { type: "token_reference", name: "PERCENT" },
            ] } },
          { type: "rule_reference", name: "unary_expression" },
        ] } },
    ] },
    lineNumber: 1932,
  },
  {
    name: "unary_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS_PLUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS_MINUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "PLUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "MINUS" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "BANG" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "TILDE" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "rule_reference", name: "cast_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "AMPERSAND" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "STAR" },
        { type: "rule_reference", name: "unary_expression" },
      ] },
      { type: "rule_reference", name: "postfix_expression" },
    ] },
    lineNumber: 1950,
  },
  {
    name: "cast_expression",
    body: { type: "sequence", elements: [
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
      { type: "rule_reference", name: "unary_expression" },
    ] },
    lineNumber: 1971,
  },
  {
    name: "postfix_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary_expression" },
      { type: "repetition", element: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS_PLUS" },
          { type: "token_reference", name: "MINUS_MINUS" },
        ] } },
    ] },
    lineNumber: 1980,
  },
  {
    name: "primary_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "primary" },
      { type: "repetition", element: { type: "rule_reference", name: "primary_suffix" } },
    ] },
    lineNumber: 2000,
  },
  {
    name: "primary_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "ARROW" },
        { type: "token_reference", name: "NAME" },
      ] },
    ] },
    lineNumber: 2005,
  },
  {
    name: "primary",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "literal" },
      { type: "literal", value: "this" },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "DOT" },
        { type: "token_reference", name: "NAME" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "base" },
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
      ] },
      { type: "rule_reference", name: "typeof_expression" },
      { type: "rule_reference", name: "sizeof_expression" },
      { type: "rule_reference", name: "checked_expression" },
      { type: "rule_reference", name: "unchecked_expression" },
      { type: "rule_reference", name: "default_value_expression" },
      { type: "rule_reference", name: "new_expression" },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LPAREN" },
        { type: "rule_reference", name: "expression" },
        { type: "token_reference", name: "RPAREN" },
      ] },
      { type: "token_reference", name: "NAME" },
    ] },
    lineNumber: 2012,
  },
  {
    name: "typeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "typeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type_or_void" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2035,
  },
  {
    name: "type_or_void",
    body: { type: "alternation", choices: [
      { type: "rule_reference", name: "type" },
      { type: "literal", value: "void" },
    ] },
    lineNumber: 2037,
  },
  {
    name: "sizeof_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "sizeof" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2047,
  },
  {
    name: "checked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "checked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2056,
  },
  {
    name: "unchecked_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "unchecked" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "expression" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2058,
  },
  {
    name: "default_value_expression",
    body: { type: "sequence", elements: [
      { type: "literal", value: "default" },
      { type: "token_reference", name: "LPAREN" },
      { type: "rule_reference", name: "type" },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2072,
  },
  {
    name: "new_expression",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_object_expression" },
      ] },
      { type: "sequence", elements: [
        { type: "literal", value: "new" },
        { type: "rule_reference", name: "new_array_expression" },
      ] },
    ] },
    lineNumber: 2094,
  },
  {
    name: "new_object_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "qualified_name" },
      { type: "token_reference", name: "LPAREN" },
      { type: "optional", element: { type: "rule_reference", name: "argument_list" } },
      { type: "token_reference", name: "RPAREN" },
    ] },
    lineNumber: 2099,
  },
  {
    name: "new_array_expression",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "array_type" },
      { type: "rule_reference", name: "array_creation_suffix" },
    ] },
    lineNumber: 2108,
  },
  {
    name: "array_type",
    body: { type: "group", element: { type: "alternation", choices: [
        { type: "rule_reference", name: "primitive_type" },
        { type: "rule_reference", name: "qualified_name" },
      ] } },
    lineNumber: 2110,
  },
  {
    name: "array_creation_suffix",
    body: { type: "alternation", choices: [
      { type: "sequence", elements: [
        { type: "rule_reference", name: "rank_specifier" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
        { type: "rule_reference", name: "array_initializer" },
      ] },
      { type: "sequence", elements: [
        { type: "token_reference", name: "LBRACKET" },
        { type: "rule_reference", name: "expression_list" },
        { type: "token_reference", name: "RBRACKET" },
        { type: "repetition", element: { type: "rule_reference", name: "rank_specifier" } },
        { type: "optional", element: { type: "rule_reference", name: "array_initializer" } },
      ] },
    ] },
    lineNumber: 2112,
  },
  {
    name: "argument_list",
    body: { type: "sequence", elements: [
      { type: "rule_reference", name: "argument" },
      { type: "repetition", element: { type: "sequence", elements: [
          { type: "token_reference", name: "COMMA" },
          { type: "rule_reference", name: "argument" },
        ] } },
    ] },
    lineNumber: 2123,
  },
  {
    name: "argument",
    body: { type: "sequence", elements: [
      { type: "optional", element: { type: "alternation", choices: [
          { type: "literal", value: "ref" },
          { type: "literal", value: "out" },
        ] } },
      { type: "rule_reference", name: "expression" },
    ] },
    lineNumber: 2125,
  },
  {
    name: "literal",
    body: { type: "alternation", choices: [
      { type: "token_reference", name: "NUMBER" },
      { type: "token_reference", name: "CHAR" },
      { type: "token_reference", name: "STRING" },
      { type: "token_reference", name: "VERBATIM_STRING" },
      { type: "literal", value: "true" },
      { type: "literal", value: "false" },
      { type: "literal", value: "null" },
    ] },
    lineNumber: 2142,
  },
],
};
