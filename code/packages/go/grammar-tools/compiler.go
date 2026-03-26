package grammartools

import (
	"fmt"
	"strings"
)

// CompileTokensToGo generates Go source code that instantiates the given TokenGrammar.
func CompileTokensToGo(grammar *TokenGrammar, pkgName, varName string) string {
	var b strings.Builder

	b.WriteString("// AUTO-GENERATED FILE - DO NOT EDIT\n")
	b.WriteString(fmt.Sprintf("package %s\n\n", pkgName))
	b.WriteString("import (\n")
	b.WriteString("\tgrammartools \"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools\"\n")
	b.WriteString(")\n\n")

	b.WriteString(fmt.Sprintf("var %s = &grammartools.TokenGrammar{\n", varName))
	
	b.WriteString(fmt.Sprintf("\tVersion: %d,\n", grammar.Version))
	b.WriteString(fmt.Sprintf("\tCaseInsensitive: %t,\n", grammar.CaseInsensitive))
	b.WriteString(fmt.Sprintf("\tCaseSensitive: %t,\n", grammar.CaseSensitive))
	if grammar.Mode != "" {
		b.WriteString(fmt.Sprintf("\tMode: %q,\n", grammar.Mode))
	}
	if grammar.EscapeMode != "" {
		b.WriteString(fmt.Sprintf("\tEscapeMode: %q,\n", grammar.EscapeMode))
	}

	b.WriteString("\tDefinitions: []grammartools.TokenDefinition{\n")
	for _, def := range grammar.Definitions {
		b.WriteString(fmt.Sprintf("\t\t%s,\n", compileTokenDef(def)))
	}
	b.WriteString("\t},\n")

	if len(grammar.Keywords) > 0 {
		b.WriteString("\tKeywords: []string{")
		for _, kw := range grammar.Keywords {
			b.WriteString(fmt.Sprintf("%q, ", kw))
		}
		b.WriteString("},\n")
	}

	if len(grammar.ReservedKeywords) > 0 {
		b.WriteString("\tReservedKeywords: []string{")
		for _, kw := range grammar.ReservedKeywords {
			b.WriteString(fmt.Sprintf("%q, ", kw))
		}
		b.WriteString("},\n")
	}
	
	if len(grammar.SkipDefinitions) > 0 {
		b.WriteString("\tSkipDefinitions: []grammartools.TokenDefinition{\n")
		for _, def := range grammar.SkipDefinitions {
			b.WriteString(fmt.Sprintf("\t\t%s,\n", compileTokenDef(def)))
		}
		b.WriteString("\t},\n")
	}

	if len(grammar.ErrorDefinitions) > 0 {
		b.WriteString("\tErrorDefinitions: []grammartools.TokenDefinition{\n")
		for _, def := range grammar.ErrorDefinitions {
			b.WriteString(fmt.Sprintf("\t\t%s,\n", compileTokenDef(def)))
		}
		b.WriteString("\t},\n")
	}

	if len(grammar.Groups) > 0 {
		b.WriteString("\tGroups: map[string]*grammartools.PatternGroup{\n")
		for gname, group := range grammar.Groups {
			b.WriteString(fmt.Sprintf("\t\t%q: {\n", gname))
			b.WriteString(fmt.Sprintf("\t\t\tName: %q,\n", group.Name))
			b.WriteString("\t\t\tDefinitions: []grammartools.TokenDefinition{\n")
			for _, def := range group.Definitions {
				b.WriteString(fmt.Sprintf("\t\t\t\t%s,\n", compileTokenDef(def)))
			}
			b.WriteString("\t\t\t},\n")
			b.WriteString("\t\t},\n")
		}
		b.WriteString("\t},\n")
	}

	b.WriteString("}\n")

	return b.String()
}

func compileTokenDef(def TokenDefinition) string {
	return fmt.Sprintf("grammartools.TokenDefinition{Name: %q, Pattern: %q, IsRegex: %t, LineNumber: %d, Alias: %q}",
		def.Name, def.Pattern, def.IsRegex, def.LineNumber, def.Alias)
}

// CompileParserToGo generates Go source code that instantiates the given ParserGrammar.
func CompileParserToGo(grammar *ParserGrammar, pkgName, varName string) string {
	var b strings.Builder

	b.WriteString("// AUTO-GENERATED FILE - DO NOT EDIT\n")
	b.WriteString(fmt.Sprintf("package %s\n\n", pkgName))
	b.WriteString("import (\n")
	b.WriteString("\tgrammartools \"github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools\"\n")
	b.WriteString(")\n\n")

	b.WriteString(fmt.Sprintf("var %s = &grammartools.ParserGrammar{\n", varName))
	b.WriteString(fmt.Sprintf("\tVersion: %d,\n", grammar.Version))

	b.WriteString("\tRules: []grammartools.GrammarRule{\n")
	for _, rule := range grammar.Rules {
		b.WriteString("\t\t{\n")
		b.WriteString(fmt.Sprintf("\t\t\tName: %q,\n", rule.Name))
		b.WriteString(fmt.Sprintf("\t\t\tLineNumber: %d,\n", rule.LineNumber))
		b.WriteString(fmt.Sprintf("\t\t\tBody: %s,\n", compileGrammarElement(rule.Body)))
		b.WriteString("\t\t},\n")
	}
	b.WriteString("\t},\n")
	b.WriteString("}\n")

	return b.String()
}

func compileGrammarElement(el GrammarElement) string {
	switch v := el.(type) {
	case RuleReference:
		return fmt.Sprintf("grammartools.RuleReference{Name: %q, IsToken: %t}", v.Name, v.IsToken)
	case Literal:
		return fmt.Sprintf("grammartools.Literal{Value: %q}", v.Value)
	case Sequence:
		var elems []string
		for _, e := range v.Elements {
			elems = append(elems, compileGrammarElement(e))
		}
		return fmt.Sprintf("grammartools.Sequence{Elements: []grammartools.GrammarElement{%s}}", strings.Join(elems, ", "))
	case Alternation:
		var choices []string
		for _, e := range v.Choices {
			choices = append(choices, compileGrammarElement(e))
		}
		return fmt.Sprintf("grammartools.Alternation{Choices: []grammartools.GrammarElement{%s}}", strings.Join(choices, ", "))
	case Repetition:
		return fmt.Sprintf("grammartools.Repetition{Element: %s}", compileGrammarElement(v.Element))
	case Optional:
		return fmt.Sprintf("grammartools.Optional{Element: %s}", compileGrammarElement(v.Element))
	case Group:
		return fmt.Sprintf("grammartools.Group{Element: %s}", compileGrammarElement(v.Element))
	default:
		panic(fmt.Sprintf("unknown GrammarElement type: %%T", v))
	}
}
