package minisqlite

import (
	"fmt"
	"math"
	"strconv"
	"strings"
)

func bindParameters(sql string, params []any) (string, error) {
	var out strings.Builder
	index := 0

	for i := 0; i < len(sql); {
		ch := sql[i]
		if ch == '\'' || ch == '"' {
			lit, next := readQuoted(sql, i, ch)
			out.WriteString(lit)
			i = next
			continue
		}
		if ch == '-' && i+1 < len(sql) && sql[i+1] == '-' {
			next := readLineComment(sql, i)
			out.WriteString(sql[i:next])
			i = next
			continue
		}
		if ch == '/' && i+1 < len(sql) && sql[i+1] == '*' {
			next := readBlockComment(sql, i)
			out.WriteString(sql[i:next])
			i = next
			continue
		}
		if ch == '?' {
			if index >= len(params) {
				return "", &ProgrammingError{Message: "not enough parameters for SQL statement"}
			}
			lit, err := toSQLLiteral(params[index])
			if err != nil {
				return "", err
			}
			out.WriteString(lit)
			index++
			i++
			continue
		}
		out.WriteByte(ch)
		i++
	}

	if index != len(params) {
		return "", &ProgrammingError{Message: "too many parameters for SQL statement"}
	}
	return out.String(), nil
}

func readQuoted(sql string, start int, quote byte) (string, int) {
	for i := start + 1; i < len(sql); i++ {
		if sql[i] == quote {
			if i+1 < len(sql) && sql[i+1] == quote {
				i++
				continue
			}
			return sql[start : i+1], i + 1
		}
	}
	return sql[start:], len(sql)
}

func readLineComment(sql string, start int) int {
	i := start + 2
	for i < len(sql) && sql[i] != '\n' {
		i++
	}
	return i
}

func readBlockComment(sql string, start int) int {
	i := start + 2
	for i+1 < len(sql) {
		if sql[i] == '*' && sql[i+1] == '/' {
			return i + 2
		}
		i++
	}
	return len(sql)
}

func toSQLLiteral(value any) (string, error) {
	switch v := value.(type) {
	case nil:
		return "NULL", nil
	case bool:
		if v {
			return "TRUE", nil
		}
		return "FALSE", nil
	case int:
		return strconv.FormatInt(int64(v), 10), nil
	case int8:
		return strconv.FormatInt(int64(v), 10), nil
	case int16:
		return strconv.FormatInt(int64(v), 10), nil
	case int32:
		return strconv.FormatInt(int64(v), 10), nil
	case int64:
		return strconv.FormatInt(v, 10), nil
	case uint:
		return strconv.FormatUint(uint64(v), 10), nil
	case uint8:
		return strconv.FormatUint(uint64(v), 10), nil
	case uint16:
		return strconv.FormatUint(uint64(v), 10), nil
	case uint32:
		return strconv.FormatUint(uint64(v), 10), nil
	case uint64:
		return strconv.FormatUint(v, 10), nil
	case float32:
		f := float64(v)
		if math.IsInf(f, 0) || math.IsNaN(f) {
			return "", &ProgrammingError{Message: "non-finite numeric parameter is not supported"}
		}
		return strconv.FormatFloat(f, 'f', -1, 64), nil
	case float64:
		if math.IsInf(v, 0) || math.IsNaN(v) {
			return "", &ProgrammingError{Message: "non-finite numeric parameter is not supported"}
		}
		return strconv.FormatFloat(v, 'f', -1, 64), nil
	case string:
		return "'" + strings.ReplaceAll(v, "'", "''") + "'", nil
	default:
		return "", &ProgrammingError{Message: fmt.Sprintf("unsupported parameter type: %T", value)}
	}
}
