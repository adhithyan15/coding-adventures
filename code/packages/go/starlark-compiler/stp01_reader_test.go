package starlarkcompiler

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"strconv"
	"strings"
	"unicode/utf8"
)

// This reader is test-only. The public package remains an inert typed table:
// JSON transports expectations to its tests, never commands to a compiler.
// Bound bytes first, preserve token types/keys second, then check table shape.
func stpRead(r io.Reader) ([]byte, error) {
	b, err := io.ReadAll(io.LimitReader(r, 131073))
	if err != nil {
		return nil, err
	}
	if len(b) > 131072 {
		return nil, fmt.Errorf("STP01 byte limit")
	}
	return b, nil
}

// encoding/json intentionally replaces malformed surrogate escapes. Inspect
// escapes before decoding so replacement cannot erase transport evidence.
// Skipping each complete escape distinguishes \\ud800 from a Unicode escape.
func stpScalars(b []byte) error {
	if !utf8.Valid(b) {
		return fmt.Errorf("invalid UTF-8")
	}
	inString := false
	for i := 0; i < len(b); i++ {
		if b[i] == '"' {
			inString = !inString
			continue
		}
		if !inString || b[i] != '\\' {
			continue
		}
		i++
		if i >= len(b) {
			return fmt.Errorf("truncated escape")
		}
		if b[i] != 'u' {
			continue
		}
		if i+4 >= len(b) {
			return fmt.Errorf("truncated Unicode escape")
		}
		n, err := strconv.ParseUint(string(b[i+1:i+5]), 16, 16)
		if err != nil {
			return err
		}
		i += 4
		if n >= 0xdc00 && n <= 0xdfff {
			return fmt.Errorf("unpaired low surrogate")
		}
		if n >= 0xd800 && n <= 0xdbff {
			if i+6 >= len(b) || b[i+1] != '\\' || b[i+2] != 'u' {
				return fmt.Errorf("unpaired high surrogate")
			}
			low, err := strconv.ParseUint(string(b[i+3:i+7]), 16, 16)
			if err != nil || low < 0xdc00 || low > 0xdfff {
				return fmt.Errorf("invalid surrogate pair")
			}
			i += 6
		}
	}
	return nil
}

func stpJSON(b []byte) (any, error) {
	if len(b) > 131072 {
		return nil, fmt.Errorf("STP01 byte limit")
	}
	if err := stpScalars(b); err != nil {
		return nil, err
	}
	d := json.NewDecoder(bytes.NewReader(b))
	d.UseNumber()
	var read func(int) (any, error)
	read = func(depth int) (any, error) {
		t, err := d.Token()
		if err != nil {
			return nil, err
		}
		delim, container := t.(json.Delim)
		if !container {
			return t, nil
		}
		if depth > 8 {
			return nil, fmt.Errorf("STP01 nesting limit")
		}
		switch delim {
		case '{':
			m := map[string]any{}
			for d.More() {
				key, err := d.Token()
				if err != nil {
					return nil, err
				}
				s, ok := key.(string)
				if !ok {
					return nil, fmt.Errorf("object key required")
				}
				if _, exists := m[s]; exists {
					return nil, fmt.Errorf("duplicate decoded key %q", s)
				}
				v, err := read(depth + 1)
				if err != nil {
					return nil, err
				}
				m[s] = v
			}
			if _, err := d.Token(); err != nil {
				return nil, err
			}
			return m, nil
		case '[':
			a := []any{}
			for d.More() {
				if len(a) >= 512 {
					return nil, fmt.Errorf("STP01 array limit")
				}
				v, err := read(depth + 1)
				if err != nil {
					return nil, err
				}
				a = append(a, v)
			}
			if _, err := d.Token(); err != nil {
				return nil, err
			}
			return a, nil
		default:
			return nil, fmt.Errorf("unexpected delimiter")
		}
	}
	v, err := read(1)
	if err != nil {
		return nil, err
	}
	if _, err = d.Token(); err != io.EOF {
		return nil, fmt.Errorf("trailing JSON content")
	}
	return v, nil
}

type stpRow struct {
	number           int
	symbol, category string
	absent           bool
}
type stpOperator struct{ operator, symbol string }
type stpUnknown struct{ group, operator string }
type stpCorpus struct {
	ops, decode []stpRow
	operators   map[string][]stpOperator
	unknown     []stpUnknown
}

// Accessors retain the first shape error. Missing keys cannot become legal
// nulls: exact object membership is checked before inspecting values. Zero
// fallback values only keep validation bounded; no API sees a rejected corpus.
type stpShape struct{ err error }

func (r *stpShape) fail(message string) {
	if r.err == nil {
		r.err = fmt.Errorf("STP01 %s", message)
	}
}
func (r *stpShape) object(v any, keys ...string) map[string]any {
	m, ok := v.(map[string]any)
	if !ok {
		r.fail("object required")
		return nil
	}
	if len(m) != len(keys) {
		r.fail("closed object members")
	}
	for _, k := range keys {
		if _, ok := m[k]; !ok {
			r.fail("missing " + k)
		}
	}
	return m
}
func (r *stpShape) array(v any, count int) []any {
	a, ok := v.([]any)
	if !ok || len(a) != count {
		r.fail("exact row count required")
		return nil
	}
	return a
}
func (r *stpShape) text(v any, limit int) string {
	s, ok := v.(string)
	if !ok || utf8.RuneCountInString(s) > limit {
		r.fail("bounded string required")
		return ""
	}
	return s
}
func (r *stpShape) integer(v any, max int) int {
	n, ok := v.(json.Number)
	if !ok {
		r.fail("integer required")
		return 0
	}
	s := n.String()
	if strings.ContainsAny(s, ".eE") {
		r.fail("integer lexical form required")
		return 0
	}
	i, err := strconv.ParseInt(s, 10, 64)
	if err != nil || i < 0 || i > int64(max) {
		r.fail("integer out of range")
		return 0
	}
	return int(i)
}

var stpGroupOrder = []string{"binary", "compare", "augmented_assign", "unary"}

// These spellings/order come from STP01, independently of native constructors.
var stpOperatorOrder = map[string][]string{
	"binary":           {"+", "-", "*", "/", "//", "%", "**", "&", "|", "^", "<<", ">>"},
	"compare":          {"==", "!=", "<", ">", "<=", ">=", "in", "not in"},
	"augmented_assign": {"+=", "-=", "*=", "/=", "//=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>="},
	"unary":            {"-", "~"},
}
var stpUnknownOrder = []stpUnknown{{"binary", "@"}, {"binary", "++"}, {"binary", "＋"}, {"compare", "IN"}, {"compare", "not  in"}, {"augmented_assign", ":="}, {"unary", "+"}, {"unary", "not"}}

func stpDecode(b []byte) (stpCorpus, error) {
	c := stpCorpus{operators: map[string][]stpOperator{}}
	v, err := stpJSON(b)
	if err != nil {
		return c, err
	}
	r := &stpShape{}
	root := r.object(v, "schema_version", "profile", "opcodes", "decode", "operators", "unknown_lookups")
	if r.integer(root["schema_version"], 1) != 1 || r.text(root["profile"], 64) != "standalone-provider-v1" {
		r.fail("unsupported profile/version")
	}
	for _, table := range []struct {
		name  string
		count int
		out   *[]stpRow
	}{{"opcodes", 61, &c.ops}, {"decode", 256, &c.decode}} {
		previous := -1
		seen := map[string]bool{}
		for i, v := range r.array(root[table.name], table.count) {
			m := r.object(v, "byte", "symbol", "category")
			row := stpRow{number: r.integer(m["byte"], 255)}
			if row.number <= previous || (table.name == "decode" && row.number != i) {
				r.fail("ascending byte records required")
			}
			previous = row.number
			if m["symbol"] == nil && m["category"] == nil && table.name == "decode" {
				row.absent = true
			} else {
				row.symbol = r.text(m["symbol"], 64)
				row.category = r.text(m["category"], 64)
				if seen[row.symbol] {
					r.fail("duplicate assigned symbol")
				}
				seen[row.symbol] = true
			}
			*table.out = append(*table.out, row)
		}
	}
	groups := r.object(root["operators"], stpGroupOrder...)
	for _, group := range stpGroupOrder {
		for i, v := range r.array(groups[group], len(stpOperatorOrder[group])) {
			m := r.object(v, "operator", "symbol")
			row := stpOperator{r.text(m["operator"], 32), r.text(m["symbol"], 64)}
			if row.operator != stpOperatorOrder[group][i] {
				r.fail("ordered operator domain required")
			}
			c.operators[group] = append(c.operators[group], row)
		}
	}
	for i, v := range r.array(root["unknown_lookups"], 8) {
		m := r.object(v, "group", "operator", "symbol")
		row := stpUnknown{r.text(m["group"], 64), r.text(m["operator"], 32)}
		if row != stpUnknownOrder[i] || m["symbol"] != nil {
			r.fail("exact unknown roster required")
		}
		c.unknown = append(c.unknown, row)
	}
	return c, r.err
}

type stpGroup struct {
	makeMap func() map[string]Op
	lookup  func(string) (Op, bool)
}

func stpGroups() map[string]stpGroup {
	return map[string]stpGroup{
		"binary": {BinaryOpMap, BinaryOpcode}, "compare": {CompareOpMap, CompareOpcode}, "augmented_assign": {AugmentedAssignMap, AugmentedAssignOpcode}, "unary": {UnaryOpMap, UnaryOpcode},
	}
}

// The fixture supplies expected values, while the provider supplies observed
// behavior. No expected table is generated from implementation declarations.
func stpCompare(c stpCorpus) ([5]int, error) {
	counts := [5]int{}
	expectedBytes := map[string]int{}
	for _, row := range c.ops {
		expectedBytes[row.symbol] = row.number
	}
	all := AllOps()
	if len(all) != len(c.ops) {
		return counts, fmt.Errorf("AllOps count")
	}
	check := func(op Op, row stpRow) bool {
		cat, ok := op.Category()
		return ok && int(op.Byte()) == row.number && op.String() == row.symbol && string(cat) == row.category
	}
	for i, row := range c.ops {
		op, ok := OpFromByte(byte(row.number))
		if !ok || op != all[i] || !check(all[i], row) {
			return counts, fmt.Errorf("opcode %d disagrees with provider", i)
		}
		counts[0]++
	}
	for _, row := range c.decode {
		op, ok := OpFromByte(byte(row.number))
		if row.absent {
			if _, category := Op(byte(row.number)).Category(); ok || category {
				return counts, fmt.Errorf("hole %d resolved", row.number)
			}
			counts[4]++
		} else if !ok || !check(op, row) {
			return counts, fmt.Errorf("decoder %d disagrees", row.number)
		}
		counts[1]++
	}
	groups := stpGroups()
	for _, name := range stpGroupOrder {
		api := groups[name]
		m := api.makeMap()
		if len(m) != len(c.operators[name]) {
			return counts, fmt.Errorf("operator count %s", name)
		}
		for _, row := range c.operators[name] {
			op, ok := m[row.operator]
			direct, found := api.lookup(row.operator)
			wantByte, assigned := expectedBytes[row.symbol]
			if !ok || !found || !assigned || int(op.Byte()) != wantByte || op != direct || op.String() != row.symbol {
				return counts, fmt.Errorf("operator %s/%s disagrees", name, row.operator)
			}
			counts[2]++
		}
	}
	for _, row := range c.unknown {
		if _, ok := groups[row.group].lookup(row.operator); ok {
			return counts, fmt.Errorf("unknown lookup resolved")
		}
		counts[3]++
	}
	if counts != [5]int{61, 256, 34, 8, 195} {
		return counts, fmt.Errorf("incomplete visits/holes: %v", counts)
	}
	return counts, nil
}
