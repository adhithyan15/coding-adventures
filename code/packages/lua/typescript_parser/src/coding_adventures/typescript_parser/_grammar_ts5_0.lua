-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: ts5.0.grammar
-- Regenerate with: grammar-tools compile-grammar ts5.0.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="repetition", element={ type="rule_reference", name="source_element", is_token=false } },
      line_number=81,
    },
    {
      name="source_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="import_declaration", is_token=false },
        { type="rule_reference", name="export_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="generator_declaration", is_token=false },
        { type="rule_reference", name="async_function_declaration", is_token=false },
        { type="rule_reference", name="async_generator_declaration", is_token=false },
        { type="rule_reference", name="ts_class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="type_alias_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="ambient_declaration", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="decorator", is_token=false },
          { type="rule_reference", name="ts_class_declaration", is_token=false },
        } },
        { type="rule_reference", name="lexical_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=83,
    },
    {
      name="function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=103,
    },
    {
      name="function_body",
      body={ type="repetition", element={ type="rule_reference", name="source_element", is_token=false } },
      line_number=107,
    },
    {
      name="generator_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=109,
    },
    {
      name="generator_expression",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=113,
    },
    {
      name="yield_expression",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="optional", element={ type="rule_reference", name="STAR", is_token=true } },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=117,
    },
    {
      name="async_function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=119,
    },
    {
      name="async_function_expression",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="literal", value="function" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=123,
    },
    {
      name="async_arrow_function",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="rule_reference", name="arrow_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="concise_body", is_token=false },
      } },
      line_number=127,
    },
    {
      name="async_method",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="optional", element={ type="rule_reference", name="STAR", is_token=true } },
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=129,
    },
    {
      name="await_expression",
      body={ type="sequence", elements={
        { type="literal", value="await" },
        { type="rule_reference", name="unary_expression", is_token=false },
      } },
      line_number=133,
    },
    {
      name="async_generator_declaration",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=135,
    },
    {
      name="async_generator_expression",
      body={ type="sequence", elements={
        { type="literal", value="async" },
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=140,
    },
    {
      name="lexical_declaration",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="literal", value="let" },
            { type="literal", value="const" },
          } } },
        { type="rule_reference", name="binding_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=145,
    },
    {
      name="binding_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="lexical_binding", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="lexical_binding", is_token=false },
          } } },
      } },
      line_number=147,
    },
    {
      name="lexical_binding",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="binding_pattern", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=149,
    },
    {
      name="using_declaration",
      body={ type="sequence", elements={
        { type="literal", value="using" },
        { type="rule_reference", name="binding_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=165,
    },
    {
      name="await_using_declaration",
      body={ type="sequence", elements={
        { type="literal", value="await" },
        { type="literal", value="using" },
        { type="rule_reference", name="binding_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=174,
    },
    {
      name="import_declaration",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="import" },
          { type="rule_reference", name="import_clause", is_token=false },
          { type="rule_reference", name="from_clause", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="import" },
          { type="rule_reference", name="module_specifier", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="import" },
          { type="literal", value="type" },
          { type="rule_reference", name="import_clause", is_token=false },
          { type="rule_reference", name="from_clause", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=180,
    },
    {
      name="import_clause",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="default_import", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="named_imports", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="default_import", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="namespace_import", is_token=false },
            } } },
        } },
        { type="rule_reference", name="named_imports", is_token=false },
        { type="rule_reference", name="namespace_import", is_token=false },
      } },
      line_number=184,
    },
    {
      name="default_import",
      body={ type="rule_reference", name="NAME", is_token=true },
      line_number=189,
    },
    {
      name="named_imports",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="import_specifier", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="import_specifier", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=191,
    },
    {
      name="import_specifier",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="as" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=193,
    },
    {
      name="namespace_import",
      body={ type="sequence", elements={
        { type="rule_reference", name="STAR", is_token=true },
        { type="literal", value="as" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=195,
    },
    {
      name="from_clause",
      body={ type="sequence", elements={
        { type="literal", value="from" },
        { type="rule_reference", name="STRING", is_token=true },
      } },
      line_number=197,
    },
    {
      name="module_specifier",
      body={ type="rule_reference", name="STRING", is_token=true },
      line_number=199,
    },
    {
      name="export_declaration",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="literal", value="default" },
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="function_declaration", is_token=false },
              { type="rule_reference", name="generator_declaration", is_token=false },
              { type="rule_reference", name="async_function_declaration", is_token=false },
              { type="rule_reference", name="async_generator_declaration", is_token=false },
              { type="rule_reference", name="ts_class_declaration", is_token=false },
              { type="rule_reference", name="interface_declaration", is_token=false },
              { type="rule_reference", name="type_alias_declaration", is_token=false },
              { type="sequence", elements={
                { type="rule_reference", name="assignment_expression", is_token=false },
                { type="rule_reference", name="SEMICOLON", is_token=true },
              } },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="function_declaration", is_token=false },
              { type="rule_reference", name="generator_declaration", is_token=false },
              { type="rule_reference", name="async_function_declaration", is_token=false },
              { type="rule_reference", name="async_generator_declaration", is_token=false },
              { type="rule_reference", name="ts_class_declaration", is_token=false },
              { type="rule_reference", name="interface_declaration", is_token=false },
              { type="rule_reference", name="type_alias_declaration", is_token=false },
              { type="rule_reference", name="enum_declaration", is_token=false },
              { type="rule_reference", name="namespace_declaration", is_token=false },
              { type="rule_reference", name="lexical_declaration", is_token=false },
              { type="rule_reference", name="variable_statement", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="rule_reference", name="named_exports", is_token=false },
          { type="optional", element={ type="rule_reference", name="from_clause", is_token=false } },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="from_clause", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="literal", value="type" },
          { type="rule_reference", name="named_exports", is_token=false },
          { type="optional", element={ type="rule_reference", name="from_clause", is_token=false } },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="export" },
          { type="literal", value="type" },
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="from_clause", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=201,
    },
    {
      name="named_exports",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="export_specifier", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="export_specifier", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=220,
    },
    {
      name="export_specifier",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="as" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=222,
    },
    {
      name="binding_pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="object_binding_pattern", is_token=false },
        { type="rule_reference", name="array_binding_pattern", is_token=false },
      } },
      line_number=228,
    },
    {
      name="object_binding_pattern",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="binding_property", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="binding_property", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="optional", element={ type="rule_reference", name="object_rest_property", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=231,
    },
    {
      name="object_rest_property",
      body={ type="sequence", elements={
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=234,
    },
    {
      name="array_binding_pattern",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="binding_element", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="binding_element", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=236,
    },
    {
      name="binding_property",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="binding_element", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="EQUALS", is_token=true },
              { type="rule_reference", name="assignment_expression", is_token=false },
            } } },
        } },
      } },
      line_number=238,
    },
    {
      name="binding_element",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="NAME", is_token=true },
              { type="rule_reference", name="binding_pattern", is_token=false },
            } } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="EQUALS", is_token=true },
              { type="rule_reference", name="assignment_expression", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="ELLIPSIS", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=241,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="block", is_token=false },
        { type="rule_reference", name="variable_statement", is_token=false },
        { type="rule_reference", name="empty_statement", is_token=false },
        { type="rule_reference", name="if_statement", is_token=false },
        { type="rule_reference", name="while_statement", is_token=false },
        { type="rule_reference", name="do_while_statement", is_token=false },
        { type="rule_reference", name="for_statement", is_token=false },
        { type="rule_reference", name="for_in_statement", is_token=false },
        { type="rule_reference", name="for_of_statement", is_token=false },
        { type="rule_reference", name="for_await_of_statement", is_token=false },
        { type="rule_reference", name="continue_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="with_statement", is_token=false },
        { type="rule_reference", name="switch_statement", is_token=false },
        { type="rule_reference", name="labelled_statement", is_token=false },
        { type="rule_reference", name="try_statement", is_token=false },
        { type="rule_reference", name="throw_statement", is_token=false },
        { type="rule_reference", name="debugger_statement", is_token=false },
        { type="rule_reference", name="lexical_declaration", is_token=false },
        { type="rule_reference", name="using_declaration", is_token=false },
        { type="rule_reference", name="await_using_declaration", is_token=false },
        { type="rule_reference", name="expression_statement", is_token=false },
      } },
      line_number=248,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=272,
    },
    {
      name="variable_statement",
      body={ type="sequence", elements={
        { type="literal", value="var" },
        { type="rule_reference", name="variable_declaration_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=274,
    },
    {
      name="variable_declaration_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="variable_declaration", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="variable_declaration", is_token=false },
          } } },
      } },
      line_number=276,
    },
    {
      name="variable_declaration",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="binding_pattern", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=278,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=280,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=282,
    },
    {
      name="if_statement",
      body={ type="sequence", elements={
        { type="literal", value="if" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="else" },
            { type="rule_reference", name="statement", is_token=false },
          } } },
      } },
      line_number=284,
    },
    {
      name="while_statement",
      body={ type="sequence", elements={
        { type="literal", value="while" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=286,
    },
    {
      name="do_while_statement",
      body={ type="sequence", elements={
        { type="literal", value="do" },
        { type="rule_reference", name="statement", is_token=false },
        { type="literal", value="while" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=288,
    },
    {
      name="for_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="var" },
              { type="rule_reference", name="variable_declaration_list", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="let" },
              { type="rule_reference", name="binding_list", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="const" },
              { type="rule_reference", name="binding_list", is_token=false },
            } },
            { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=290,
    },
    {
      name="for_in_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="var" },
              { type="rule_reference", name="variable_declaration", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="let" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="const" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="rule_reference", name="left_hand_side_expression", is_token=false },
          } } },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=299,
    },
    {
      name="for_of_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="var" },
              { type="rule_reference", name="variable_declaration", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="let" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="const" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="using" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="rule_reference", name="left_hand_side_expression", is_token=false },
          } } },
        { type="literal", value="of" },
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=306,
    },
    {
      name="for_await_of_statement",
      body={ type="sequence", elements={
        { type="literal", value="for" },
        { type="literal", value="await" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="var" },
              { type="rule_reference", name="variable_declaration", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="let" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="const" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="using" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="await" },
              { type="literal", value="using" },
              { type="rule_reference", name="binding_element", is_token=false },
            } },
            { type="rule_reference", name="left_hand_side_expression", is_token=false },
          } } },
        { type="literal", value="of" },
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=314,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=323,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=325,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=327,
    },
    {
      name="with_statement",
      body={ type="sequence", elements={
        { type="literal", value="with" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=329,
    },
    {
      name="switch_statement",
      body={ type="sequence", elements={
        { type="literal", value="switch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="case_clause", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="default_clause", is_token=false },
            { type="repetition", element={ type="rule_reference", name="case_clause", is_token=false } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=331,
    },
    {
      name="case_clause",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=334,
    },
    {
      name="default_clause",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=336,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=338,
    },
    {
      name="try_statement",
      body={ type="sequence", elements={
        { type="literal", value="try" },
        { type="rule_reference", name="block", is_token=false },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="catch_clause", is_token=false },
              { type="optional", element={ type="rule_reference", name="finally_clause", is_token=false } },
            } },
            { type="rule_reference", name="finally_clause", is_token=false },
          } } },
      } },
      line_number=340,
    },
    {
      name="catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="LPAREN", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="RPAREN", is_token=true },
          } } },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=342,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=344,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=346,
    },
    {
      name="debugger_statement",
      body={ type="sequence", elements={
        { type="literal", value="debugger" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=348,
    },
    {
      name="expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=354,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="conditional_expression", is_token=false },
        { type="rule_reference", name="arrow_function", is_token=false },
        { type="rule_reference", name="async_arrow_function", is_token=false },
        { type="rule_reference", name="yield_expression", is_token=false },
        { type="rule_reference", name="ts_as_expression", is_token=false },
        { type="rule_reference", name="ts_satisfies_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="left_hand_side_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
      } },
      line_number=356,
    },
    {
      name="assignment_operator",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="PLUS_EQUALS", is_token=true },
        { type="rule_reference", name="MINUS_EQUALS", is_token=true },
        { type="rule_reference", name="STAR_STAR_EQUALS", is_token=true },
        { type="rule_reference", name="STAR_EQUALS", is_token=true },
        { type="rule_reference", name="SLASH_EQUALS", is_token=true },
        { type="rule_reference", name="PERCENT_EQUALS", is_token=true },
        { type="rule_reference", name="AMPERSAND_EQUALS", is_token=true },
        { type="rule_reference", name="PIPE_EQUALS", is_token=true },
        { type="rule_reference", name="CARET_EQUALS", is_token=true },
        { type="rule_reference", name="LEFT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="RIGHT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="UNSIGNED_RIGHT_SHIFT_EQUALS", is_token=true },
        { type="rule_reference", name="OR_OR_EQUALS", is_token=true },
        { type="rule_reference", name="AND_AND_EQUALS", is_token=true },
        { type="rule_reference", name="NULLISH_COALESCE_EQUALS", is_token=true },
      } },
      line_number=365,
    },
    {
      name="conditional_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="nullish_coalescing_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=372,
    },
    {
      name="nullish_coalescing_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_or_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="NULLISH_COALESCE", is_token=true },
            { type="rule_reference", name="logical_or_expression", is_token=false },
          } } },
      } },
      line_number=375,
    },
    {
      name="logical_or_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_and_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="OR_OR", is_token=true },
            { type="rule_reference", name="logical_and_expression", is_token=false },
          } } },
      } },
      line_number=378,
    },
    {
      name="logical_and_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_or_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AND_AND", is_token=true },
            { type="rule_reference", name="bitwise_or_expression", is_token=false },
          } } },
      } },
      line_number=380,
    },
    {
      name="bitwise_or_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_xor_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="bitwise_xor_expression", is_token=false },
          } } },
      } },
      line_number=382,
    },
    {
      name="bitwise_xor_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise_and_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="CARET", is_token=true },
            { type="rule_reference", name="bitwise_and_expression", is_token=false },
          } } },
      } },
      line_number=384,
    },
    {
      name="bitwise_and_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="equality_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMPERSAND", is_token=true },
            { type="rule_reference", name="equality_expression", is_token=false },
          } } },
      } },
      line_number=386,
    },
    {
      name="equality_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="relational_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STRICT_EQUALS", is_token=true },
                { type="rule_reference", name="STRICT_NOT_EQUALS", is_token=true },
                { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
                { type="rule_reference", name="NOT_EQUALS", is_token=true },
              } } },
            { type="rule_reference", name="relational_expression", is_token=false },
          } } },
      } },
      line_number=388,
    },
    {
      name="relational_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="shift_expression", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="group", element={ type="alternation", choices={
                  { type="rule_reference", name="LESS_THAN", is_token=true },
                  { type="rule_reference", name="GREATER_THAN", is_token=true },
                  { type="rule_reference", name="LESS_EQUALS", is_token=true },
                  { type="rule_reference", name="GREATER_EQUALS", is_token=true },
                  { type="literal", value="instanceof" },
                  { type="literal", value="in" },
                } } },
              { type="rule_reference", name="shift_expression", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="PRIVATE_NAME", is_token=true },
          { type="literal", value="in" },
          { type="rule_reference", name="shift_expression", is_token=false },
        } },
      } },
      line_number=393,
    },
    {
      name="shift_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="additive_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="LEFT_SHIFT", is_token=true },
                { type="rule_reference", name="RIGHT_SHIFT", is_token=true },
                { type="rule_reference", name="UNSIGNED_RIGHT_SHIFT", is_token=true },
              } } },
            { type="rule_reference", name="additive_expression", is_token=false },
          } } },
      } },
      line_number=398,
    },
    {
      name="additive_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="multiplicative_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="PLUS", is_token=true },
                { type="rule_reference", name="MINUS", is_token=true },
              } } },
            { type="rule_reference", name="multiplicative_expression", is_token=false },
          } } },
      } },
      line_number=401,
    },
    {
      name="multiplicative_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="exponentiation_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="rule_reference", name="PERCENT", is_token=true },
              } } },
            { type="rule_reference", name="exponentiation_expression", is_token=false },
          } } },
      } },
      line_number=404,
    },
    {
      name="exponentiation_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="STAR_STAR", is_token=true },
            { type="rule_reference", name="exponentiation_expression", is_token=false },
          } } },
      } },
      line_number=407,
    },
    {
      name="unary_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="postfix_expression", is_token=false },
        { type="rule_reference", name="await_expression", is_token=false },
        { type="sequence", elements={
          { type="literal", value="delete" },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="void" },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="typeof" },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="PLUS_PLUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS_MINUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="PLUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="TILDE", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="BANG", is_token=true },
          { type="rule_reference", name="unary_expression", is_token=false },
        } },
      } },
      line_number=409,
    },
    {
      name="postfix_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="left_hand_side_expression", is_token=false },
        { type="optional", element={ type="alternation", choices={
            { type="rule_reference", name="PLUS_PLUS", is_token=true },
            { type="rule_reference", name="MINUS_MINUS", is_token=true },
            { type="rule_reference", name="BANG", is_token=true },
          } } },
      } },
      line_number=421,
    },
    {
      name="left_hand_side_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="call_expression", is_token=false },
        { type="rule_reference", name="optional_chain_expression", is_token=false },
        { type="rule_reference", name="new_expression", is_token=false },
      } },
      line_number=423,
    },
    {
      name="call_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="member_expression", is_token=false },
        { type="rule_reference", name="arguments", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="rule_reference", name="arguments", is_token=false },
            { type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="PRIVATE_NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } },
            { type="rule_reference", name="template_literal", is_token=false },
          } } },
      } },
      line_number=427,
    },
    {
      name="optional_chain_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="member_expression", is_token=false },
        { type="repetition", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="rule_reference", name="OPTIONAL_CHAIN", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="OPTIONAL_CHAIN", is_token=true },
              { type="rule_reference", name="PRIVATE_NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="OPTIONAL_CHAIN", is_token=true },
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="OPTIONAL_CHAIN", is_token=true },
              { type="rule_reference", name="arguments", is_token=false },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="DOT", is_token=true },
              { type="rule_reference", name="PRIVATE_NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } },
            { type="rule_reference", name="arguments", is_token=false },
            { type="rule_reference", name="template_literal", is_token=false },
          } } },
      } },
      line_number=431,
    },
    {
      name="new_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="member_expression", is_token=false },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="new_expression", is_token=false },
        } },
      } },
      line_number=442,
    },
    {
      name="member_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="primary_expression", is_token=false },
          { type="repetition", element={ type="alternation", choices={
              { type="sequence", elements={
                { type="rule_reference", name="DOT", is_token=true },
                { type="rule_reference", name="NAME", is_token=true },
              } },
              { type="sequence", elements={
                { type="rule_reference", name="DOT", is_token=true },
                { type="rule_reference", name="PRIVATE_NAME", is_token=true },
              } },
              { type="sequence", elements={
                { type="rule_reference", name="LBRACKET", is_token=true },
                { type="rule_reference", name="expression", is_token=false },
                { type="rule_reference", name="RBRACKET", is_token=true },
              } },
              { type="rule_reference", name="template_literal", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="member_expression", is_token=false },
          { type="rule_reference", name="arguments", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="super" },
          { type="rule_reference", name="DOT", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="super" },
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="DOT", is_token=true },
          { type="literal", value="target" },
        } },
      } },
      line_number=446,
    },
    {
      name="arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="argument_list", is_token=false },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=454,
    },
    {
      name="argument_list",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="spread_element", is_token=false },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="spread_element", is_token=false },
                { type="rule_reference", name="assignment_expression", is_token=false },
              } } },
          } } },
      } },
      line_number=456,
    },
    {
      name="spread_element",
      body={ type="sequence", elements={
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=459,
    },
    {
      name="arrow_function",
      body={ type="sequence", elements={
        { type="rule_reference", name="arrow_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="concise_body", is_token=false },
      } },
      line_number=461,
    },
    {
      name="arrow_parameters",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="LESS_THAN", is_token=true },
          { type="rule_reference", name="type_parameter_list", is_token=false },
          { type="rule_reference", name="GREATER_THAN", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
        } },
      } },
      line_number=463,
    },
    {
      name="concise_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=467,
    },
    {
      name="primary_expression",
      body={ type="alternation", choices={
        { type="literal", value="this" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="BIGINT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="REGEX", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="literal", value="null" },
        { type="rule_reference", name="array_literal", is_token=false },
        { type="rule_reference", name="object_literal", is_token=false },
        { type="rule_reference", name="function_expression", is_token=false },
        { type="rule_reference", name="generator_expression", is_token=false },
        { type="rule_reference", name="async_function_expression", is_token=false },
        { type="rule_reference", name="ts_class_expression", is_token=false },
        { type="rule_reference", name="template_literal", is_token=false },
        { type="rule_reference", name="dynamic_import", is_token=false },
        { type="rule_reference", name="import_meta", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=470,
    },
    {
      name="dynamic_import",
      body={ type="sequence", elements={
        { type="literal", value="import" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=490,
    },
    {
      name="import_meta",
      body={ type="sequence", elements={
        { type="literal", value="import" },
        { type="rule_reference", name="DOT", is_token=true },
        { type="literal", value="meta" },
      } },
      line_number=492,
    },
    {
      name="array_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=494,
    },
    {
      name="element_list",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="rule_reference", name="spread_element", is_token=false },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="optional", element={ type="alternation", choices={
                { type="rule_reference", name="spread_element", is_token=false },
                { type="rule_reference", name="assignment_expression", is_token=false },
              } } },
          } } },
      } },
      line_number=496,
    },
    {
      name="object_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="property_definition", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="property_definition", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=499,
    },
    {
      name="property_definition",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="method_definition", is_token=false },
        { type="rule_reference", name="async_method", is_token=false },
        { type="rule_reference", name="object_spread_property", is_token=false },
      } },
      line_number=501,
    },
    {
      name="object_spread_property",
      body={ type="sequence", elements={
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=507,
    },
    {
      name="property_name",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LBRACKET", is_token=true },
          { type="rule_reference", name="assignment_expression", is_token=false },
          { type="rule_reference", name="RBRACKET", is_token=true },
        } },
      } },
      line_number=509,
    },
    {
      name="function_expression",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=514,
    },
    {
      name="method_definition",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="get" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="set" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="typed_parameter", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="property_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=518,
    },
    {
      name="template_literal",
      body={ type="alternation", choices={
        { type="rule_reference", name="TEMPLATE_NO_SUB", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="TEMPLATE_HEAD", is_token=true },
          { type="repetition", element={ type="rule_reference", name="template_span", is_token=false } },
          { type="rule_reference", name="TEMPLATE_TAIL", is_token=true },
        } },
      } },
      line_number=527,
    },
    {
      name="template_span",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="TEMPLATE_MIDDLE", is_token=true },
      } },
      line_number=530,
    },
    {
      name="type_annotation",
      body={ type="sequence", elements={
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=536,
    },
    {
      name="type_expression",
      body={ type="rule_reference", name="conditional_type", is_token=false },
      line_number=538,
    },
    {
      name="conditional_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="union_type", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="union_type", is_token=false },
          { type="literal", value="extends" },
          { type="rule_reference", name="type_expression", is_token=false },
          { type="rule_reference", name="QUESTION", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
      } },
      line_number=540,
    },
    {
      name="union_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="intersection_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="PIPE", is_token=true },
            { type="rule_reference", name="intersection_type", is_token=false },
          } } },
      } },
      line_number=543,
    },
    {
      name="intersection_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="array_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="AMPERSAND", is_token=true },
            { type="rule_reference", name="array_type", is_token=false },
          } } },
      } },
      line_number=545,
    },
    {
      name="array_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="primary_type", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="LBRACKET", is_token=true },
            { type="rule_reference", name="RBRACKET", is_token=true },
          } } },
      } },
      line_number=547,
    },
    {
      name="primary_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="predefined_type", is_token=false },
        { type="rule_reference", name="type_reference", is_token=false },
        { type="rule_reference", name="literal_type", is_token=false },
        { type="rule_reference", name="object_type", is_token=false },
        { type="rule_reference", name="tuple_type", is_token=false },
        { type="rule_reference", name="function_type", is_token=false },
        { type="rule_reference", name="constructor_type", is_token=false },
        { type="rule_reference", name="mapped_type", is_token=false },
        { type="sequence", elements={
          { type="literal", value="typeof" },
          { type="rule_reference", name="left_hand_side_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="keyof" },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="unique" },
          { type="literal", value="symbol" },
        } },
        { type="sequence", elements={
          { type="literal", value="infer" },
          { type="rule_reference", name="NAME", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="extends" },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="readonly" },
          { type="rule_reference", name="array_type", is_token=false },
        } },
        { type="rule_reference", name="template_literal_type", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=553,
    },
    {
      name="predefined_type",
      body={ type="alternation", choices={
        { type="literal", value="any" },
        { type="literal", value="string" },
        { type="literal", value="number" },
        { type="literal", value="boolean" },
        { type="literal", value="void" },
        { type="literal", value="never" },
        { type="literal", value="object" },
        { type="literal", value="symbol" },
        { type="literal", value="bigint" },
        { type="literal", value="undefined" },
        { type="literal", value="null" },
        { type="literal", value="unknown" },
      } },
      line_number=569,
    },
    {
      name="literal_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="BIGINT", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
      } },
      line_number=572,
    },
    {
      name="type_reference",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="optional", element={ type="rule_reference", name="type_arguments", is_token=false } },
      } },
      line_number=574,
    },
    {
      name="type_arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_argument_list", is_token=false },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=575,
    },
    {
      name="type_argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=576,
    },
    {
      name="type_parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_parameter_list", is_token=false },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=580,
    },
    {
      name="type_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_parameter", is_token=false },
          } } },
      } },
      line_number=581,
    },
    {
      name="type_parameter",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="in" },
            { type="literal", value="out" },
            { type="sequence", elements={
              { type="literal", value="in" },
              { type="literal", value="out" },
            } },
            { type="literal", value="const" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="extends" },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=582,
    },
    {
      name="object_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="type_member", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=589,
    },
    {
      name="type_member",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="construct_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="call_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="index_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="method_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="property_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
      } },
      line_number=591,
    },
    {
      name="property_signature",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="readonly" } },
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=597,
    },
    {
      name="index_signature",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=599,
    },
    {
      name="method_signature",
      body={ type="sequence", elements={
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=601,
    },
    {
      name="call_signature",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=604,
    },
    {
      name="construct_signature",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="abstract" },
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="ARROW", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
        } },
      } },
      line_number=606,
    },
    {
      name="tuple_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="tuple_element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=613,
    },
    {
      name="tuple_element_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="tuple_element", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="tuple_element", is_token=false },
          } } },
      } },
      line_number=614,
    },
    {
      name="tuple_element",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="COLON", is_token=true },
          } } },
        { type="optional", element={ type="literal", value="readonly" } },
        { type="optional", element={ type="rule_reference", name="ELLIPSIS", is_token=true } },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
      } },
      line_number=615,
    },
    {
      name="function_type",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=621,
    },
    {
      name="constructor_type",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="abstract" },
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="ARROW", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="ARROW", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
      } },
      line_number=623,
    },
    {
      name="mapped_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="readonly_modifier", is_token=false } },
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="in" },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="as" },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="question_modifier", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=630,
    },
    {
      name="readonly_modifier",
      body={ type="alternation", choices={
        { type="literal", value="readonly" },
        { type="sequence", elements={
          { type="rule_reference", name="PLUS", is_token=true },
          { type="literal", value="readonly" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS", is_token=true },
          { type="literal", value="readonly" },
        } },
      } },
      line_number=634,
    },
    {
      name="question_modifier",
      body={ type="alternation", choices={
        { type="rule_reference", name="QUESTION", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="PLUS", is_token=true },
          { type="rule_reference", name="QUESTION", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="MINUS", is_token=true },
          { type="rule_reference", name="QUESTION", is_token=true },
        } },
      } },
      line_number=635,
    },
    {
      name="template_literal_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="TEMPLATE_NO_SUB", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="TEMPLATE_HEAD", is_token=true },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="type_expression", is_token=false },
              { type="rule_reference", name="TEMPLATE_MIDDLE", is_token=true },
            } } },
          { type="rule_reference", name="type_expression", is_token=false },
          { type="rule_reference", name="TEMPLATE_TAIL", is_token=true },
        } },
      } },
      line_number=641,
    },
    {
      name="typed_parameter_list",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="typed_parameter", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="typed_parameter", is_token=false },
            } } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="rest_typed_parameter", is_token=false },
            } } },
        } },
        { type="rule_reference", name="rest_typed_parameter", is_token=false },
      } },
      line_number=648,
    },
    {
      name="typed_parameter",
      body={ type="sequence", elements={
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="public" },
            { type="literal", value="private" },
            { type="literal", value="protected" },
          } } },
        { type="optional", element={ type="literal", value="override" } },
        { type="optional", element={ type="literal", value="readonly" } },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="binding_pattern", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=651,
    },
    {
      name="rest_typed_parameter",
      body={ type="sequence", elements={
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
      } },
      line_number=655,
    },
    {
      name="interface_declaration",
      body={ type="sequence", elements={
        { type="literal", value="interface" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="rule_reference", name="interface_heritage", is_token=false } },
        { type="rule_reference", name="object_type", is_token=false },
      } },
      line_number=661,
    },
    {
      name="interface_heritage",
      body={ type="sequence", elements={
        { type="literal", value="extends" },
        { type="rule_reference", name="type_reference", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="type_reference", is_token=false },
          } } },
      } },
      line_number=663,
    },
    {
      name="type_alias_declaration",
      body={ type="sequence", elements={
        { type="literal", value="type" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=665,
    },
    {
      name="enum_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="const" } },
        { type="literal", value="enum" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="rule_reference", name="enum_body", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=667,
    },
    {
      name="enum_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="enum_member", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="enum_member", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
      } },
      line_number=669,
    },
    {
      name="enum_member",
      body={ type="sequence", elements={
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=671,
    },
    {
      name="namespace_declaration",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="literal", value="namespace" },
            { type="literal", value="module" },
          } } },
        { type="rule_reference", name="qualified_name", is_token=false },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="namespace_element", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=673,
    },
    {
      name="qualified_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="DOT", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=675,
    },
    {
      name="namespace_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="type_alias_declaration", is_token=false },
        { type="rule_reference", name="ts_class_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="generator_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="lexical_declaration", is_token=false },
        { type="rule_reference", name="variable_statement", is_token=false },
        { type="rule_reference", name="export_assignment", is_token=false },
        { type="rule_reference", name="export_namespace_element", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=677,
    },
    {
      name="export_assignment",
      body={ type="sequence", elements={
        { type="literal", value="export" },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=682,
    },
    {
      name="export_namespace_element",
      body={ type="sequence", elements={
        { type="literal", value="export" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="namespace_declaration", is_token=false },
            { type="rule_reference", name="interface_declaration", is_token=false },
            { type="rule_reference", name="type_alias_declaration", is_token=false },
            { type="rule_reference", name="ts_class_declaration", is_token=false },
            { type="rule_reference", name="function_declaration", is_token=false },
            { type="rule_reference", name="enum_declaration", is_token=false },
            { type="rule_reference", name="lexical_declaration", is_token=false },
            { type="rule_reference", name="variable_statement", is_token=false },
          } } },
      } },
      line_number=684,
    },
    {
      name="ambient_declaration",
      body={ type="sequence", elements={
        { type="literal", value="declare" },
        { type="rule_reference", name="ambient_declaration_body", is_token=false },
      } },
      line_number=688,
    },
    {
      name="ambient_declaration_body",
      body={ type="alternation", choices={
        { type="rule_reference", name="variable_statement", is_token=false },
        { type="rule_reference", name="ambient_function_declaration", is_token=false },
        { type="rule_reference", name="ts_class_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="type_alias_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="ambient_module_declaration", is_token=false },
        { type="sequence", elements={
          { type="literal", value="global" },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="repetition", element={ type="rule_reference", name="namespace_element", is_token=false } },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=690,
    },
    {
      name="ambient_module_declaration",
      body={ type="sequence", elements={
        { type="literal", value="module" },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="namespace_element", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=695,
    },
    {
      name="ambient_function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=699,
    },
    {
      name="ts_class_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="decorator", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ts_class_modifiers", is_token=false } },
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ts_class_heritage", is_token=false } },
        { type="rule_reference", name="ts_class_body", is_token=false },
      } },
      line_number=719,
    },
    {
      name="ts_class_expression",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="decorator", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ts_class_modifiers", is_token=false } },
        { type="literal", value="class" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ts_class_heritage", is_token=false } },
        { type="rule_reference", name="ts_class_body", is_token=false },
      } },
      line_number=722,
    },
    {
      name="ts_class_modifiers",
      body={ type="alternation", choices={
        { type="literal", value="abstract" },
        { type="literal", value="declare" },
      } },
      line_number=725,
    },
    {
      name="ts_class_heritage",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="extends" },
          { type="rule_reference", name="type_reference", is_token=false },
          { type="optional", element={ type="sequence", elements={
              { type="literal", value="implements" },
              { type="rule_reference", name="type_reference", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="rule_reference", name="COMMA", is_token=true },
                  { type="rule_reference", name="type_reference", is_token=false },
                } } },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="implements" },
          { type="rule_reference", name="type_reference", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="type_reference", is_token=false },
            } } },
        } },
      } },
      line_number=727,
    },
    {
      name="ts_class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="ts_class_element", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=730,
    },
    {
      name="ts_class_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="ts_class_member", is_token=false },
        { type="rule_reference", name="static_block", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=732,
    },
    {
      name="ts_class_member",
      body={ type="alternation", choices={
        { type="rule_reference", name="ts_constructor_declaration", is_token=false },
        { type="rule_reference", name="ts_method_declaration", is_token=false },
        { type="rule_reference", name="ts_property_declaration", is_token=false },
        { type="rule_reference", name="ts_private_field_declaration", is_token=false },
        { type="rule_reference", name="ts_private_method_declaration", is_token=false },
        { type="rule_reference", name="ts_accessor_declaration", is_token=false },
        { type="rule_reference", name="index_signature", is_token=false },
      } },
      line_number=736,
    },
    {
      name="static_block",
      body={ type="sequence", elements={
        { type="literal", value="static" },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=745,
    },
    {
      name="ts_private_field_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="static" } },
        { type="rule_reference", name="PRIVATE_NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=748,
    },
    {
      name="ts_private_method_declaration",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="static" } },
          { type="rule_reference", name="PRIVATE_NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="static" } },
          { type="literal", value="get" },
          { type="rule_reference", name="PRIVATE_NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="static" } },
          { type="literal", value="set" },
          { type="rule_reference", name="PRIVATE_NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="formal_parameter", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="static" } },
          { type="literal", value="async" },
          { type="optional", element={ type="rule_reference", name="STAR", is_token=true } },
          { type="rule_reference", name="PRIVATE_NAME", is_token=true },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=753,
    },
    {
      name="ts_constructor_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="accessibility_modifier", is_token=false } },
        { type="literal", value="constructor" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="ts_constructor_params", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="literal", value="void" },
          } } },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=768,
    },
    {
      name="ts_constructor_params",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="ts_constructor_param", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="ts_constructor_param", is_token=false },
            } } },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COMMA", is_token=true },
              { type="rule_reference", name="rest_typed_parameter", is_token=false },
            } } },
        } },
        { type="rule_reference", name="rest_typed_parameter", is_token=false },
      } },
      line_number=772,
    },
    {
      name="ts_constructor_param",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="accessibility_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="override" } },
        { type="optional", element={ type="literal", value="readonly" } },
        { type="rule_reference", name="typed_parameter", is_token=false },
      } },
      line_number=775,
    },
    {
      name="accessibility_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="private" },
        { type="literal", value="protected" },
      } },
      line_number=777,
    },
    {
      name="ts_method_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="decorator", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="ts_member_modifier", is_token=false } },
        { type="rule_reference", name="ts_method_body", is_token=false },
      } },
      line_number=779,
    },
    {
      name="ts_member_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="private" },
        { type="literal", value="protected" },
        { type="literal", value="static" },
        { type="literal", value="abstract" },
        { type="literal", value="readonly" },
        { type="literal", value="override" },
        { type="literal", value="declare" },
      } },
      line_number=781,
    },
    {
      name="ts_method_body",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="get" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="set" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="typed_parameter", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="property_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="async" },
          { type="optional", element={ type="rule_reference", name="STAR", is_token=true } },
          { type="rule_reference", name="property_name", is_token=false },
          { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
          { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="optional", element={ type="sequence", elements={
              { type="rule_reference", name="COLON", is_token=true },
              { type="rule_reference", name="type_expression", is_token=false },
            } } },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=784,
    },
    {
      name="ts_property_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="decorator", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="ts_member_modifier", is_token=false } },
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=798,
    },
    {
      name="ts_accessor_declaration",
      body={ type="sequence", elements={
        { type="repetition", element={ type="rule_reference", name="decorator", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="ts_member_modifier", is_token=false } },
        { type="literal", value="accessor" },
        { type="rule_reference", name="property_name", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=804,
    },
    {
      name="decorator",
      body={ type="sequence", elements={
        { type="rule_reference", name="AT", is_token=true },
        { type="rule_reference", name="decorator_expression", is_token=false },
      } },
      line_number=818,
    },
    {
      name="decorator_expression",
      body={ type="rule_reference", name="left_hand_side_expression", is_token=false },
      line_number=820,
    },
    {
      name="ts_as_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="conditional_expression", is_token=false },
        { type="literal", value="as" },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=826,
    },
    {
      name="ts_satisfies_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="conditional_expression", is_token=false },
        { type="literal", value="satisfies" },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=828,
    },
    {
      name="type_predicate",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="asserts" } },
          { type="rule_reference", name="NAME", is_token=true },
          { type="literal", value="is" },
          { type="rule_reference", name="type_expression", is_token=false },
        } },
        { type="sequence", elements={
          { type="literal", value="asserts" },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=830,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
