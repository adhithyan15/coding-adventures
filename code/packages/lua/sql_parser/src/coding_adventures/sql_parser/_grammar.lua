-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: sql.grammar
-- Regenerate with: grammar-tools compile-grammar sql.grammar
--
-- This file embeds a ParserGrammar as native Lua data structures.
-- Call parser_grammar() instead of reading and parsing the .grammar file.

local gt = require("coding_adventures.grammar_tools")

local function parser_grammar()
  local g = gt.ParserGrammar.new()
  g.rules = {
    {
      name="program",
      body={ type="sequence", elements={
        { type="rule_reference", name="statement", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value=";" },
            { type="rule_reference", name="statement", is_token=false },
          } } },
        { type="optional", element={ type="literal", value=";" } },
      } },
      line_number=10,
    },
    {
      name="statement",
      body={ type="alternation", choices={
        { type="rule_reference", name="query_stmt", is_token=false },
        { type="rule_reference", name="insert_stmt", is_token=false },
        { type="rule_reference", name="replace_stmt", is_token=false },
        { type="rule_reference", name="update_stmt", is_token=false },
        { type="rule_reference", name="delete_stmt", is_token=false },
        { type="rule_reference", name="create_table_stmt", is_token=false },
        { type="rule_reference", name="drop_table_stmt", is_token=false },
        { type="rule_reference", name="alter_table_stmt", is_token=false },
        { type="rule_reference", name="create_index_stmt", is_token=false },
        { type="rule_reference", name="drop_index_stmt", is_token=false },
        { type="rule_reference", name="create_view_stmt", is_token=false },
        { type="rule_reference", name="drop_view_stmt", is_token=false },
        { type="rule_reference", name="create_trigger_stmt", is_token=false },
        { type="rule_reference", name="drop_trigger_stmt", is_token=false },
        { type="rule_reference", name="begin_stmt", is_token=false },
        { type="rule_reference", name="commit_stmt", is_token=false },
        { type="rule_reference", name="rollback_to_stmt", is_token=false },
        { type="rule_reference", name="rollback_stmt", is_token=false },
        { type="rule_reference", name="savepoint_stmt", is_token=false },
        { type="rule_reference", name="release_stmt", is_token=false },
        { type="rule_reference", name="attach_stmt", is_token=false },
        { type="rule_reference", name="detach_stmt", is_token=false },
      } },
      line_number=12,
    },
    {
      name="attach_stmt",
      body={ type="sequence", elements={
        { type="literal", value="ATTACH" },
        { type="optional", element={ type="literal", value="DATABASE" } },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value="AS" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=25,
    },
    {
      name="detach_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DETACH" },
        { type="optional", element={ type="literal", value="DATABASE" } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=26,
    },
    {
      name="query_stmt",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="with_clause", is_token=false } },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="values_stmt", is_token=false },
            { type="rule_reference", name="select_stmt", is_token=false },
          } } },
        { type="repetition", element={ type="rule_reference", name="set_op_clause", is_token=false } },
      } },
      line_number=48,
    },
    {
      name="values_stmt",
      body={ type="sequence", elements={
        { type="literal", value="VALUES" },
        { type="rule_reference", name="row_value", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="row_value", is_token=false },
          } } },
      } },
      line_number=55,
    },
    {
      name="with_clause",
      body={ type="sequence", elements={
        { type="literal", value="WITH" },
        { type="optional", element={ type="literal", value="RECURSIVE" } },
        { type="rule_reference", name="cte_def", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="cte_def", is_token=false },
          } } },
      } },
      line_number=56,
    },
    {
      name="cte_def",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="literal", value="AS" },
        { type="optional", element={ type="sequence", elements={
            { type="optional", element={ type="literal", value="NOT" } },
            { type="literal", value="MATERIALIZED" },
          } } },
        { type="literal", value="(" },
        { type="rule_reference", name="query_stmt", is_token=false },
        { type="literal", value=")" },
      } },
      line_number=61,
    },
    {
      name="set_op_clause",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="literal", value="UNION" },
            { type="literal", value="INTERSECT" },
            { type="literal", value="EXCEPT" },
          } } },
        { type="optional", element={ type="literal", value="ALL" } },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="values_stmt", is_token=false },
            { type="rule_reference", name="select_stmt", is_token=false },
          } } },
      } },
      line_number=68,
    },
    {
      name="select_stmt",
      body={ type="sequence", elements={
        { type="literal", value="SELECT" },
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="DISTINCT" },
            { type="literal", value="ALL" },
          } } },
        { type="rule_reference", name="select_list", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="FROM" },
            { type="rule_reference", name="table_ref", is_token=false },
            { type="repetition", element={ type="rule_reference", name="join_clause", is_token=false } },
          } } },
        { type="optional", element={ type="rule_reference", name="where_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="group_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="having_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="window_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="order_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="limit_clause", is_token=false } },
      } },
      line_number=73,
    },
    {
      name="select_list",
      body={ type="alternation", choices={
        { type="rule_reference", name="STAR", is_token=true },
        { type="sequence", elements={
          { type="rule_reference", name="select_item", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="literal", value="," },
              { type="rule_reference", name="select_item", is_token=false },
            } } },
        } },
      } },
      line_number=78,
    },
    {
      name="select_item",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="optional", element={ type="literal", value="AS" } },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=79,
    },
    {
      name="table_ref",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="(" },
          { type="rule_reference", name="query_stmt", is_token=false },
          { type="literal", value=")" },
          { type="optional", element={ type="sequence", elements={
              { type="optional", element={ type="literal", value="AS" } },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="table_name", is_token=false },
          { type="optional", element={ type="alternation", choices={
              { type="sequence", elements={
                { type="literal", value="AS" },
                { type="rule_reference", name="NAME", is_token=true },
              } },
              { type="rule_reference", name="NAME", is_token=true },
            } } },
          { type="optional", element={ type="rule_reference", name="index_hint", is_token=false } },
        } },
      } },
      line_number=100,
    },
    {
      name="table_name",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="." },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=102,
    },
    {
      name="index_hint",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="INDEXED" },
          { type="literal", value="BY" },
          { type="rule_reference", name="NAME", is_token=true },
        } },
        { type="sequence", elements={
          { type="literal", value="NOT" },
          { type="literal", value="INDEXED" },
        } },
      } },
      line_number=103,
    },
    {
      name="join_clause",
      body={ type="alternation", choices={
        { type="group", element={ type="sequence", elements={
            { type="optional", element={ type="rule_reference", name="join_type", is_token=false } },
            { type="literal", value="JOIN" },
            { type="rule_reference", name="table_ref", is_token=false },
            { type="optional", element={ type="alternation", choices={
                { type="sequence", elements={
                  { type="literal", value="ON" },
                  { type="rule_reference", name="expr", is_token=false },
                } },
                { type="sequence", elements={
                  { type="literal", value="USING" },
                  { type="literal", value="(" },
                  { type="rule_reference", name="NAME", is_token=true },
                  { type="repetition", element={ type="sequence", elements={
                      { type="literal", value="," },
                      { type="rule_reference", name="NAME", is_token=true },
                    } } },
                  { type="literal", value=")" },
                } },
              } } },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="table_ref", is_token=false },
          } } },
      } },
      line_number=111,
    },
    {
      name="join_type",
      body={ type="alternation", choices={
        { type="literal", value="CROSS" },
        { type="literal", value="INNER" },
        { type="literal", value="NATURAL" },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="LEFT" },
            { type="optional", element={ type="literal", value="OUTER" } },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="RIGHT" },
            { type="optional", element={ type="literal", value="OUTER" } },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="FULL" },
            { type="optional", element={ type="literal", value="OUTER" } },
          } } },
      } },
      line_number=113,
    },
    {
      name="where_clause",
      body={ type="sequence", elements={
        { type="literal", value="WHERE" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=117,
    },
    {
      name="group_clause",
      body={ type="sequence", elements={
        { type="literal", value="GROUP" },
        { type="literal", value="BY" },
        { type="rule_reference", name="column_ref", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="column_ref", is_token=false },
          } } },
      } },
      line_number=118,
    },
    {
      name="having_clause",
      body={ type="sequence", elements={
        { type="literal", value="HAVING" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=119,
    },
    {
      name="order_clause",
      body={ type="sequence", elements={
        { type="literal", value="ORDER" },
        { type="literal", value="BY" },
        { type="rule_reference", name="order_item", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="order_item", is_token=false },
          } } },
      } },
      line_number=120,
    },
    {
      name="order_item",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="COLLATE" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="ASC" },
            { type="literal", value="DESC" },
          } } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="NULLS" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=141,
    },
    {
      name="limit_clause",
      body={ type="sequence", elements={
        { type="literal", value="LIMIT" },
        { type="rule_reference", name="signed_number", is_token=false },
        { type="optional", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="OFFSET" },
              { type="rule_reference", name="signed_number", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="," },
              { type="rule_reference", name="signed_number", is_token=false },
            } },
          } } },
      } },
      line_number=143,
    },
    {
      name="signed_number",
      body={ type="sequence", elements={
        { type="optional", element={ type="literal", value="-" } },
        { type="rule_reference", name="NUMBER", is_token=true },
      } },
      line_number=158,
    },
    {
      name="conflict_clause",
      body={ type="sequence", elements={
        { type="literal", value="OR" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="REPLACE" },
            { type="literal", value="IGNORE" },
            { type="literal", value="ABORT" },
            { type="literal", value="FAIL" },
            { type="literal", value="ROLLBACK" },
          } } },
      } },
      line_number=180,
    },
    {
      name="insert_stmt",
      body={ type="sequence", elements={
        { type="literal", value="INSERT" },
        { type="optional", element={ type="rule_reference", name="conflict_clause", is_token=false } },
        { type="literal", value="INTO" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="rule_reference", name="insert_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="upsert_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="returning_clause", is_token=false } },
      } },
      line_number=182,
    },
    {
      name="upsert_clause",
      body={ type="sequence", elements={
        { type="literal", value="ON" },
        { type="literal", value="CONFLICT" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="DO" },
              { type="literal", value="NOTHING" },
            } },
            { type="sequence", elements={
              { type="literal", value="DO" },
              { type="literal", value="UPDATE" },
              { type="literal", value="SET" },
              { type="rule_reference", name="upsert_assignment", is_token=false },
              { type="repetition", element={ type="sequence", elements={
                  { type="literal", value="," },
                  { type="rule_reference", name="upsert_assignment", is_token=false },
                } } },
              { type="optional", element={ type="rule_reference", name="where_clause", is_token=false } },
            } },
          } } },
      } },
      line_number=199,
    },
    {
      name="upsert_assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="=" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=205,
    },
    {
      name="replace_stmt",
      body={ type="sequence", elements={
        { type="literal", value="REPLACE" },
        { type="literal", value="INTO" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="rule_reference", name="insert_body", is_token=false },
        { type="optional", element={ type="rule_reference", name="returning_clause", is_token=false } },
      } },
      line_number=206,
    },
    {
      name="insert_body",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="VALUES" },
          { type="rule_reference", name="row_value", is_token=false },
          { type="repetition", element={ type="sequence", elements={
              { type="literal", value="," },
              { type="rule_reference", name="row_value", is_token=false },
            } } },
        } },
        { type="sequence", elements={
          { type="literal", value="DEFAULT" },
          { type="literal", value="VALUES" },
        } },
        { type="rule_reference", name="query_stmt", is_token=false },
      } },
      line_number=210,
    },
    {
      name="row_value",
      body={ type="sequence", elements={
        { type="literal", value="(" },
        { type="rule_reference", name="expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="expr", is_token=false },
          } } },
        { type="literal", value=")" },
      } },
      line_number=217,
    },
    {
      name="row_value_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="row_value", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="row_value", is_token=false },
          } } },
      } },
      line_number=219,
    },
    {
      name="update_stmt",
      body={ type="sequence", elements={
        { type="literal", value="UPDATE" },
        { type="optional", element={ type="rule_reference", name="conflict_clause", is_token=false } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="SET" },
        { type="rule_reference", name="assignment", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="assignment", is_token=false },
          } } },
        { type="optional", element={ type="rule_reference", name="where_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="returning_clause", is_token=false } },
      } },
      line_number=228,
    },
    {
      name="assignment",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="=" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=230,
    },
    {
      name="delete_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DELETE" },
        { type="literal", value="FROM" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="where_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="returning_clause", is_token=false } },
      } },
      line_number=232,
    },
    {
      name="returning_clause",
      body={ type="sequence", elements={
        { type="literal", value="RETURNING" },
        { type="rule_reference", name="returning_item", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="returning_item", is_token=false },
          } } },
      } },
      line_number=234,
    },
    {
      name="returning_item",
      body={ type="alternation", choices={
        { type="literal", value="*" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=238,
    },
    {
      name="create_table_stmt",
      body={ type="sequence", elements={
        { type="literal", value="CREATE" },
        { type="literal", value="TABLE" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="NOT" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="(" },
        { type="rule_reference", name="col_def", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="col_def", is_token=false },
          } } },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="table_constraint", is_token=false },
          } } },
        { type="literal", value=")" },
        { type="optional", element={ type="rule_reference", name="table_options", is_token=false } },
      } },
      line_number=242,
    },
    {
      name="table_constraint",
      body={ type="alternation", choices={
        { type="group", element={ type="sequence", elements={
            { type="literal", value="PRIMARY" },
            { type="literal", value="KEY" },
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="UNIQUE" },
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="CHECK" },
            { type="literal", value="(" },
            { type="rule_reference", name="expr", is_token=false },
            { type="literal", value=")" },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="FOREIGN" },
            { type="literal", value="KEY" },
            { type="literal", value="(" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NAME", is_token=true },
              } } },
            { type="literal", value=")" },
            { type="literal", value="REFERENCES" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="optional", element={ type="sequence", elements={
                { type="literal", value="(" },
                { type="rule_reference", name="NAME", is_token=true },
                { type="repetition", element={ type="sequence", elements={
                    { type="literal", value="," },
                    { type="rule_reference", name="NAME", is_token=true },
                  } } },
                { type="literal", value=")" },
              } } },
          } } },
      } },
      line_number=250,
    },
    {
      name="table_options",
      body={ type="sequence", elements={
        { type="rule_reference", name="table_option", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="table_option", is_token=false },
          } } },
      } },
      line_number=261,
    },
    {
      name="table_option",
      body={ type="alternation", choices={
        { type="literal", value="STRICT" },
        { type="sequence", elements={
          { type="literal", value="WITHOUT" },
          { type="rule_reference", name="NAME", is_token=true },
        } },
      } },
      line_number=262,
    },
    {
      name="col_def",
      body={ type="sequence", elements={
        { type="negative_lookahead", element={ type="group", element={ type="sequence", elements={
              { type="literal", value="FOREIGN" },
              { type="literal", value="KEY" },
            } } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="rule_reference", name="col_type", is_token=false } },
        { type="repetition", element={ type="rule_reference", name="col_constraint", is_token=false } },
      } },
      line_number=267,
    },
    {
      name="col_type",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="(" },
            { type="rule_reference", name="NUMBER", is_token=true },
            { type="repetition", element={ type="sequence", elements={
                { type="literal", value="," },
                { type="rule_reference", name="NUMBER", is_token=true },
              } } },
            { type="literal", value=")" },
          } } },
      } },
      line_number=279,
    },
    {
      name="col_constraint",
      body={ type="alternation", choices={
        { type="group", element={ type="sequence", elements={
            { type="literal", value="NOT" },
            { type="literal", value="NULL" },
            { type="optional", element={ type="rule_reference", name="col_conflict_clause", is_token=false } },
          } } },
        { type="literal", value="NULL" },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="PRIMARY" },
            { type="literal", value="KEY" },
            { type="optional", element={ type="literal", value="AUTOINCREMENT" } },
            { type="optional", element={ type="rule_reference", name="col_conflict_clause", is_token=false } },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="UNIQUE" },
            { type="optional", element={ type="rule_reference", name="col_conflict_clause", is_token=false } },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="DEFAULT" },
            { type="rule_reference", name="primary", is_token=false },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="CHECK" },
            { type="literal", value="(" },
            { type="rule_reference", name="expr", is_token=false },
            { type="literal", value=")" },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="COLLATE" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="group", element={ type="sequence", elements={
            { type="literal", value="REFERENCES" },
            { type="rule_reference", name="NAME", is_token=true },
            { type="optional", element={ type="sequence", elements={
                { type="literal", value="(" },
                { type="rule_reference", name="NAME", is_token=true },
                { type="literal", value=")" },
              } } },
          } } },
      } },
      line_number=280,
    },
    {
      name="col_conflict_clause",
      body={ type="sequence", elements={
        { type="literal", value="ON" },
        { type="literal", value="CONFLICT" },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="ROLLBACK" },
            { type="literal", value="ABORT" },
            { type="literal", value="FAIL" },
            { type="literal", value="IGNORE" },
            { type="literal", value="REPLACE" },
          } } },
      } },
      line_number=296,
    },
    {
      name="drop_table_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DROP" },
        { type="literal", value="TABLE" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=304,
    },
    {
      name="alter_table_stmt",
      body={ type="sequence", elements={
        { type="literal", value="ALTER" },
        { type="literal", value="TABLE" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="ADD" },
              { type="optional", element={ type="literal", value="COLUMN" } },
              { type="rule_reference", name="col_def", is_token=false },
            } },
            { type="sequence", elements={
              { type="literal", value="RENAME" },
              { type="literal", value="TO" },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="literal", value="RENAME" },
              { type="optional", element={ type="literal", value="COLUMN" } },
              { type="rule_reference", name="NAME", is_token=true },
              { type="literal", value="TO" },
              { type="rule_reference", name="NAME", is_token=true },
            } },
            { type="sequence", elements={
              { type="literal", value="DROP" },
              { type="optional", element={ type="literal", value="COLUMN" } },
              { type="rule_reference", name="NAME", is_token=true },
            } },
          } } },
      } },
      line_number=313,
    },
    {
      name="create_index_stmt",
      body={ type="sequence", elements={
        { type="literal", value="CREATE" },
        { type="optional", element={ type="literal", value="UNIQUE" } },
        { type="literal", value="INDEX" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="NOT" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="ON" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="(" },
        { type="rule_reference", name="index_col", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="index_col", is_token=false },
          } } },
        { type="literal", value=")" },
        { type="optional", element={ type="rule_reference", name="where_clause", is_token=false } },
      } },
      line_number=327,
    },
    {
      name="index_col",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="COLLATE" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
        { type="optional", element={ type="alternation", choices={
            { type="literal", value="ASC" },
            { type="literal", value="DESC" },
          } } },
      } },
      line_number=344,
    },
    {
      name="drop_index_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DROP" },
        { type="literal", value="INDEX" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=346,
    },
    {
      name="create_view_stmt",
      body={ type="sequence", elements={
        { type="literal", value="CREATE" },
        { type="literal", value="VIEW" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="NOT" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="AS" },
        { type="rule_reference", name="query_stmt", is_token=false },
      } },
      line_number=354,
    },
    {
      name="drop_view_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DROP" },
        { type="literal", value="VIEW" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=356,
    },
    {
      name="begin_stmt",
      body={ type="sequence", elements={
        { type="literal", value="BEGIN" },
        { type="optional", element={ type="literal", value="TRANSACTION" } },
      } },
      line_number=362,
    },
    {
      name="commit_stmt",
      body={ type="sequence", elements={
        { type="literal", value="COMMIT" },
        { type="optional", element={ type="literal", value="TRANSACTION" } },
      } },
      line_number=363,
    },
    {
      name="rollback_stmt",
      body={ type="sequence", elements={
        { type="literal", value="ROLLBACK" },
        { type="optional", element={ type="literal", value="TRANSACTION" } },
      } },
      line_number=364,
    },
    {
      name="savepoint_stmt",
      body={ type="sequence", elements={
        { type="literal", value="SAVEPOINT" },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=380,
    },
    {
      name="release_stmt",
      body={ type="sequence", elements={
        { type="literal", value="RELEASE" },
        { type="optional", element={ type="literal", value="SAVEPOINT" } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=381,
    },
    {
      name="rollback_to_stmt",
      body={ type="sequence", elements={
        { type="literal", value="ROLLBACK" },
        { type="literal", value="TO" },
        { type="optional", element={ type="literal", value="SAVEPOINT" } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=382,
    },
    {
      name="expr",
      body={ type="rule_reference", name="or_expr", is_token=false },
      line_number=386,
    },
    {
      name="or_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="and_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="OR" },
            { type="rule_reference", name="and_expr", is_token=false },
          } } },
      } },
      line_number=387,
    },
    {
      name="and_expr",
      body={ type="sequence", elements={
        { type="rule_reference", name="not_expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="AND" },
            { type="rule_reference", name="not_expr", is_token=false },
          } } },
      } },
      line_number=388,
    },
    {
      name="not_expr",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="NOT" },
          { type="rule_reference", name="not_expr", is_token=false },
        } },
        { type="rule_reference", name="comparison", is_token=false },
      } },
      line_number=389,
    },
    {
      name="collated",
      body={ type="sequence", elements={
        { type="rule_reference", name="bitwise", is_token=false },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="COLLATE" },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=402,
    },
    {
      name="comparison",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="row_value", is_token=false },
          { type="rule_reference", name="cmp_op", is_token=false },
          { type="rule_reference", name="row_value", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="row_value", is_token=false },
          { type="literal", value="NOT" },
          { type="literal", value="IN" },
          { type="literal", value="(" },
          { type="rule_reference", name="row_value_list", is_token=false },
          { type="literal", value=")" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="row_value", is_token=false },
          { type="literal", value="IN" },
          { type="literal", value="(" },
          { type="rule_reference", name="row_value_list", is_token=false },
          { type="literal", value=")" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="collated", is_token=false },
          { type="optional", element={ type="alternation", choices={
              { type="sequence", elements={
                { type="rule_reference", name="cmp_op", is_token=false },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="BETWEEN" },
                { type="rule_reference", name="collated", is_token=false },
                { type="literal", value="AND" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="NOT" },
                { type="literal", value="BETWEEN" },
                { type="rule_reference", name="collated", is_token=false },
                { type="literal", value="AND" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="IN" },
                { type="literal", value="(" },
                { type="optional", element={ type="rule_reference", name="in_expr", is_token=false } },
                { type="literal", value=")" },
              } },
              { type="sequence", elements={
                { type="literal", value="NOT" },
                { type="literal", value="IN" },
                { type="literal", value="(" },
                { type="optional", element={ type="rule_reference", name="in_expr", is_token=false } },
                { type="literal", value=")" },
              } },
              { type="sequence", elements={
                { type="literal", value="LIKE" },
                { type="rule_reference", name="collated", is_token=false },
                { type="optional", element={ type="sequence", elements={
                    { type="literal", value="ESCAPE" },
                    { type="rule_reference", name="collated", is_token=false },
                  } } },
              } },
              { type="sequence", elements={
                { type="literal", value="NOT" },
                { type="literal", value="LIKE" },
                { type="rule_reference", name="collated", is_token=false },
                { type="optional", element={ type="sequence", elements={
                    { type="literal", value="ESCAPE" },
                    { type="rule_reference", name="collated", is_token=false },
                  } } },
              } },
              { type="sequence", elements={
                { type="literal", value="GLOB" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="NOT" },
                { type="literal", value="GLOB" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="literal", value="NULL" },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="literal", value="NOT" },
                { type="literal", value="NULL" },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="literal", value="DISTINCT" },
                { type="literal", value="FROM" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="literal", value="NOT" },
                { type="literal", value="DISTINCT" },
                { type="literal", value="FROM" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="literal", value="NOT" },
                { type="rule_reference", name="collated", is_token=false },
              } },
              { type="sequence", elements={
                { type="literal", value="IS" },
                { type="rule_reference", name="collated", is_token=false },
              } },
            } } },
        } },
      } },
      line_number=407,
    },
    {
      name="in_expr",
      body={ type="alternation", choices={
        { type="rule_reference", name="query_stmt", is_token=false },
        { type="rule_reference", name="value_list", is_token=false },
      } },
      line_number=435,
    },
    {
      name="cmp_op",
      body={ type="alternation", choices={
        { type="literal", value="=" },
        { type="rule_reference", name="NOT_EQUALS", is_token=true },
        { type="literal", value="<" },
        { type="literal", value=">" },
        { type="literal", value="<=" },
        { type="literal", value=">=" },
      } },
      line_number=437,
    },
    {
      name="bitwise",
      body={ type="sequence", elements={
        { type="rule_reference", name="additive", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="literal", value="&" },
                { type="literal", value="|" },
                { type="literal", value="<<" },
                { type="literal", value=">>" },
              } } },
            { type="rule_reference", name="additive", is_token=false },
          } } },
      } },
      line_number=451,
    },
    {
      name="additive",
      body={ type="sequence", elements={
        { type="rule_reference", name="multiplicative", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="literal", value="+" },
                { type="literal", value="-" },
                { type="literal", value="||" },
                { type="rule_reference", name="JSON_ARROW", is_token=true },
                { type="rule_reference", name="JSON_ARROW_TEXT", is_token=true },
              } } },
            { type="rule_reference", name="multiplicative", is_token=false },
          } } },
      } },
      line_number=452,
    },
    {
      name="multiplicative",
      body={ type="sequence", elements={
        { type="rule_reference", name="unary", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="group", element={ type="alternation", choices={
                { type="rule_reference", name="STAR", is_token=true },
                { type="literal", value="/" },
                { type="literal", value="%" },
              } } },
            { type="rule_reference", name="unary", is_token=false },
          } } },
      } },
      line_number=453,
    },
    {
      name="unary",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="group", element={ type="alternation", choices={
              { type="literal", value="-" },
              { type="literal", value="~" },
              { type="literal", value="+" },
            } } },
          { type="rule_reference", name="unary", is_token=false },
        } },
        { type="rule_reference", name="primary", is_token=false },
      } },
      line_number=459,
    },
    {
      name="primary",
      body={ type="alternation", choices={
        { type="rule_reference", name="NUMBER", is_token=true },
        { type="rule_reference", name="STRING", is_token=true },
        { type="rule_reference", name="BLOB", is_token=true },
        { type="literal", value="NULL" },
        { type="literal", value="TRUE" },
        { type="literal", value="FALSE" },
        { type="rule_reference", name="case_expr", is_token=false },
        { type="rule_reference", name="cast_expr", is_token=false },
        { type="rule_reference", name="window_func_call", is_token=false },
        { type="rule_reference", name="function_call", is_token=false },
        { type="sequence", elements={
          { type="literal", value="EXISTS" },
          { type="literal", value="(" },
          { type="rule_reference", name="query_stmt", is_token=false },
          { type="literal", value=")" },
        } },
        { type="sequence", elements={
          { type="literal", value="(" },
          { type="rule_reference", name="query_stmt", is_token=false },
          { type="literal", value=")" },
        } },
        { type="rule_reference", name="column_ref", is_token=false },
        { type="sequence", elements={
          { type="literal", value="(" },
          { type="rule_reference", name="expr", is_token=false },
          { type="literal", value=")" },
        } },
      } },
      line_number=479,
    },
    {
      name="column_ref",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="." },
            { type="rule_reference", name="NAME", is_token=true },
          } } },
      } },
      line_number=489,
    },
    {
      name="function_call",
      body={ type="sequence", elements={
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="NAME", is_token=true },
            { type="literal", value="REPLACE" },
          } } },
        { type="literal", value="(" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="STAR", is_token=true },
            { type="sequence", elements={
              { type="literal", value="DISTINCT" },
              { type="rule_reference", name="value_list", is_token=false },
            } },
            { type="optional", element={ type="rule_reference", name="value_list", is_token=false } },
          } } },
        { type="literal", value=")" },
        { type="optional", element={ type="rule_reference", name="filter_clause", is_token=false } },
      } },
      line_number=503,
    },
    {
      name="filter_clause",
      body={ type="sequence", elements={
        { type="literal", value="FILTER" },
        { type="literal", value="(" },
        { type="literal", value="WHERE" },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value=")" },
      } },
      line_number=507,
    },
    {
      name="cast_expr",
      body={ type="sequence", elements={
        { type="literal", value="CAST" },
        { type="literal", value="(" },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value="AS" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value=")" },
      } },
      line_number=513,
    },
    {
      name="window_func_call",
      body={ type="sequence", elements={
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="(" },
        { type="group", element={ type="alternation", choices={
            { type="rule_reference", name="STAR", is_token=true },
            { type="optional", element={ type="rule_reference", name="value_list", is_token=false } },
          } } },
        { type="literal", value=")" },
        { type="literal", value="OVER" },
        { type="group", element={ type="alternation", choices={
            { type="sequence", elements={
              { type="literal", value="(" },
              { type="rule_reference", name="window_spec", is_token=false },
              { type="literal", value=")" },
            } },
            { type="rule_reference", name="window_name_ref", is_token=false },
          } } },
      } },
      line_number=542,
    },
    {
      name="window_name_ref",
      body={ type="rule_reference", name="NAME", is_token=true },
      line_number=543,
    },
    {
      name="window_spec",
      body={ type="sequence", elements={
        { type="optional", element={ type="rule_reference", name="partition_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="order_clause", is_token=false } },
        { type="optional", element={ type="rule_reference", name="frame_clause", is_token=false } },
      } },
      line_number=544,
    },
    {
      name="window_clause",
      body={ type="sequence", elements={
        { type="literal", value="WINDOW" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="literal", value="AS" },
        { type="literal", value="(" },
        { type="rule_reference", name="window_spec", is_token=false },
        { type="literal", value=")" },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="NAME", is_token=true },
            { type="literal", value="AS" },
            { type="literal", value="(" },
            { type="rule_reference", name="window_spec", is_token=false },
            { type="literal", value=")" },
          } } },
      } },
      line_number=545,
    },
    {
      name="partition_clause",
      body={ type="sequence", elements={
        { type="literal", value="PARTITION" },
        { type="literal", value="BY" },
        { type="rule_reference", name="expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="expr", is_token=false },
          } } },
      } },
      line_number=546,
    },
    {
      name="value_list",
      body={ type="sequence", elements={
        { type="rule_reference", name="expr", is_token=false },
        { type="repetition", element={ type="sequence", elements={
            { type="literal", value="," },
            { type="rule_reference", name="expr", is_token=false },
          } } },
      } },
      line_number=547,
    },
    {
      name="frame_clause",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="rule_reference", name="frame_unit", is_token=false },
          { type="literal", value="BETWEEN" },
          { type="rule_reference", name="frame_bound", is_token=false },
          { type="literal", value="AND" },
          { type="rule_reference", name="frame_bound", is_token=false },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="frame_unit", is_token=false },
          { type="rule_reference", name="frame_bound", is_token=false },
        } },
      } },
      line_number=569,
    },
    {
      name="frame_unit",
      body={ type="alternation", choices={
        { type="literal", value="ROWS" },
        { type="literal", value="RANGE" },
        { type="literal", value="GROUPS" },
      } },
      line_number=571,
    },
    {
      name="frame_bound",
      body={ type="alternation", choices={
        { type="sequence", elements={
          { type="literal", value="UNBOUNDED" },
          { type="literal", value="PRECEDING" },
        } },
        { type="sequence", elements={
          { type="literal", value="UNBOUNDED" },
          { type="literal", value="FOLLOWING" },
        } },
        { type="sequence", elements={
          { type="literal", value="CURRENT" },
          { type="literal", value="ROW" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expr", is_token=false },
          { type="literal", value="PRECEDING" },
        } },
        { type="sequence", elements={
          { type="rule_reference", name="expr", is_token=false },
          { type="literal", value="FOLLOWING" },
        } },
      } },
      line_number=572,
    },
    {
      name="create_trigger_stmt",
      body={ type="sequence", elements={
        { type="literal", value="CREATE" },
        { type="literal", value="TRIGGER" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="NOT" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="BEFORE" },
            { type="literal", value="AFTER" },
          } } },
        { type="group", element={ type="alternation", choices={
            { type="literal", value="INSERT" },
            { type="literal", value="UPDATE" },
            { type="literal", value="DELETE" },
          } } },
        { type="literal", value="ON" },
        { type="rule_reference", name="NAME", is_token=true },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="FOR" },
            { type="literal", value="EACH" },
            { type="literal", value="ROW" },
          } } },
        { type="literal", value="BEGIN" },
        { type="rule_reference", name="trigger_body_stmt", is_token=false },
        { type="literal", value=";" },
        { type="repetition", element={ type="sequence", elements={
            { type="rule_reference", name="trigger_body_stmt", is_token=false },
            { type="literal", value=";" },
          } } },
        { type="literal", value="END" },
      } },
      line_number=598,
    },
    {
      name="trigger_body_stmt",
      body={ type="alternation", choices={
        { type="rule_reference", name="insert_stmt", is_token=false },
        { type="rule_reference", name="replace_stmt", is_token=false },
        { type="rule_reference", name="update_stmt", is_token=false },
        { type="rule_reference", name="delete_stmt", is_token=false },
        { type="rule_reference", name="query_stmt", is_token=false },
      } },
      line_number=603,
    },
    {
      name="drop_trigger_stmt",
      body={ type="sequence", elements={
        { type="literal", value="DROP" },
        { type="literal", value="TRIGGER" },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="IF" },
            { type="literal", value="EXISTS" },
          } } },
        { type="rule_reference", name="NAME", is_token=true },
      } },
      line_number=605,
    },
    {
      name="case_expr",
      body={ type="sequence", elements={
        { type="literal", value="CASE" },
        { type="optional", element={ type="rule_reference", name="case_operand", is_token=false } },
        { type="rule_reference", name="case_when", is_token=false },
        { type="repetition", element={ type="rule_reference", name="case_when", is_token=false } },
        { type="optional", element={ type="sequence", elements={
            { type="literal", value="ELSE" },
            { type="rule_reference", name="expr", is_token=false },
          } } },
        { type="literal", value="END" },
      } },
      line_number=620,
    },
    {
      name="case_operand",
      body={ type="rule_reference", name="expr", is_token=false },
      line_number=621,
    },
    {
      name="case_when",
      body={ type="sequence", elements={
        { type="literal", value="WHEN" },
        { type="rule_reference", name="expr", is_token=false },
        { type="literal", value="THEN" },
        { type="rule_reference", name="expr", is_token=false },
      } },
      line_number=622,
    },
  }
  g.version = 0
  return g
end

return { parser_grammar = parser_grammar }
