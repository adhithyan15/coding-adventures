package satsolver

import (
	"errors"
	"fmt"
	"strconv"
	"strings"
)

type CnfFormula struct {
	NumVars uint32
	Clauses [][]Lit
}
type ParseDimacsOptions struct{ MaxVars uint32 }

func ParseDimacs(text string, options ...ParseDimacsOptions) (CnfFormula, error) {
	max := uint32(1 << 20)
	if len(options) > 1 {
		return CnfFormula{}, errors.New("at most one parser options value")
	}
	if len(options) == 1 && options[0].MaxVars > 0 {
		max = options[0].MaxVars
	}
	if max > maxVar+1 {
		return CnfFormula{}, errors.New("MaxVars exceeds literal representation")
	}
	text = strings.ReplaceAll(strings.ReplaceAll(text, "\r\n", "\n"), "\r", "\n")
	lines := strings.Split(text, "\n")
	var n uint32
	var expected uint64
	header := false
	var clauses [][]Lit
	var cur []Lit
	for i, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "c") {
			continue
		}
		if line == "%" {
			break
		}
		if !header {
			f := strings.Fields(line)
			if len(f) != 4 || f[0] != "p" || f[1] != "cnf" {
				return CnfFormula{}, fmt.Errorf("line %d: malformed DIMACS header", i+1)
			}
			if !unsignedDecimal(f[2]) || !unsignedDecimal(f[3]) {
				return CnfFormula{}, fmt.Errorf("line %d: invalid header count spelling", i+1)
			}
			nv, e1 := strconv.ParseUint(f[2], 10, 32)
			nc, e2 := strconv.ParseUint(f[3], 10, 64)
			if e1 != nil || e2 != nil {
				return CnfFormula{}, fmt.Errorf("line %d: invalid header counts", i+1)
			}
			if nv > uint64(max) || nv > uint64(maxVar)+1 {
				return CnfFormula{}, errors.New("declared variables exceed configured limit")
			}
			n = uint32(nv)
			expected = nc
			header = true
			continue
		}
		for _, tok := range strings.Fields(line) {
			if !signedDecimal(tok) {
				return CnfFormula{}, fmt.Errorf("line %d: invalid literal spelling %q", i+1, tok)
			}
			v, e := strconv.ParseInt(tok, 10, 64)
			if e != nil {
				return CnfFormula{}, fmt.Errorf("line %d: malformed literal %q", i+1, tok)
			}
			if v == 0 {
				clauses = append(clauses, append([]Lit{}, cur...))
				cur = nil
				continue
			}
			if v < -int64(n) || v > int64(n) {
				return CnfFormula{}, fmt.Errorf("line %d: literal %d out of range", i+1, v)
			}
			neg := v < 0
			if neg {
				v = -v
			}
			l, e := MakeLit(Var(v-1), neg)
			if e != nil {
				return CnfFormula{}, e
			}
			cur = append(cur, l)
		}
	}
	if !header {
		return CnfFormula{}, errors.New("missing DIMACS header")
	}
	if len(cur) != 0 {
		return CnfFormula{}, errors.New("unterminated DIMACS clause")
	}
	if uint64(len(clauses)) != expected {
		return CnfFormula{}, fmt.Errorf("DIMACS clause count mismatch: declared %d, parsed %d", expected, len(clauses))
	}
	return CnfFormula{n, clauses}, nil
}
func unsignedDecimal(s string) bool {
	if s == "" {
		return false
	}
	for _, c := range s {
		if c < '0' || c > '9' {
			return false
		}
	}
	return true
}
func signedDecimal(s string) bool {
	if strings.HasPrefix(s, "-") {
		s = s[1:]
	}
	return unsignedDecimal(s)
}
func WriteDimacs(f CnfFormula) (string, error) {
	if f.NumVars > maxVar+1 {
		return "", errors.New("variable count exceeds literal representation")
	}
	var b strings.Builder
	fmt.Fprintf(&b, "p cnf %d %d\n", f.NumVars, len(f.Clauses))
	for _, c := range f.Clauses {
		if len(c) == 0 {
			b.WriteByte(' ')
		}
		for _, l := range c {
			if !validLit(l) || uint32(l)>>1 >= f.NumVars {
				return "", fmt.Errorf("invalid literal %d", l)
			}
			n := int64(uint32(l)>>1) + 1
			if uint32(l)&1 != 0 {
				n = -n
			}
			fmt.Fprintf(&b, "%d ", n)
		}
		b.WriteString("0\n")
	}
	out := b.String()
	if len(f.Clauses) == 0 {
		return strings.TrimSuffix(out, "\n"), nil
	}
	return strings.TrimSuffix(out, "\n"), nil
}
