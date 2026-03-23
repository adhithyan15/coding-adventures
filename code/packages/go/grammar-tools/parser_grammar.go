package grammartools

import (
	"fmt"
	"strings"
	"unicode"
)

type GrammarElement interface {
	isElement()
}

type RuleReference struct {
	Name    string
	IsToken bool
}
func (RuleReference) isElement() {}

type Literal struct {
	Value string
}
func (Literal) isElement() {}

type Sequence struct {
	Elements []GrammarElement
}
func (Sequence) isElement() {}

type Alternation struct {
	Choices []GrammarElement
}
func (Alternation) isElement() {}

type Repetition struct {
	Element GrammarElement
}
func (Repetition) isElement() {}

type Optional struct {
	Element GrammarElement
}
func (Optional) isElement() {}

type Group struct {
	Element GrammarElement
}
func (Group) isElement() {}

type GrammarRule struct {
	Name       string
	Body       GrammarElement
	LineNumber int
}

type ParserGrammar struct {
	Rules []GrammarRule
}

type internalToken struct {
	kind  string
	value string
	line  int
}

func tokenizeGrammar(source string) ([]internalToken, error) {
	var tokens []internalToken
	lines := strings.Split(source, "\n")
	for i, rawLine := range lines {
		lineNum := i + 1
		line := strings.TrimRight(rawLine, " \t\r")
		stripped := strings.TrimSpace(line)

		if stripped == "" || strings.HasPrefix(stripped, "#") {
			continue
		}

		j := 0
		for j < len(line) {
			ch := line[j]
			if ch == ' ' || ch == '\t' {
				j++
				continue
			}
			if ch == '#' {
				break
			}
			
			switch ch {
			case '=':
				tokens = append(tokens, internalToken{"EQUALS", "=", lineNum})
				j++
			case ';':
				tokens = append(tokens, internalToken{"SEMI", ";", lineNum})
				j++
			case '|':
				tokens = append(tokens, internalToken{"PIPE", "|", lineNum})
				j++
			case '{':
				tokens = append(tokens, internalToken{"LBRACE", "{", lineNum})
				j++
			case '}':
				tokens = append(tokens, internalToken{"RBRACE", "}", lineNum})
				j++
			case '[':
				tokens = append(tokens, internalToken{"LBRACKET", "[", lineNum})
				j++
			case ']':
				tokens = append(tokens, internalToken{"RBRACKET", "]", lineNum})
				j++
			case '(':
				tokens = append(tokens, internalToken{"LPAREN", "(", lineNum})
				j++
			case ')':
				tokens = append(tokens, internalToken{"RPAREN", ")", lineNum})
				j++
			case '"':
				k := j + 1
				for k < len(line) && line[k] != '"' {
					if line[k] == '\\' {
						k++
					}
					k++
				}
				if k >= len(line) {
					return nil, fmt.Errorf("Line %d: Unterminated string literal", lineNum)
				}
				tokens = append(tokens, internalToken{"STRING", line[j+1 : k], lineNum})
				j = k + 1
			default:
				if unicode.IsLetter(rune(ch)) || ch == '_' {
					k := j
					for k < len(line) && (unicode.IsLetter(rune(line[k])) || unicode.IsDigit(rune(line[k])) || line[k] == '_') {
						k++
					}
					tokens = append(tokens, internalToken{"IDENT", line[j:k], lineNum})
					j = k
				} else {
					return nil, fmt.Errorf("Line %d: Unexpected character %q", lineNum, ch)
				}
			}
		}
	}
	tokens = append(tokens, internalToken{"EOF", "", len(lines)})
	return tokens, nil
}

type parser struct {
	tokens []internalToken
	pos    int
}

func (p *parser) peek() internalToken {
	return p.tokens[p.pos]
}

func (p *parser) advance() internalToken {
	tok := p.tokens[p.pos]
	p.pos++
	return tok
}

func (p *parser) expect(kind string) (internalToken, error) {
	tok := p.advance()
	if tok.kind != kind {
		return tok, fmt.Errorf("Line %d: Expected %s, got %s", tok.line, kind, tok.kind)
	}
	return tok, nil
}

func (p *parser) parse() ([]GrammarRule, error) {
	var rules []GrammarRule
	for p.peek().kind != "EOF" {
		rule, err := p.parseRule()
		if err != nil {
			return nil, err
		}
		rules = append(rules, rule)
	}
	return rules, nil
}

func (p *parser) parseRule() (GrammarRule, error) {
	nameTok, err := p.expect("IDENT")
	if err != nil {
		return GrammarRule{}, err
	}
	if _, err := p.expect("EQUALS"); err != nil {
		return GrammarRule{}, err
	}
	body, err := p.parseBody()
	if err != nil {
		return GrammarRule{}, err
	}
	if _, err := p.expect("SEMI"); err != nil {
		return GrammarRule{}, err
	}
	return GrammarRule{Name: nameTok.value, Body: body, LineNumber: nameTok.line}, nil
}

func (p *parser) parseBody() (GrammarElement, error) {
	first, err := p.parseSequence()
	if err != nil {
		return nil, err
	}
	alternatives := []GrammarElement{first}
	
	for p.peek().kind == "PIPE" {
		p.advance()
		seq, err := p.parseSequence()
		if err != nil {
			return nil, err
		}
		alternatives = append(alternatives, seq)
	}
	if len(alternatives) == 1 {
		return alternatives[0], nil
	}
	return Alternation{Choices: alternatives}, nil
}

func (p *parser) parseSequence() (GrammarElement, error) {
	var elements []GrammarElement
	for {
		kind := p.peek().kind
		if kind == "PIPE" || kind == "SEMI" || kind == "RBRACE" || kind == "RBRACKET" || kind == "RPAREN" || kind == "EOF" {
			break
		}
		elem, err := p.parseElement()
		if err != nil {
			return nil, err
		}
		elements = append(elements, elem)
	}
	if len(elements) == 0 {
		return nil, fmt.Errorf("Line %d: Expected at least one element in sequence", p.peek().line)
	}
	if len(elements) == 1 {
		return elements[0], nil
	}
	return Sequence{Elements: elements}, nil
}

func (p *parser) parseElement() (GrammarElement, error) {
	tok := p.peek()
	
	switch tok.kind {
	case "IDENT":
		p.advance()
		isToken := unicode.IsUpper(rune(tok.value[0]))
		return RuleReference{Name: tok.value, IsToken: isToken}, nil
	case "STRING":
		p.advance()
		return Literal{Value: tok.value}, nil
	case "LBRACE":
		p.advance()
		body, err := p.parseBody()
		if err != nil {
			return nil, err
		}
		if _, err := p.expect("RBRACE"); err != nil {
			return nil, err
		}
		return Repetition{Element: body}, nil
	case "LBRACKET":
		p.advance()
		body, err := p.parseBody()
		if err != nil {
			return nil, err
		}
		if _, err := p.expect("RBRACKET"); err != nil {
			return nil, err
		}
		return Optional{Element: body}, nil
	case "LPAREN":
		p.advance()
		body, err := p.parseBody()
		if err != nil {
			return nil, err
		}
		if _, err := p.expect("RPAREN"); err != nil {
			return nil, err
		}
		return Group{Element: body}, nil
	}
	return nil, fmt.Errorf("Line %d: Unexpected token %s", tok.line, tok.kind)
}

func ParseParserGrammar(source string) (*ParserGrammar, error) {
	tokens, err := tokenizeGrammar(source)
	if err != nil {
		return nil, err
	}
	p := &parser{tokens: tokens, pos: 0}
	rules, err := p.parse()
	if err != nil {
		return nil, err
	}
	return &ParserGrammar{Rules: rules}, nil
}

// ---------------------------------------------------------------------------
// ParserGrammar helper methods
// ---------------------------------------------------------------------------

// RuleNames returns the set of all defined rule names in the grammar.
func (g *ParserGrammar) RuleNames() map[string]bool {
	names := make(map[string]bool)
	for _, rule := range g.Rules {
		names[rule.Name] = true
	}
	return names
}

// RuleReferences returns all lowercase rule names referenced in rule bodies.
// These are the non-token references — names that should correspond to other
// rules in this grammar.
func (g *ParserGrammar) RuleReferences() map[string]bool {
	refs := make(map[string]bool)
	for _, rule := range g.Rules {
		collectRuleRefs(rule.Body, refs)
	}
	return refs
}

// TokenReferences returns all UPPERCASE names referenced in rule bodies.
// These are token references — names that should correspond to tokens defined
// in a .tokens file.
func (g *ParserGrammar) TokenReferences() map[string]bool {
	refs := make(map[string]bool)
	for _, rule := range g.Rules {
		collectTokenRefs(rule.Body, refs)
	}
	return refs
}

// collectRuleRefs walks the grammar element tree and collects lowercase (rule)
// references into the provided set.
func collectRuleRefs(element GrammarElement, refs map[string]bool) {
	switch e := element.(type) {
	case RuleReference:
		if !e.IsToken {
			refs[e.Name] = true
		}
	case Sequence:
		for _, sub := range e.Elements {
			collectRuleRefs(sub, refs)
		}
	case Alternation:
		for _, choice := range e.Choices {
			collectRuleRefs(choice, refs)
		}
	case Repetition:
		collectRuleRefs(e.Element, refs)
	case Optional:
		collectRuleRefs(e.Element, refs)
	case Group:
		collectRuleRefs(e.Element, refs)
	}
}

// collectTokenRefs walks the grammar element tree and collects UPPERCASE
// (token) references into the provided set.
func collectTokenRefs(element GrammarElement, refs map[string]bool) {
	switch e := element.(type) {
	case RuleReference:
		if e.IsToken {
			refs[e.Name] = true
		}
	case Sequence:
		for _, sub := range e.Elements {
			collectTokenRefs(sub, refs)
		}
	case Alternation:
		for _, choice := range e.Choices {
			collectTokenRefs(choice, refs)
		}
	case Repetition:
		collectTokenRefs(e.Element, refs)
	case Optional:
		collectTokenRefs(e.Element, refs)
	case Group:
		collectTokenRefs(e.Element, refs)
	}
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

// ValidateParserGrammar checks a parsed ParserGrammar for common problems.
//
// Validation checks (mirrors Python's validate_parser_grammar):
//   - Duplicate rule names — two rules with the same name
//   - Non-lowercase rule names — convention violation (token names are UPPER_CASE,
//     rule names are lowercase)
//   - Undefined rule references — a lowercase name used in a body but not defined
//   - Undefined token references — an UPPERCASE name not in tokenNames (if provided)
//   - Unreachable rules — defined but never referenced; the start rule is exempt
//
// tokenNames is an optional set of valid token names from a .tokens file. When
// non-nil, UPPERCASE references are checked against it. Synthetic tokens
// (NEWLINE, INDENT, DEDENT, EOF) are always valid.
func ValidateParserGrammar(grammar *ParserGrammar, tokenNames map[string]bool) []string {
	var issues []string

	defined := grammar.RuleNames()
	referencedRules := grammar.RuleReferences()
	referencedTokens := grammar.TokenReferences()

	// --- Duplicate rule names ---
	seen := make(map[string]int) // name -> first line number
	for _, rule := range grammar.Rules {
		if firstLine, exists := seen[rule.Name]; exists {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Duplicate rule name '%s' (first defined on line %d)",
				rule.LineNumber, rule.Name, firstLine,
			))
		} else {
			seen[rule.Name] = rule.LineNumber
		}
	}

	// --- Non-lowercase rule names ---
	for _, rule := range grammar.Rules {
		if rule.Name != strings.ToLower(rule.Name) {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Rule name '%s' should be lowercase",
				rule.LineNumber, rule.Name,
			))
		}
	}

	// --- Undefined rule references ---
	// Sort for deterministic output
	sortedRuleRefs := sortedKeys(referencedRules)
	for _, ref := range sortedRuleRefs {
		if !defined[ref] {
			issues = append(issues, fmt.Sprintf("Undefined rule reference: '%s'", ref))
		}
	}

	// --- Undefined token references ---
	// Synthetic tokens are always valid — the lexer produces these implicitly:
	//   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
	//   INDENT/DEDENT — emitted in indentation mode
	//   EOF — always emitted at end of input
	syntheticTokens := map[string]bool{
		"NEWLINE": true,
		"INDENT":  true,
		"DEDENT":  true,
		"EOF":     true,
	}
	if tokenNames != nil {
		sortedTokRefs := sortedKeys(referencedTokens)
		for _, ref := range sortedTokRefs {
			if !tokenNames[ref] && !syntheticTokens[ref] {
				issues = append(issues, fmt.Sprintf("Undefined token reference: '%s'", ref))
			}
		}
	}

	// --- Unreachable rules ---
	// The first rule is the start symbol and is always reachable.
	if len(grammar.Rules) > 0 {
		startRule := grammar.Rules[0].Name
		for _, rule := range grammar.Rules {
			if rule.Name != startRule && !referencedRules[rule.Name] {
				issues = append(issues, fmt.Sprintf(
					"Line %d: Rule '%s' is defined but never referenced (unreachable)",
					rule.LineNumber, rule.Name,
				))
			}
		}
	}

	return issues
}

// sortedKeys returns the keys of a map[string]bool in sorted order.
// This ensures deterministic output for validation messages.
func sortedKeys(m map[string]bool) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	// Simple insertion sort for small maps (grammar refs are typically small)
	for i := 1; i < len(keys); i++ {
		for j := i; j > 0 && keys[j] < keys[j-1]; j-- {
			keys[j], keys[j-1] = keys[j-1], keys[j]
		}
	}
	return keys
}
