package minisqlite

import (
	"regexp"
	"strconv"
	"strings"
	"unicode"
)

type createTableStatement struct {
	table       string
	columns     []string
	ifNotExists bool
}

type dropTableStatement struct {
	table    string
	ifExists bool
}

type insertStatement struct {
	table   string
	columns []string
	rows    [][]any
}

type updateStatement struct {
	table       string
	assignments map[string]any
	where       string
}

type deleteStatement struct {
	table string
	where string
}

var (
	createRe = regexp.MustCompile(`(?is)^\s*CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*;?\s*$`)
	dropRe   = regexp.MustCompile(`(?is)^\s*DROP\s+TABLE\s+(IF\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*;?\s*$`)
	insertRe = regexp.MustCompile(`(?is)^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\(([^)]*)\))?\s+VALUES\s+(.*?)\s*;?\s*$`)
	deleteRe = regexp.MustCompile(`(?is)^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+(.*?))?\s*;?\s*$`)
)

func firstKeyword(sql string) string {
	trimmed := strings.TrimLeftFunc(sql, unicode.IsSpace)
	end := 0
	for end < len(trimmed) && unicode.IsLetter(rune(trimmed[end])) {
		end++
	}
	return strings.ToUpper(trimmed[:end])
}

func parseCreate(sql string) (*createTableStatement, error) {
	m := createRe.FindStringSubmatch(sql)
	if m == nil {
		return nil, &ProgrammingError{Message: "invalid CREATE TABLE statement"}
	}
	var columns []string
	for _, part := range splitTopLevel(m[3], ",") {
		name := identifierAtStart(strings.TrimSpace(part))
		if name != "" {
			columns = append(columns, name)
		}
	}
	if len(columns) == 0 {
		return nil, &ProgrammingError{Message: "CREATE TABLE requires at least one column"}
	}
	return &createTableStatement{
		table:       m[2],
		columns:     columns,
		ifNotExists: m[1] != "",
	}, nil
}

func parseDrop(sql string) (*dropTableStatement, error) {
	m := dropRe.FindStringSubmatch(sql)
	if m == nil {
		return nil, &ProgrammingError{Message: "invalid DROP TABLE statement"}
	}
	return &dropTableStatement{table: m[2], ifExists: m[1] != ""}, nil
}

func parseInsert(sql string) (*insertStatement, error) {
	m := insertRe.FindStringSubmatch(sql)
	if m == nil {
		return nil, &ProgrammingError{Message: "invalid INSERT statement"}
	}
	var columns []string
	if strings.TrimSpace(m[2]) != "" {
		for _, part := range splitTopLevel(m[2], ",") {
			name, err := normalizeIdentifier(strings.TrimSpace(part))
			if err != nil {
				return nil, err
			}
			columns = append(columns, name)
		}
	}
	rows, err := parseValueRows(m[3])
	if err != nil {
		return nil, err
	}
	return &insertStatement{table: m[1], columns: columns, rows: rows}, nil
}

func parseUpdate(sql string) (*updateStatement, error) {
	trimmed := strings.TrimSpace(sql)
	if strings.HasSuffix(trimmed, ";") {
		trimmed = strings.TrimSpace(strings.TrimSuffix(trimmed, ";"))
	}
	re := regexp.MustCompile(`(?is)^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.*)$`)
	m := re.FindStringSubmatch(trimmed)
	if m == nil {
		return nil, &ProgrammingError{Message: "invalid UPDATE statement"}
	}
	assignSQL, whereSQL := splitTopLevelKeyword(m[2], "WHERE")
	assignments := map[string]any{}
	for _, assignment := range splitTopLevel(assignSQL, ",") {
		parts := splitTopLevel(assignment, "=")
		if len(parts) != 2 {
			return nil, &ProgrammingError{Message: "invalid assignment: " + strings.TrimSpace(assignment)}
		}
		name, err := normalizeIdentifier(strings.TrimSpace(parts[0]))
		if err != nil {
			return nil, err
		}
		value, err := parseLiteral(strings.TrimSpace(parts[1]))
		if err != nil {
			return nil, err
		}
		assignments[name] = value
	}
	if len(assignments) == 0 {
		return nil, &ProgrammingError{Message: "UPDATE requires at least one assignment"}
	}
	return &updateStatement{table: m[1], assignments: assignments, where: whereSQL}, nil
}

func parseDelete(sql string) (*deleteStatement, error) {
	m := deleteRe.FindStringSubmatch(sql)
	if m == nil {
		return nil, &ProgrammingError{Message: "invalid DELETE statement"}
	}
	return &deleteStatement{table: m[1], where: strings.TrimSpace(m[2])}, nil
}

func parseValueRows(sql string) ([][]any, error) {
	rest := strings.TrimSpace(sql)
	var rows [][]any
	for rest != "" {
		if rest[0] != '(' {
			return nil, &ProgrammingError{Message: "INSERT VALUES rows must be parenthesized"}
		}
		end := findMatchingParen(rest, 0)
		if end < 0 {
			return nil, &ProgrammingError{Message: "unterminated INSERT VALUES row"}
		}
		var row []any
		for _, part := range splitTopLevel(rest[1:end], ",") {
			value, err := parseLiteral(strings.TrimSpace(part))
			if err != nil {
				return nil, err
			}
			row = append(row, value)
		}
		rows = append(rows, row)
		rest = strings.TrimSpace(rest[end+1:])
		if strings.HasPrefix(rest, ",") {
			rest = strings.TrimSpace(rest[1:])
		} else if rest != "" {
			return nil, &ProgrammingError{Message: "invalid text after INSERT row"}
		}
	}
	if len(rows) == 0 {
		return nil, &ProgrammingError{Message: "INSERT requires at least one row"}
	}
	return rows, nil
}

func parseLiteral(text string) (any, error) {
	value := strings.TrimSpace(text)
	upper := strings.ToUpper(value)
	switch upper {
	case "NULL":
		return nil, nil
	case "TRUE":
		return true, nil
	case "FALSE":
		return false, nil
	}
	if strings.HasPrefix(value, "'") && strings.HasSuffix(value, "'") {
		return strings.ReplaceAll(value[1:len(value)-1], "''", "'"), nil
	}
	if strings.Contains(value, ".") {
		f, err := strconv.ParseFloat(value, 64)
		if err == nil {
			return f, nil
		}
	} else {
		i, err := strconv.ParseInt(value, 10, 64)
		if err == nil {
			return i, nil
		}
	}
	return nil, &ProgrammingError{Message: "expected literal value, got: " + text}
}

func splitTopLevel(text, delimiter string) []string {
	var parts []string
	start, depth := 0, 0
	var quote byte
	for i := 0; i < len(text); i++ {
		ch := text[i]
		if quote != 0 {
			if ch == quote && i+1 < len(text) && text[i+1] == quote {
				i++
			} else if ch == quote {
				quote = 0
			}
			continue
		}
		if ch == '\'' || ch == '"' {
			quote = ch
			continue
		}
		if ch == '(' {
			depth++
		} else if ch == ')' {
			depth--
		} else if depth == 0 && strings.HasPrefix(text[i:], delimiter) {
			part := strings.TrimSpace(text[start:i])
			if part != "" {
				parts = append(parts, part)
			}
			i += len(delimiter) - 1
			start = i + 1
		}
	}
	part := strings.TrimSpace(text[start:])
	if part != "" {
		parts = append(parts, part)
	}
	return parts
}

func splitTopLevelKeyword(text, keyword string) (string, string) {
	depth := 0
	var quote byte
	upper := strings.ToUpper(text)
	needle := strings.ToUpper(keyword)
	for i := 0; i < len(text); i++ {
		ch := text[i]
		if quote != 0 {
			if ch == quote && i+1 < len(text) && text[i+1] == quote {
				i++
			} else if ch == quote {
				quote = 0
			}
			continue
		}
		if ch == '\'' || ch == '"' {
			quote = ch
			continue
		}
		if ch == '(' {
			depth++
		} else if ch == ')' {
			depth--
		} else if depth == 0 && strings.HasPrefix(upper[i:], needle) && boundary(text, i-1) && boundary(text, i+len(needle)) {
			return strings.TrimSpace(text[:i]), strings.TrimSpace(text[i+len(needle):])
		}
	}
	return strings.TrimSpace(text), ""
}

func findMatchingParen(text string, open int) int {
	depth := 0
	var quote byte
	for i := open; i < len(text); i++ {
		ch := text[i]
		if quote != 0 {
			if ch == quote && i+1 < len(text) && text[i+1] == quote {
				i++
			} else if ch == quote {
				quote = 0
			}
			continue
		}
		if ch == '\'' || ch == '"' {
			quote = ch
			continue
		}
		if ch == '(' {
			depth++
		}
		if ch == ')' {
			depth--
			if depth == 0 {
				return i
			}
		}
	}
	return -1
}

func identifierAtStart(text string) string {
	for i, r := range text {
		if i == 0 {
			if !(unicode.IsLetter(r) || r == '_') {
				return ""
			}
			continue
		}
		if !(unicode.IsLetter(r) || unicode.IsDigit(r) || r == '_') {
			return text[:i]
		}
	}
	return text
}

func normalizeIdentifier(text string) (string, error) {
	if text == "" || identifierAtStart(text) != text {
		return "", &ProgrammingError{Message: "invalid identifier: " + text}
	}
	return text, nil
}

func boundary(text string, index int) bool {
	if index < 0 || index >= len(text) {
		return true
	}
	r := rune(text[index])
	return !(unicode.IsLetter(r) || unicode.IsDigit(r) || r == '_')
}
