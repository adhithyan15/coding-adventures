-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: ts1.0.grammar
-- Regenerate with: grammar-tools compile-grammar ts1.0.grammar
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
      line_number=56,
    },
    {
      name="source_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="type_alias_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="ambient_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="ts_class_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=65,
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
      line_number=91,
    },
    {
      name="function_body",
      body={ type="repetition", element={ type="rule_reference", name="source_element", is_token=false } },
      line_number=95,
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
      line_number=113,
    },
    {
      name="typed_parameter",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="accessibility_modifier", is_token=false } },
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
      line_number=116,
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
      line_number=118,
    },
    {
      name="variable_statement",
      body={ type="sequence", elements={
        { type="literal", value="var" },
        { type="rule_reference", name="variable_declaration_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=128,
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
      line_number=130,
    },
    {
      name="variable_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="COLON", is_token=true },
            { type="rule_reference", name="type_expression", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=132,
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
        { type="rule_reference", name="continue_statement", is_token=false },
        { type="rule_reference", name="break_statement", is_token=false },
        { type="rule_reference", name="return_statement", is_token=false },
        { type="rule_reference", name="with_statement", is_token=false },
        { type="rule_reference", name="switch_statement", is_token=false },
        { type="rule_reference", name="labelled_statement", is_token=false },
        { type="rule_reference", name="try_statement", is_token=false },
        { type="rule_reference", name="throw_statement", is_token=false },
        { type="rule_reference", name="debugger_statement", is_token=false },
        { type="rule_reference", name="expression_statement", is_token=false },
      } },
      line_number=141,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=160,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=162,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=164,
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
      line_number=166,
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
      line_number=168,
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
      line_number=170,
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
            { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
          } } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=172,
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
            { type="rule_reference", name="left_hand_side_expression", is_token=false },
          } } },
        { type="literal", value="in" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=178,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=182,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=184,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=186,
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
      line_number=188,
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
      line_number=190,
    },
    {
      name="case_clause",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=193,
    },
    {
      name="default_clause",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=195,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=197,
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
      line_number=199,
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
      line_number=201,
    },
    {
      name="finally_clause",
      body={ type="sequence", elements={
        { type="literal", value="finally" },
        { type="rule_reference", name="block", is_token=false },
      } },
      line_number=203,
    },
    {
      name="throw_statement",
      body={ type="sequence", elements={
        { type="literal", value="throw" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=205,
    },
    {
      name="debugger_statement",
      body={ type="sequence", elements={
        { type="literal", value="debugger" },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=207,
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
      line_number=229,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="ts_as_expression", is_token=false },
        { type="rule_reference", name="ts_angle_bracket_assertion", is_token=false },
        { type="rule_reference", name="conditional_expression", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="left_hand_side_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
      } },
      line_number=241,
    },
    {
      name="assignment_operator",
      body={ type="alternation", choices={
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="PLUS_EQUALS", is_token=true },
        { type="rule_reference", name="MINUS_EQUALS", is_token=true },
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
      line_number=246,
    },
    {
      name="ts_as_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="ts_non_null_expression", is_token=false },
        { type="literal", value="as" },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=255,
    },
    {
      name="ts_non_null_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="left_hand_side_expression", is_token=false },
        { type="rule_reference", name="BANG", is_token=true },
      } },
      line_number=260,
    },
    {
      name="ts_angle_bracket_assertion",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=264,
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
      line_number=266,
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
      line_number=269,
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
      line_number=271,
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
      line_number=273,
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
      line_number=275,
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
      line_number=277,
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
      line_number=279,
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
      line_number=283,
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
      line_number=287,
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
      line_number=290,
    },
    {
      name="multiplicative_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="rule_reference", name="SLASH", is_token=true },
                { type="rule_reference", name="PERCENT", is_token=true },
              } } },
            { type="rule_reference", name="unary_expression", is_token=false },
          } } },
      } },
      line_number=293,
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
      line_number=296,
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
      line_number=307,
    },
    {
      name="left_hand_side_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="call_expression", is_token=false },
        { type="rule_reference", name="new_expression", is_token=false },
      } },
      line_number=309,
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
          } } },
      } },
      line_number=311,
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
      line_number=314,
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
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="new" },
          { type="rule_reference", name="member_expression", is_token=false },
          { type="rule_reference", name="arguments", is_token=false },
        } },
      } },
      line_number=317,
    },
    {
      name="arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="argument_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=320,
    },
    {
      name="argument_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="assignment_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=322,
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
        { type="sequence", elements={
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=324,
    },
    {
      name="array_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=337,
    },
    {
      name="element_list",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="assignment_expression", is_token=false } },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="optional", element={ type="rule_reference", name="assignment_expression", is_token=false } },
          } } },
      } },
      line_number=339,
    },
    {
      name="object_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="property_assignment", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="property_assignment", is_token=false },
              } } },
            { type="optional", element={ type="rule_reference", name="COMMA", is_token=true } },
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=344,
    },
    {
      name="property_assignment",
      body={ type="alternation", choices={
        { type="rule_reference", name="getter_property", is_token=false },
        { type="rule_reference", name="setter_property", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
      } },
      line_number=346,
    },
    {
      name="getter_property",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=351,
    },
    {
      name="setter_property",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=354,
    },
    {
      name="property_name",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=356,
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
      line_number=361,
    },
    {
      name="binding_pattern",
      body={ type="alternation", choices={
        { type="rule_reference", name="object_binding_pattern", is_token=false },
        { type="rule_reference", name="array_binding_pattern", is_token=false },
      } },
      line_number=372,
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
          } } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=374,
    },
    {
      name="binding_property",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="property_name", is_token=false },
          { type="rule_reference", name="COLON", is_token=true },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=376,
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
          } } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=379,
    },
    {
      name="binding_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="binding_pattern", is_token=false },
      } },
      line_number=381,
    },
    {
      name="type_annotation",
      body={ type="sequence", elements={
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=410,
    },
    {
      name="type_expression",
      body={ type="rule_reference", name="union_type", is_token=false },
      line_number=428,
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
      line_number=436,
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
      line_number=444,
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
      line_number=451,
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
          { type="rule_reference", name="LPAREN", is_token=true },
          { type="rule_reference", name="type_expression", is_token=false },
          { type="rule_reference", name="RPAREN", is_token=true },
        } },
      } },
      line_number=456,
    },
    {
      name="predefined_type",
      body={ type="alternation", choices={
        { type="literal", value="any" },
        { type="literal", value="string" },
        { type="literal", value="number" },
        { type="literal", value="boolean" },
        { type="literal", value="void" },
        { type="literal", value="object" },
        { type="literal", value="symbol" },
        { type="literal", value="undefined" },
        { type="literal", value="null" },
      } },
      line_number=484,
    },
    {
      name="literal_type",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="literal", value="true" },
        { type="literal", value="false" },
      } },
      line_number=498,
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
      line_number=510,
    },
    {
      name="type_arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_argument_list", is_token=false },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=520,
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
      line_number=521,
    },
    {
      name="type_parameters",
      body={ type="sequence", elements={
        { type="rule_reference", name="LESS_THAN", is_token=true },
        { type="rule_reference", name="type_parameter_list", is_token=false },
        { type="rule_reference", name="GREATER_THAN", is_token=true },
      } },
      line_number=538,
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
      line_number=539,
    },
    {
      name="type_parameter",
      body={ type="sequence", elements={
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
      line_number=540,
    },
    {
      name="object_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="type_member_semicolon", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=557,
    },
    {
      name="type_member_semicolon",
      body={ type="sequence", elements={
        { type="rule_reference", name="type_member", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=558,
    },
    {
      name="type_member",
      body={ type="alternation", choices={
        { type="rule_reference", name="construct_signature", is_token=false },
        { type="rule_reference", name="call_signature", is_token=false },
        { type="rule_reference", name="index_signature", is_token=false },
        { type="rule_reference", name="method_signature", is_token=false },
        { type="rule_reference", name="property_signature", is_token=false },
      } },
      line_number=559,
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
      line_number=567,
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
      line_number=572,
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
      line_number=576,
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
      line_number=580,
    },
    {
      name="construct_signature",
      body={ type="sequence", elements={
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
      line_number=584,
    },
    {
      name="tuple_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="tuple_element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=598,
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
      line_number=599,
    },
    {
      name="tuple_element",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="ELLIPSIS", is_token=true } },
        { type="rule_reference", name="type_expression", is_token=false },
        { type="optional", element={ type="rule_reference", name="QUESTION", is_token=true } },
      } },
      line_number=600,
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
      line_number=618,
    },
    {
      name="constructor_type",
      body={ type="sequence", elements={
        { type="literal", value="new" },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="typed_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="ARROW", is_token=true },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=622,
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
      line_number=654,
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
      line_number=655,
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
      line_number=671,
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
      line_number=695,
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
      line_number=696,
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
      line_number=697,
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
      line_number=719,
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
      line_number=720,
    },
    {
      name="namespace_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="namespace_declaration", is_token=false },
        { type="rule_reference", name="interface_declaration", is_token=false },
        { type="rule_reference", name="type_alias_declaration", is_token=false },
        { type="rule_reference", name="ts_class_declaration", is_token=false },
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="enum_declaration", is_token=false },
        { type="rule_reference", name="variable_statement", is_token=false },
        { type="rule_reference", name="export_assignment", is_token=false },
        { type="rule_reference", name="export_namespace_element", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=724,
    },
    {
      name="export_assignment",
      body={ type="sequence", elements={
        { type="literal", value="export" },
        { type="rule_reference", name="EQUALS", is_token=true },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=737,
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
            { type="rule_reference", name="variable_statement", is_token=false },
          } } },
      } },
      line_number=740,
    },
    {
      name="ambient_declaration",
      body={ type="sequence", elements={
        { type="literal", value="declare" },
        { type="rule_reference", name="ambient_declaration_body", is_token=false },
      } },
      line_number=769,
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
      } },
      line_number=770,
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
      line_number=783,
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
      line_number=787,
    },
    {
      name="ts_class_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="abstract" } },
        { type="literal", value="class" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="type_parameters", is_token=false } },
        { type="optional", element={ type="rule_reference", name="ts_class_heritage", is_token=false } },
        { type="rule_reference", name="ts_class_body", is_token=false },
      } },
      line_number=829,
    },
    {
      name="ts_class_heritage",
      body={ type="sequence", elements={
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="extends" },
            { type="rule_reference", name="type_reference", is_token=false },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="implements" },
            { type="rule_reference", name="type_reference", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="type_reference", is_token=false },
              } } },
          } } },
      } },
      line_number=834,
    },
    {
      name="ts_class_body",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="ts_class_element", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=836,
    },
    {
      name="ts_class_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="ts_method_definition", is_token=false },
        { type="rule_reference", name="ts_property_declaration", is_token=false },
        { type="sequence", elements={
          { type="rule_reference", name="index_signature", is_token=false },
          { type="rule_reference", name="SEMICOLON", is_token=true },
        } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=838,
    },
    {
      name="ts_method_definition",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="accessibility_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="abstract" } },
        { type="optional", element={ type="literal", value="static" } },
        { type="optional", element={ type="literal", value="readonly" } },
        { type="rule_reference", name="ts_method_definition_body", is_token=false },
      } },
      line_number=844,
    },
    {
      name="accessibility_modifier",
      body={ type="alternation", choices={
        { type="literal", value="public" },
        { type="literal", value="private" },
        { type="literal", value="protected" },
      } },
      line_number=850,
    },
    {
      name="ts_method_definition_body",
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
      } },
      line_number=853,
    },
    {
      name="ts_property_declaration",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="accessibility_modifier", is_token=false } },
        { type="optional", element={ type="literal", value="static" } },
        { type="optional", element={ type="literal", value="abstract" } },
        { type="optional", element={ type="literal", value="readonly" } },
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
      line_number=860,
    },
    {
      name="type_predicate",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="is" },
        { type="rule_reference", name="type_expression", is_token=false },
      } },
      line_number=883,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
