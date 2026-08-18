-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: es2016.grammar
-- Regenerate with: grammar-tools compile-grammar es2016.grammar
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
      line_number=34,
    },
    {
      name="source_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="import_declaration", is_token=false },
        { type="rule_reference", name="export_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="generator_declaration", is_token=false },
        { type="rule_reference", name="class_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=36,
    },
    {
      name="function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=49,
    },
    {
      name="formal_parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="formal_parameter", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="formal_parameter", is_token=false },
          } } },
      } },
      line_number=52,
    },
    {
      name="formal_parameter",
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
          { type="group", element={ type="alternation", choices={
              { type="rule_reference", name="NAME", is_token=true },
              { type="rule_reference", name="binding_pattern", is_token=false },
            } } },
        } },
      } },
      line_number=54,
    },
    {
      name="function_body",
      body={ type="repetition", element={ type="rule_reference", name="source_element", is_token=false } },
      line_number=57,
    },
    {
      name="generator_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=64,
    },
    {
      name="generator_expression",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="STAR", is_token=true },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=67,
    },
    {
      name="yield_expression",
      body={ type="sequence", elements={
        { type="literal", value="yield" },
        { type="optional", element={ type="rule_reference", name="STAR", is_token=true } },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=70,
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
      line_number=77,
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
      line_number=79,
    },
    {
      name="lexical_binding",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="binding_pattern", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=81,
    },
    {
      name="class_declaration",
      body={ type="sequence", elements={
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="class_heritage", is_token=false } },
        { type="rule_reference", name="class_body", is_token=false },
      } },
      line_number=89,
    },
    {
      name="class_expression",
      body={ type="sequence", elements={
        { type="literal", value="class" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="optional", element={ type="rule_reference", name="class_heritage", is_token=false } },
        { type="rule_reference", name="class_body", is_token=false },
      } },
      line_number=91,
    },
    {
      name="class_heritage",
      body={ type="sequence", elements={
        { type="literal", value="extends" },
        { type="rule_reference", name="left_hand_side_expression", is_token=false },
      } },
      line_number=93,
    },
    {
      name="class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="class_element", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=95,
    },
    {
      name="class_element",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="optional", element={ type="literal", value="static" } },
          { type="rule_reference", name="method_definition", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=97,
    },
    {
      name="method_definition",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="get" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="set" },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="formal_parameter", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="STAR", is_token=true },
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
          { type="rule_reference", name="LBRACE", is_token=true },
          { type="rule_reference", name="function_body", is_token=false },
          { type="rule_reference", name="RBRACE", is_token=true },
        } },
      } },
      line_number=100,
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
      } },
      line_number=114,
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
      line_number=117,
    },
    {
      name="default_import",
      body={ type="rule_reference", name="NAME", is_token=true },
      line_number=122,
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
      line_number=124,
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
      line_number=126,
    },
    {
      name="namespace_import",
      body={ type="sequence", elements={
        { type="rule_reference", name="STAR", is_token=true },
        { type="literal", value="as" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=128,
    },
    {
      name="from_clause",
      body={ type="sequence", elements={
        { type="literal", value="from" },
        { type="rule_reference", name="STRING", is_token=true },
      } },
      line_number=130,
    },
    {
      name="module_specifier",
      body={ type="rule_reference", name="STRING", is_token=true },
      line_number=132,
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
              { type="rule_reference", name="class_declaration", is_token=false },
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
              { type="rule_reference", name="class_declaration", is_token=false },
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
      } },
      line_number=134,
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
      line_number=143,
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
      line_number=145,
    },
    {
      name="binding_pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="object_binding_pattern", is_token=false },
        { type="rule_reference", name="array_binding_pattern", is_token=false },
      } },
      line_number=158,
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
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=161,
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
      line_number=163,
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
      line_number=165,
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
      line_number=168,
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
        { type="rule_reference", name="expression_statement", is_token=false },
      } },
      line_number=175,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=196,
    },
    {
      name="variable_statement",
      body={ type="sequence", elements={
        { type="literal", value="var" },
        { type="rule_reference", name="variable_declaration_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=198,
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
      line_number=200,
    },
    {
      name="variable_declaration",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="rule_reference", name="binding_pattern", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=202,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=204,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=206,
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
      line_number=208,
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
      line_number=210,
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
      line_number=212,
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
      line_number=214,
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
      line_number=223,
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
            { type="rule_reference", name="left_hand_side_expression", is_token=false },
          } } },
        { type="literal", value="of" },
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=235,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=242,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=244,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=246,
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
      line_number=248,
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
      line_number=250,
    },
    {
      name="case_clause",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=253,
    },
    {
      name="default_clause",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=255,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=257,
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
      line_number=259,
    },
    {
      name="catch_clause",
      body={ type="sequence", elements={
        { type="literal", value="catch" },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=261,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=263,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=265,
    },
    {
      name="debugger_statement",
      body={ type="sequence", elements={
        { type="literal", value="debugger" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=267,
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
      line_number=281,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="arrow_function", is_token=false },
        { type="rule_reference", name="yield_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="left_hand_side_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
        { type="rule_reference", name="conditional_expression", is_token=false },
      } },
      line_number=283,
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
      } },
      line_number=290,
    },
    {
      name="conditional_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="logical_or_expression", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="QUESTION", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=296,
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
      line_number=299,
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
      line_number=301,
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
      line_number=303,
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
      line_number=305,
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
      line_number=307,
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
      line_number=309,
    },
    {
      name="relational_expression",
      body={ type="sequence", elements={
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
      line_number=313,
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
      line_number=317,
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
      line_number=320,
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
      line_number=328,
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
      line_number=346,
    },
    {
      name="unary_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="postfix_expression", is_token=false },
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
      line_number=348,
    },
    {
      name="postfix_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="left_hand_side_expression", is_token=false },
        { type="optional", element={ type="alternation", choices={
            { type="rule_reference", name="PLUS_PLUS", is_token=true },
            { type="rule_reference", name="MINUS_MINUS", is_token=true },
          } } },
      } },
      line_number=359,
    },
    {
      name="left_hand_side_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="call_expression", is_token=false },
        { type="rule_reference", name="new_expression", is_token=false },
      } },
      line_number=361,
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
              { type="rule_reference", name="LBRACKET", is_token=true },
              { type="rule_reference", name="expression", is_token=false },
              { type="rule_reference", name="RBRACKET", is_token=true },
            } },
            { type="rule_reference", name="template_literal", is_token=false },
          } } },
      } },
      line_number=363,
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
      line_number=367,
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
      line_number=370,
    },
    {
      name="arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=377,
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
      line_number=379,
    },
    {
      name="spread_element",
      body={ type="sequence", elements={
        { type="rule_reference", name="ELLIPSIS", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=382,
    },
    {
      name="arrow_function",
      body={ type="sequence", elements={
        { type="rule_reference", name="arrow_parameters", is_token=false },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="concise_body", is_token=false },
      } },
      line_number=392,
    },
    {
      name="arrow_parameters",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=394,
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
      line_number=397,
    },
    {
      name="primary_expression",
      body={ type="alternation", choices={
        { type="literal", value="this" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="REGEX", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
        { type="literal", value="null" },
        { type="rule_reference", name="array_literal", is_token=false },
        { type="rule_reference", name="object_literal", is_token=false },
        { type="rule_reference", name="function_expression", is_token=false },
        { type="rule_reference", name="generator_expression", is_token=false },
        { type="rule_reference", name="class_expression", is_token=false },
        { type="rule_reference", name="template_literal", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=402,
    },
    {
      name="array_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=418,
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
      line_number=420,
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
      line_number=430,
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
        { type="sequence", elements={
          { type="rule_reference", name="ELLIPSIS", is_token=true },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
      } },
      line_number=432,
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
      line_number=437,
    },
    {
      name="function_expression",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameters", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=442,
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
      line_number=453,
    },
    {
      name="template_span",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="TEMPLATE_MIDDLE", is_token=true },
      } },
      line_number=456,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
