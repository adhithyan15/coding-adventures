package grammartools

import (
	"fmt"
	"strings"
)

type TokenDefinition struct {
	Name       string
	Pattern    string
	IsRegex    bool
	LineNumber int
}

type TokenGrammar struct {
	Definitions []TokenDefinition
	Keywords    []string
}

func ParseTokenGrammar(source string) (*TokenGrammar, error) {
	grammar := &TokenGrammar{}
	lines := strings.Split(source, "\n")
	inKeywords := false

	for i, rawLine := range lines {
		lineNumber := i + 1
		line := strings.TrimRight(rawLine, " \t\r")
		stripped := strings.TrimSpace(line)

		if stripped == "" || strings.HasPrefix(stripped, "#") {
			continue
		}

		if stripped == "keywords:" || stripped == "keywords :" {
			inKeywords = true
			continue
		}

		if inKeywords {
			if len(line) > 0 && (line[0] == ' ' || line[0] == '\t') {
				if stripped != "" {
					grammar.Keywords = append(grammar.Keywords, stripped)
				}
				continue
			} else {
				inKeywords = false
			}
		}

		eqIndex := strings.Index(line, "=")
		if eqIndex == -1 {
			return nil, fmt.Errorf("Line %d: Expected token definition (NAME = pattern)", lineNumber)
		}

		namePart := strings.TrimSpace(line[:eqIndex])
		patternPart := strings.TrimSpace(line[eqIndex+1:])

		if namePart == "" {
			return nil, fmt.Errorf("Line %d: Missing token name", lineNumber)
		}

		if patternPart == "" {
			return nil, fmt.Errorf("Line %d: Missing pattern after '='", lineNumber)
		}

		if strings.HasPrefix(patternPart, "/") && strings.HasSuffix(patternPart, "/") {
			regexBody := patternPart[1 : len(patternPart)-1]
			grammar.Definitions = append(grammar.Definitions, TokenDefinition{
				Name:       namePart,
				Pattern:    regexBody,
				IsRegex:    true,
				LineNumber: lineNumber,
			})
		} else if strings.HasPrefix(patternPart, "\"") && strings.HasSuffix(patternPart, "\"") {
			regexBody := patternPart[1 : len(patternPart)-1]
			grammar.Definitions = append(grammar.Definitions, TokenDefinition{
				Name:       namePart,
				Pattern:    regexBody,
				IsRegex:    false,
				LineNumber: lineNumber,
			})
		} else {
			return nil, fmt.Errorf("Line %d: Pattern must be /regex/ or \"literal\"", lineNumber)
		}
	}
	return grammar, nil
}
