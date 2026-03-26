// AUTO-GENERATED FILE - DO NOT EDIT
package sqlparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var SqlGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "program",
			LineNumber: 10,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "statement", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ";"}, grammartools.RuleReference{Name: "statement", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Literal{Value: ";"}}}},
		},
		{
			Name: "statement",
			LineNumber: 12,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "select_stmt", IsToken: false}, grammartools.RuleReference{Name: "insert_stmt", IsToken: false}, grammartools.RuleReference{Name: "update_stmt", IsToken: false}, grammartools.RuleReference{Name: "delete_stmt", IsToken: false}, grammartools.RuleReference{Name: "create_table_stmt", IsToken: false}, grammartools.RuleReference{Name: "drop_table_stmt", IsToken: false}}},
		},
		{
			Name: "select_stmt",
			LineNumber: 17,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "SELECT"}, grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "DISTINCT"}, grammartools.Literal{Value: "ALL"}}}}, grammartools.RuleReference{Name: "select_list", IsToken: false}, grammartools.Literal{Value: "FROM"}, grammartools.RuleReference{Name: "table_ref", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "join_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "where_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "group_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "having_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "order_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "limit_clause", IsToken: false}}}},
		},
		{
			Name: "select_list",
			LineNumber: 22,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "select_item", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "select_item", IsToken: false}}}}}}}},
		},
		{
			Name: "select_item",
			LineNumber: 23,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "AS"}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "table_ref",
			LineNumber: 25,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "table_name", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "AS"}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "table_name",
			LineNumber: 26,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "."}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "join_clause",
			LineNumber: 28,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "join_type", IsToken: false}, grammartools.Literal{Value: "JOIN"}, grammartools.RuleReference{Name: "table_ref", IsToken: false}, grammartools.Literal{Value: "ON"}, grammartools.RuleReference{Name: "expr", IsToken: false}}},
		},
		{
			Name: "join_type",
			LineNumber: 29,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "CROSS"}, grammartools.Literal{Value: "INNER"}, grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "LEFT"}, grammartools.Optional{Element: grammartools.Literal{Value: "OUTER"}}}}}, grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "RIGHT"}, grammartools.Optional{Element: grammartools.Literal{Value: "OUTER"}}}}}, grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "FULL"}, grammartools.Optional{Element: grammartools.Literal{Value: "OUTER"}}}}}}},
		},
		{
			Name: "where_clause",
			LineNumber: 32,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "WHERE"}, grammartools.RuleReference{Name: "expr", IsToken: false}}},
		},
		{
			Name: "group_clause",
			LineNumber: 33,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "GROUP"}, grammartools.Literal{Value: "BY"}, grammartools.RuleReference{Name: "column_ref", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "column_ref", IsToken: false}}}}}},
		},
		{
			Name: "having_clause",
			LineNumber: 34,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "HAVING"}, grammartools.RuleReference{Name: "expr", IsToken: false}}},
		},
		{
			Name: "order_clause",
			LineNumber: 35,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "ORDER"}, grammartools.Literal{Value: "BY"}, grammartools.RuleReference{Name: "order_item", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "order_item", IsToken: false}}}}}},
		},
		{
			Name: "order_item",
			LineNumber: 36,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expr", IsToken: false}, grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "ASC"}, grammartools.Literal{Value: "DESC"}}}}}},
		},
		{
			Name: "limit_clause",
			LineNumber: 37,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "LIMIT"}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "OFFSET"}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}}}}}},
		},
		{
			Name: "insert_stmt",
			LineNumber: 41,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "INSERT"}, grammartools.Literal{Value: "INTO"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}, grammartools.Literal{Value: ")"}}}}, grammartools.Literal{Value: "VALUES"}, grammartools.RuleReference{Name: "row_value", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "row_value", IsToken: false}}}}}},
		},
		{
			Name: "row_value",
			LineNumber: 44,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "expr", IsToken: false}}}}, grammartools.Literal{Value: ")"}}},
		},
		{
			Name: "update_stmt",
			LineNumber: 46,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "UPDATE"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "SET"}, grammartools.RuleReference{Name: "assignment", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "assignment", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "where_clause", IsToken: false}}}},
		},
		{
			Name: "assignment",
			LineNumber: 48,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "="}, grammartools.RuleReference{Name: "expr", IsToken: false}}},
		},
		{
			Name: "delete_stmt",
			LineNumber: 50,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "DELETE"}, grammartools.Literal{Value: "FROM"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "where_clause", IsToken: false}}}},
		},
		{
			Name: "create_table_stmt",
			LineNumber: 54,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "CREATE"}, grammartools.Literal{Value: "TABLE"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "IF"}, grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "EXISTS"}}}}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "col_def", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "col_def", IsToken: false}}}}, grammartools.Literal{Value: ")"}}},
		},
		{
			Name: "col_def",
			LineNumber: 56,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "col_constraint", IsToken: false}}}},
		},
		{
			Name: "col_constraint",
			LineNumber: 57,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "NULL"}}}}, grammartools.Literal{Value: "NULL"}, grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "PRIMARY"}, grammartools.Literal{Value: "KEY"}}}}, grammartools.Literal{Value: "UNIQUE"}, grammartools.Group{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "DEFAULT"}, grammartools.RuleReference{Name: "primary", IsToken: false}}}}}},
		},
		{
			Name: "drop_table_stmt",
			LineNumber: 60,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "DROP"}, grammartools.Literal{Value: "TABLE"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "IF"}, grammartools.Literal{Value: "EXISTS"}}}}, grammartools.RuleReference{Name: "NAME", IsToken: true}}},
		},
		{
			Name: "expr",
			LineNumber: 64,
			Body: grammartools.RuleReference{Name: "or_expr", IsToken: false},
		},
		{
			Name: "or_expr",
			LineNumber: 65,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "and_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "OR"}, grammartools.RuleReference{Name: "and_expr", IsToken: false}}}}}},
		},
		{
			Name: "and_expr",
			LineNumber: 66,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "not_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "AND"}, grammartools.RuleReference{Name: "not_expr", IsToken: false}}}}}},
		},
		{
			Name: "not_expr",
			LineNumber: 67,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "NOT"}, grammartools.RuleReference{Name: "not_expr", IsToken: false}}}, grammartools.RuleReference{Name: "comparison", IsToken: false}}},
		},
		{
			Name: "comparison",
			LineNumber: 68,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "additive", IsToken: false}, grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "cmp_op", IsToken: false}, grammartools.RuleReference{Name: "additive", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "BETWEEN"}, grammartools.RuleReference{Name: "additive", IsToken: false}, grammartools.Literal{Value: "AND"}, grammartools.RuleReference{Name: "additive", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "BETWEEN"}, grammartools.RuleReference{Name: "additive", IsToken: false}, grammartools.Literal{Value: "AND"}, grammartools.RuleReference{Name: "additive", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "IN"}, grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "value_list", IsToken: false}, grammartools.Literal{Value: ")"}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "IN"}, grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "value_list", IsToken: false}, grammartools.Literal{Value: ")"}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "LIKE"}, grammartools.RuleReference{Name: "additive", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "LIKE"}, grammartools.RuleReference{Name: "additive", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "IS"}, grammartools.Literal{Value: "NULL"}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "IS"}, grammartools.Literal{Value: "NOT"}, grammartools.Literal{Value: "NULL"}}}}}}}},
		},
		{
			Name: "cmp_op",
			LineNumber: 78,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "="}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.Literal{Value: "<"}, grammartools.Literal{Value: ">"}, grammartools.Literal{Value: "<="}, grammartools.Literal{Value: ">="}}},
		},
		{
			Name: "additive",
			LineNumber: 79,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "multiplicative", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "+"}, grammartools.Literal{Value: "-"}}}}, grammartools.RuleReference{Name: "multiplicative", IsToken: false}}}}}},
		},
		{
			Name: "multiplicative",
			LineNumber: 80,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "unary", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.Literal{Value: "/"}, grammartools.Literal{Value: "%"}}}}, grammartools.RuleReference{Name: "unary", IsToken: false}}}}}},
		},
		{
			Name: "unary",
			LineNumber: 81,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "-"}, grammartools.RuleReference{Name: "unary", IsToken: false}}}, grammartools.RuleReference{Name: "primary", IsToken: false}}},
		},
		{
			Name: "primary",
			LineNumber: 82,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.Literal{Value: "NULL"}, grammartools.Literal{Value: "TRUE"}, grammartools.Literal{Value: "FALSE"}, grammartools.RuleReference{Name: "function_call", IsToken: false}, grammartools.RuleReference{Name: "column_ref", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "("}, grammartools.RuleReference{Name: "expr", IsToken: false}, grammartools.Literal{Value: ")"}}}}},
		},
		{
			Name: "column_ref",
			LineNumber: 85,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "."}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "function_call",
			LineNumber: 86,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "("}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "value_list", IsToken: false}}}}}, grammartools.Literal{Value: ")"}}},
		},
		{
			Name: "value_list",
			LineNumber: 87,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: ","}, grammartools.RuleReference{Name: "expr", IsToken: false}}}}}},
		},
	},
}
