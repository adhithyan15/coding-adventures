-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: es1.grammar
-- Regenerate with: grammar-tools compile-grammar es1.grammar
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
      line_number=40,
    },
    {
      name="source_element",
      body={ type="alternation", choices={
        { type="rule_reference", name="function_declaration", is_token=false },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=42,
    },
    {
      name="function_declaration",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=52,
    },
    {
      name="formal_parameter_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="COMMA", is_token=true },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=55,
    },
    {
      name="function_body",
      body={ type="repetition", element={ type="rule_reference", name="source_element", is_token=false } },
      line_number=57,
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
        { type="rule_reference", name="expression_statement", is_token=false },
      } },
      line_number=70,
    },
    {
      name="block",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=89,
    },
    {
      name="variable_statement",
      body={ type="sequence", elements={
        { type="literal", value="var" },
        { type="rule_reference", name="variable_declaration_list", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=94,
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
      line_number=96,
    },
    {
      name="variable_declaration",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="EQUALS", is_token=true },
            { type="rule_reference", name="assignment_expression", is_token=false },
          } } },
      } },
      line_number=98,
    },
    {
      name="empty_statement",
      body={ type="rule_reference", name="SEMICOLON", is_token=true },
      line_number=102,
    },
    {
      name="expression_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=107,
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
      line_number=112,
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
      line_number=116,
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
      line_number=120,
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
      line_number=124,
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
      line_number=132,
    },
    {
      name="continue_statement",
      body={ type="sequence", elements={
        { type="literal", value="continue" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=138,
    },
    {
      name="break_statement",
      body={ type="sequence", elements={
        { type="literal", value="break" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=140,
    },
    {
      name="return_statement",
      body={ type="sequence", elements={
        { type="literal", value="return" },
        { type="optional", element={ type="rule_reference", name="expression", is_token=false } },
        { type="rule_reference", name="SEMICOLON", is_token=true },
      } },
      line_number=144,
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
      line_number=149,
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
      line_number=153,
    },
    {
      name="case_clause",
      body={ type="sequence", elements={
        { type="literal", value="case" },
        { type="rule_reference", name="expression", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=156,
    },
    {
      name="default_clause",
      body={ type="sequence", elements={
        { type="literal", value="default" },
        { type="rule_reference", name="COLON", is_token=true },
        { type="repetition", element={ type="rule_reference", name="statement", is_token=false } },
      } },
      line_number=158,
    },
    {
      name="labelled_statement",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="statement", is_token=false },
      } },
      line_number=162,
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
      line_number=180,
    },
    {
      name="assignment_expression",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="left_hand_side_expression", is_token=false },
          { type="rule_reference", name="assignment_operator", is_token=false },
          { type="rule_reference", name="assignment_expression", is_token=false },
        } },
        { type="rule_reference", name="conditional_expression", is_token=false },
      } },
      line_number=184,
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
      line_number=187,
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
      line_number=194,
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
      line_number=199,
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
      line_number=201,
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
      line_number=205,
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
      line_number=207,
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
      line_number=209,
    },
    {
      name="equality_expression",
      body={ type="sequence", elements={
        { type="rule_reference", name="relational_expression", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="EQUALS_EQUALS", is_token=true },
                { type="rule_reference", name="NOT_EQUALS", is_token=true },
              } } },
            { type="rule_reference", name="relational_expression", is_token=false },
          } } },
      } },
      line_number=213,
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
                { type="literal", value="in" },
              } } },
            { type="rule_reference", name="shift_expression", is_token=false },
          } } },
      } },
      line_number=219,
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
      line_number=225,
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
      line_number=230,
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
      line_number=235,
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
      line_number=240,
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
      line_number=253,
    },
    {
      name="left_hand_side_expression",
      body={ type="alternation", choices={
        { type="rule_reference", name="call_expression", is_token=false },
        { type="rule_reference", name="new_expression", is_token=false },
      } },
      line_number=258,
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
      line_number=263,
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
      line_number=268,
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
      line_number=273,
    },
    {
      name="arguments",
      body={ type="sequence", elements={
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="rule_reference", name="assignment_expression", is_token=false },
            { type="repetition", element={ type="sequence", elements={
                { type="rule_reference", name="COMMA", is_token=true },
                { type="rule_reference", name="assignment_expression", is_token=false },
              } } },
          } } },
        { type="rule_reference", name="RPAREN", is_token=true },
      } },
      line_number=278,
    },
    {
      name="primary_expression",
      body={ type="alternation", choices={
        { type="literal", value="this" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
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
      line_number=282,
    },
    {
      name="array_literal",
      body={ type="sequence", elements={
        { type="rule_reference", name="LBRACKET", is_token=true },
        { type="optional", element={ type="rule_reference", name="element_list", is_token=false } },
        { type="rule_reference", name="RBRACKET", is_token=true },
      } },
      line_number=296,
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
      line_number=298,
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
      line_number=303,
    },
    {
      name="property_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="property_name", is_token=false },
        { type="rule_reference", name="COLON", is_token=true },
        { type="rule_reference", name="assignment_expression", is_token=false },
      } },
      line_number=305,
    },
    {
      name="property_name",
      body={ type="alternation", choices={
        { type="rule_reference", name="NAME", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=307,
    },
    {
      name="function_expression",
      body={ type="sequence", elements={
        { type="literal", value="function" },
        { type="optional", element={ type="rule_reference", name="NAME", is_token=true } },
        { type="rule_reference", name="LPAREN", is_token=true },
        { type="optional", element={ type="rule_reference", name="formal_parameter_list", is_token=false } },
        { type="rule_reference", name="RPAREN", is_token=true },
        { type="rule_reference", name="LBRACE", is_token=true },
        { type="rule_reference", name="function_body", is_token=false },
        { type="rule_reference", name="RBRACE", is_token=true },
      } },
      line_number=311,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
