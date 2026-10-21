package compilerir

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

// The wire reader is test transport, not production conformance. Preserve raw
// lexical evidence before encoding/json can overwrite keys or replace Unicode.
func coreReadJSON(r io.Reader) (any, error) {
	b, e := io.ReadAll(io.LimitReader(r, 1048577))
	if e != nil {
		return nil, e
	}
	if len(b) > 1048576 || !utf8.Valid(b) || bytes.HasPrefix(b, []byte{239, 187, 191}) {
		return nil, fmt.Errorf("wire bytes")
	}
	quoted := false
	for i := 0; i < len(b); i++ {
		if b[i] == '"' {
			quoted = !quoted
			continue
		}
		if !quoted || b[i] != '\\' {
			continue
		}
		i++
		if i >= len(b) {
			return nil, fmt.Errorf("escape")
		}
		if b[i] != 'u' {
			continue
		}
		if i+4 >= len(b) {
			return nil, fmt.Errorf("unicode escape")
		}
		n, err := strconv.ParseUint(string(b[i+1:i+5]), 16, 16)
		if err != nil {
			return nil, err
		}
		i += 4
		if n >= 0xdc00 && n <= 0xdfff {
			return nil, fmt.Errorf("lone low surrogate")
		}
		if n >= 0xd800 && n <= 0xdbff {
			if i+6 >= len(b) || b[i+1] != '\\' || b[i+2] != 'u' {
				return nil, fmt.Errorf("lone high surrogate")
			}
			low, err := strconv.ParseUint(string(b[i+3:i+7]), 16, 16)
			if err != nil || low < 0xdc00 || low > 0xdfff {
				return nil, fmt.Errorf("surrogate pair")
			}
			i += 6
		}
	}
	d := json.NewDecoder(bytes.NewReader(b))
	d.UseNumber()
	nodes, total := 0, 0
	var value func(int) (any, error)
	value = func(depth int) (any, error) {
		nodes++
		if depth > 16 || nodes > 100000 {
			return nil, fmt.Errorf("tree budget")
		}
		tok, err := d.Token()
		if err != nil {
			return nil, err
		}
		switch x := tok.(type) {
		case json.Delim:
			if x == '{' {
				out := map[string]any{}
				for d.More() {
					k, e := d.Token()
					if e != nil {
						return nil, e
					}
					key, ok := k.(string)
					if !ok {
						return nil, fmt.Errorf("key")
					}
					nodes++
					total += len(key)
					if _, ok := out[key]; ok {
						return nil, fmt.Errorf("duplicate key")
					}
					v, e := value(depth + 1)
					if e != nil {
						return nil, e
					}
					out[key] = v
				}
				_, err = d.Token()
				return out, err
			}
			if x == '[' {
				out := []any{}
				for d.More() {
					v, e := value(depth + 1)
					if e != nil {
						return nil, e
					}
					out = append(out, v)
				}
				_, err = d.Token()
				return out, err
			}
			return nil, fmt.Errorf("delimiter")
		case json.Number:
			s := string(x)
			digits := strings.TrimPrefix(s, "-")
			if len(digits) > 10 || strings.ContainsAny(s, ".eE+") || s == "-0" {
				return nil, fmt.Errorf("integer lexeme")
			}
			n, e := strconv.ParseInt(s, 10, 64)
			return n, e
		case string:
			total += len(x)
			if total > 1048576 {
				return nil, fmt.Errorf("string budget")
			}
			return x, nil
		default:
			return x, nil
		}
	}
	v, e := value(1)
	if e != nil {
		return nil, e
	}
	if _, e = d.Token(); e != io.EOF {
		return nil, fmt.Errorf("trailing JSON")
	}
	return v, nil
}

func coreObject(v any, keys string) (map[string]any, error) {
	m, ok := v.(map[string]any)
	want := strings.Fields(keys)
	if !ok || len(m) != len(want) {
		return nil, fmt.Errorf("object shape")
	}
	for _, k := range want {
		if _, ok := m[k]; !ok {
			return nil, fmt.Errorf("missing %s", k)
		}
	}
	return m, nil
}
func coreInt(v any) (int, error) {
	n, ok := v.(int64)
	if !ok || int64(int(n)) != n {
		return 0, fmt.Errorf("native integer")
	}
	return int(n), nil
}

// Conversion preserves semantic negatives. In particular, label never becomes
// text, and unknown spelling exercises real ParseOp plus strict opcode rejection.
func coreWireProgram(v any) (*IrProgram, error) {
	m, e := coreObject(v, "version entry data instructions")
	if e != nil {
		return nil, e
	}
	version, e := coreInt(m["version"])
	if e != nil {
		return nil, e
	}
	entry, ok := m["entry"].(string)
	if !ok {
		return nil, fmt.Errorf("entry type")
	}
	p := &IrProgram{Version: version, EntryLabel: entry, Data: []IrDataDecl{}, Instructions: []IrInstruction{}}
	ds, ok := m["data"].([]any)
	if !ok {
		return nil, fmt.Errorf("data type")
	}
	for _, v := range ds {
		d, e := coreObject(v, "label size init")
		if e != nil {
			return nil, e
		}
		name, ok := d["label"].(string)
		if !ok {
			return nil, fmt.Errorf("label type")
		}
		size, e := coreInt(d["size"])
		if e != nil {
			return nil, e
		}
		init, e := coreInt(d["init"])
		if e != nil {
			return nil, e
		}
		p.Data = append(p.Data, IrDataDecl{name, size, init})
	}
	is, ok := m["instructions"].([]any)
	if !ok {
		return nil, fmt.Errorf("instructions type")
	}
	for _, v := range is {
		r, e := coreObject(v, "op operands id")
		if e != nil {
			return nil, e
		}
		name, ok := r["op"].(string)
		if !ok {
			return nil, fmt.Errorf("opcode type")
		}
		op, known := ParseOp(name)
		if !known {
			op = IrOp(-1)
		}
		id, e := coreInt(r["id"])
		if e != nil {
			return nil, e
		}
		args, ok := r["operands"].([]any)
		if !ok {
			return nil, fmt.Errorf("operands type")
		}
		ins := IrInstruction{Opcode: op, ID: id, Operands: []IrOperand{}}
		for _, v := range args {
			a, ok := v.(map[string]any)
			if !ok {
				return nil, fmt.Errorf("operand shape")
			}
			switch a["kind"] {
			case "register":
				a, e = coreObject(v, "kind index")
				if e != nil {
					return nil, e
				}
				n, e := coreInt(a["index"])
				if e != nil {
					return nil, e
				}
				ins.Operands = append(ins.Operands, IrRegister{n})
			case "immediate":
				a, e = coreObject(v, "kind value")
				if e != nil {
					return nil, e
				}
				n, e := coreInt(a["value"])
				if e != nil {
					return nil, e
				}
				ins.Operands = append(ins.Operands, IrImmediate{n})
			case "label":
				a, e = coreObject(v, "kind name")
				if e != nil {
					return nil, e
				}
				s, ok := a["name"].(string)
				if !ok {
					return nil, fmt.Errorf("label type")
				}
				ins.Operands = append(ins.Operands, IrLabel{s})
			case "text":
				a, e = coreObject(v, "kind value")
				if e != nil {
					return nil, e
				}
				s, ok := a["value"].(string)
				if !ok {
					return nil, fmt.Errorf("text type")
				}
				ins.Operands = append(ins.Operands, IrText{s})
			default:
				return nil, fmt.Errorf("operand tag")
			}
		}
		p.Instructions = append(p.Instructions, ins)
	}
	return p, nil
}

var coreCaseNames = map[string]string{
	"programs":         "empty all_core sparse_ids numeric_bounds",
	"invalid_programs": "meta_id duplicate_id negative_id version_two register_over immediate_over wrong_arity wrong_tag register_label comment_hash comment_space duplicate_label duplicate_data extension_MUL extension_DIV extension_OR extension_OR_IMM extension_XOR extension_XOR_IMM extension_NOT",
	"invalid_texts":    "missing_version version_two duplicate_version missing_entry duplicate_entry late_data partial_id missing_id leading_zero plus_id float negative_zero wrong_case unknown_op crlf no_final_lf tab extra_blank padded_halt comment_only_id unknown_directive empty_text",
	"id_sequences":     "initial empty suffix last",
}
var coreLiteralOps = []string{
	"LOAD_IMM constants register immediate", "LOAD_ADDR constants register label",
	"LOAD_BYTE memory register register register|immediate", "STORE_BYTE memory register register register|immediate", "LOAD_WORD memory register register register|immediate", "STORE_WORD memory register register register|immediate",
	"ADD arithmetic register register register", "ADD_IMM arithmetic register register immediate", "SUB arithmetic register register register", "AND arithmetic register register register", "AND_IMM arithmetic register register immediate",
	"CMP_EQ comparison register register register", "CMP_NE comparison register register register", "CMP_LT comparison register register register", "CMP_GT comparison register register register",
	"LABEL control_flow label", "JUMP control_flow label", "BRANCH_Z control_flow register label", "BRANCH_NZ control_flow register label", "CALL control_flow label", "RET control_flow", "SYSCALL system immediate", "HALT system", "NOP meta", "COMMENT meta text",
}

const coreSparseGolden = ".version 1\n\n.entry _start\n\n_start:\n  LOAD_IMM   v0, -1  ; #42\n  ; note\n  NOP  ; #7\n  HALT  ; #99\n"

func coreCorpus(v any) (map[string]any, error) {
	m, e := coreObject(v, "schema_version profile opcodes programs invalid_programs invalid_texts id_sequences")
	if e != nil {
		return nil, e
	}
	if m["schema_version"] != int64(1) || m["profile"] != "core-text-v1" {
		return nil, fmt.Errorf("profile")
	}
	ops, ok := m["opcodes"].([]any)
	if !ok || len(ops) != 25 {
		return nil, fmt.Errorf("operation roster")
	}
	for n, v := range ops {
		r, e := coreObject(v, "name category operands")
		if e != nil {
			return nil, e
		}
		lit := strings.Fields(coreLiteralOps[n])
		args := []any{}
		for _, a := range lit[2:] {
			args = append(args, a)
		}
		if r["name"] != lit[0] || r["category"] != lit[1] || !reflect.DeepEqual(r["operands"], args) {
			return nil, fmt.Errorf("operation role")
		}
	}
	cases, insts, decls, args := 0, 0, 0, 0
	for key, names := range coreCaseNames {
		rows, ok := m[key].([]any)
		want := strings.Fields(names)
		if !ok || len(rows) != len(want) {
			return nil, fmt.Errorf("case roster")
		}
		cases += len(rows)
		for n, v := range rows {
			keys := "name text"
			switch key {
			case "programs":
				keys = "name program text"
			case "invalid_programs":
				keys = "name program"
			case "id_sequences":
				keys = "name start count values next"
			}
			row, e := coreObject(v, keys)
			if e != nil {
				return nil, e
			}
			if row["name"] != want[n] {
				return nil, fmt.Errorf("case name")
			}
			if key == "programs" || key == "invalid_programs" {
				p, e := coreWireProgram(row["program"])
				if e != nil {
					return nil, e
				}
				insts += len(p.Instructions)
				decls += len(p.Data)
				for _, i := range p.Instructions {
					args += len(i.Operands)
				}
			}
			if key == "programs" || key == "invalid_texts" {
				if _, ok := row["text"].(string); !ok {
					return nil, fmt.Errorf("text type")
				}
			}
			if key == "id_sequences" {
				for _, k := range []string{"start", "count", "next"} {
					if _, ok := row[k].(int64); !ok {
						return nil, fmt.Errorf("id type")
					}
				}
				vs, ok := row["values"].([]any)
				if !ok {
					return nil, fmt.Errorf("values type")
				}
				for _, v := range vs {
					if _, ok := v.(int64); !ok {
						return nil, fmt.Errorf("value type")
					}
				}
			}
		}
	}
	if cases > 128 || insts > 4096 || decls > 512 || args > 12288 {
		return nil, fmt.Errorf("aggregate budget")
	}
	return m, nil
}

func TestCoreFrozenCorpus(t *testing.T) {
	f, e := os.Open("../../../specs/fixtures/compiler-ir-v1/cases.json")
	if e != nil {
		t.Fatal(e)
	}
	defer f.Close()
	b, e := io.ReadAll(io.LimitReader(f, 1048577))
	if e != nil || len(b) > 1048576 {
		t.Fatal("corpus read budget", e)
	}
	if fmt.Sprintf("%x", sha256.Sum256(b)) != "f4a620e6c82ede6da52f3ee67940c18fa4f0a78915b56bbf4537c61c8e0faa52" {
		t.Fatal("frozen corpus changed; independent anchors need review")
	}
	v, e := coreReadJSON(bytes.NewReader(b))
	if e != nil {
		t.Fatal(e)
	}
	m, e := coreCorpus(v)
	if e != nil {
		t.Fatal(e)
	}
	for _, key := range []string{"programs", "invalid_programs"} {
		for _, r := range m[key].([]any) {
			row := r.(map[string]any)
			p, e := coreWireProgram(row["program"])
			if e != nil {
				t.Fatal(e)
			}
			t.Run(row["name"].(string), func(t *testing.T) {
				before := fmt.Sprintf("%#v", p)
				text, e := PrintCoreProfile(p)
				if key == "invalid_programs" {
					if e == nil || ValidateCoreProfile(p) == nil {
						t.Fatal("semantic rejection accepted")
					}
					return
				}
				if e != nil || text != row["text"] {
					t.Fatalf("print: %q %v", text, e)
				}
				parsed, e := ParseCoreProfile(text)
				if e != nil || !reflect.DeepEqual(parsed, p) {
					t.Fatalf("roundtrip %v %#v", e, parsed)
				}
				if before != fmt.Sprintf("%#v", p) {
					t.Fatal("mutated input")
				}
				if row["name"] == "sparse_ids" && text != coreSparseGolden {
					t.Fatal("independent sparse witness")
				}
			})
		}
	}
	for _, v := range m["invalid_texts"].([]any) {
		r := v.(map[string]any)
		t.Run("text_"+r["name"].(string), func(t *testing.T) {
			if _, e := ParseCoreProfile(r["text"].(string)); e == nil {
				t.Fatal("invalid text accepted")
			}
		})
	}
	for _, v := range m["id_sequences"].([]any) {
		r := v.(map[string]any)
		values, next, e := CoreIDSequence(r["start"].(int64), int(r["count"].(int64)))
		want := []int64{}
		for _, v := range r["values"].([]any) {
			want = append(want, v.(int64))
		}
		if e != nil || next != r["next"] || !reflect.DeepEqual(values, want) {
			t.Fatal(r["name"], values, next, e)
		}
	}
	// Mutations exercise independent rosters without the digest short circuit.
	for _, key := range []string{"opcodes", "programs", "invalid_programs", "invalid_texts", "id_sequences"} {
		copy := map[string]any{}
		for k, v := range m {
			copy[k] = v
		}
		copy[key] = []any{}
		if _, e := coreCorpus(copy); e == nil {
			t.Fatal("erased roster", key)
		}
	}
	// Corrupt admitted data without erasing a roster. These witnesses exercise
	// the native boundary and byte equality, not just the digest guard.
	for _, mutation := range []string{"comment_tag", "duplicate_id", "changed_id", "instruction_order", "data_order", "golden_bytes"} {
		encoded, _ := json.Marshal(m)
		fresh, err := coreReadJSON(bytes.NewReader(encoded))
		if err != nil {
			t.Fatal(err)
		}
		rows := fresh.(map[string]any)["programs"].([]any)
		index := 2
		if mutation == "data_order" {
			index = 3
		}
		row := rows[index].(map[string]any)
		wire := row["program"].(map[string]any)
		instructions := wire["instructions"].([]any)
		switch mutation {
		case "comment_tag":
			instructions[2].(map[string]any)["operands"] = []any{map[string]any{"kind": "label", "name": "note"}}
		case "duplicate_id":
			instructions[1].(map[string]any)["id"] = int64(7)
		case "changed_id":
			instructions[1].(map[string]any)["id"] = int64(43)
		case "instruction_order":
			instructions[1], instructions[3] = instructions[3], instructions[1]
		case "data_order":
			data := wire["data"].([]any)
			data[0], data[1] = data[1], data[0]
		case "golden_bytes":
			row["text"] = row["text"].(string) + "\n"
		}
		p, err := coreWireProgram(wire)
		if err != nil {
			t.Fatal("reader erased semantic negative", mutation, err)
		}
		if mutation == "comment_tag" {
			if _, ok := p.Instructions[2].Operands[0].(IrLabel); !ok {
				t.Fatal("reader retagged label")
			}
		}
		text, err := PrintCoreProfile(p)
		if mutation == "comment_tag" || mutation == "duplicate_id" {
			if err == nil {
				t.Fatal("semantic corruption accepted", mutation)
			}
		} else if err != nil || text == row["text"] {
			t.Fatal("lost data/ID/order/golden witness", mutation, err)
		}
	}
}

func TestCoreReaderAdmission(t *testing.T) {
	bad := []string{"{} {}", "{\"a\":1,\"a\":2}", "{\"a\":1,\"\\u0061\":2}", "1.0", "1e0", "-0", "99999999999", "NaN", "\"\\ud800\"", "\"\\udc00\"", "\"\\ud800\\u0041\"", string([]byte{'"', 255, '"'}), "\ufeff{}", strings.Repeat("[", 17) + "0" + strings.Repeat("]", 17), strings.Repeat(" ", 1048577)}
	for _, s := range bad {
		if _, e := coreReadJSON(strings.NewReader(s)); e == nil {
			t.Fatalf("accepted %.70q", s)
		}
	}
	for _, s := range []string{"\"\\ud83d\\ude00\"", "\"\\\\ud800\"", "{\"a\":-2147483648}", "[]"} {
		if _, e := coreReadJSON(strings.NewReader(s)); e != nil {
			t.Fatal(s, e)
		}
	}
	for _, s := range []string{"{}", "{\"version\":true,\"entry\":\"a\",\"data\":[],\"instructions\":[]}", "{\"version\":1,\"entry\":\"a\",\"data\":[],\"instructions\":[],\"extra\":0}"} {
		v, e := coreReadJSON(strings.NewReader(s))
		if e != nil {
			t.Fatal(e)
		}
		if _, e := coreWireProgram(v); e == nil {
			t.Fatal("wire shape accepted", s)
		}
	}
}

func TestCoreNativeBoundsOwnershipAndLegacy(t *testing.T) {
	p, e := ParseCoreProfile(coreSparseGolden)
	if e != nil {
		t.Fatal(e)
	}
	other, _ := ParseCoreProfile(coreSparseGolden)
	p.Instructions[1].Operands[0] = IrRegister{8}
	p.Instructions = append(p.Instructions, IrInstruction{})
	if text, _ := PrintCoreProfile(other); text != coreSparseGolden {
		t.Fatal("parse alias")
	}
	legacy := &IrProgram{Version: 1, EntryLabel: "a", Instructions: []IrInstruction{{Opcode: OpComment, Operands: []IrOperand{IrLabel{"function: NAME"}}, ID: -1}, {Opcode: OpHalt, ID: 0}}}
	text := Print(legacy)
	if !strings.Contains(text, "  ; function: NAME\n") || !strings.Contains(text, "  HALT         ; #0\n") {
		t.Fatal("legacy bytes")
	}
	round, e := Parse(text)
	if e != nil || round.Instructions[0].Operands[0] != (IrLabel{"function: NAME"}) {
		t.Fatal("legacy comment", e)
	}
	if ValidateCoreProfile(legacy) == nil {
		t.Fatal("wrong comment tag accepted")
	}
	var reg *IrRegister
	for _, operand := range []IrOperand{nil, reg, &IrRegister{0}, IrRegister{-1}, IrRegister{65536}, IrImmediate{0}, IrLabel{"x"}, IrText{"x"}} {
		q := NewIrProgram("a")
		q.Instructions = []IrInstruction{{Opcode: OpLoadImm, Operands: []IrOperand{operand, IrImmediate{0}}, ID: 0}}
		if ValidateCoreProfile(q) == nil {
			t.Fatal("bad operand", operand)
		}
	}
	if ValidateCoreProfile(nil) == nil {
		t.Fatal("nil program")
	}
	for _, q := range []*IrProgram{{Version: 1, EntryLabel: "a", Data: make([]IrDataDecl, 33)}, {Version: 1, EntryLabel: "a", Instructions: make([]IrInstruction, 257)}, {Version: 1, EntryLabel: "a", Instructions: []IrInstruction{{Opcode: OpLabel, ID: -1}}}} {
		if _, e := PrintCoreProfile(q); e == nil {
			t.Fatal("bounds/arity")
		}
	}
	for _, text := range []string{strings.Repeat("a", 65537), ".version 1\n\n.entry é\n", strings.Repeat("\n", 1025)} {
		if _, e := ParseCoreProfile(text); e == nil {
			t.Fatal("text budget/ascii")
		}
	}
	for _, x := range []struct {
		s int64
		n int
	}{{-1, 1}, {2147483648, 0}, {0, -1}, {0, 257}, {2147483647, 2}} {
		v, next, e := CoreIDSequence(x.s, x.n)
		if e == nil || v != nil || next != x.s {
			t.Fatal("atomic allocation", x, v, next, e)
		}
	}
	values, _, _ := CoreIDSequence(0, 2)
	values[0] = 99
	again, _, _ := CoreIDSequence(0, 2)
	if again[0] != 0 {
		t.Fatal("ID alias")
	}
}

func TestCoreRolesAndRemainingBoundaries(t *testing.T) {
	for _, name := range []string{"MUL", "DIV", "OR", "OR_IMM", "XOR", "XOR_IMM", "NOT"} {
		if _, ok := ParseOp(name); ok {
			t.Fatal("extension admitted", name)
		}
	}
	for _, literal := range coreLiteralOps {
		parts := strings.Fields(literal)
		op, ok := ParseOp(parts[0])
		if !ok || op.String() != parts[0] {
			t.Fatal("native operation", parts[0])
		}
		p := NewIrProgram("a")
		ins := IrInstruction{Opcode: op, ID: 0}
		if op == OpLabel || op == OpComment {
			ins.ID = -1
		}
		for _, role := range parts[2:] {
			switch role {
			case "register", "register|immediate":
				ins.Operands = append(ins.Operands, IrRegister{0})
			case "immediate":
				ins.Operands = append(ins.Operands, IrImmediate{0})
			case "label":
				ins.Operands = append(ins.Operands, IrLabel{"x"})
			case "text":
				ins.Operands = append(ins.Operands, IrText{"note"})
			}
		}
		p.Instructions = []IrInstruction{ins}
		if e := ValidateCoreProfile(p); e != nil {
			t.Fatal(literal, e)
		}
		for pos, role := range parts[2:] {
			for _, operand := range []IrOperand{IrRegister{0}, IrImmediate{0}, IrLabel{"x"}, IrText{"note"}} {
				clone := ins
				clone.Operands = append([]IrOperand{}, ins.Operands...)
				clone.Operands[pos] = operand
				p.Instructions = []IrInstruction{clone}
				admitted := false
				switch operand.(type) {
				case IrRegister:
					admitted = role == "register" || role == "register|immediate"
				case IrImmediate:
					admitted = role == "immediate" || role == "register|immediate"
				case IrLabel:
					admitted = role == "label"
				case IrText:
					admitted = role == "text"
				}
				if (ValidateCoreProfile(p) == nil) != admitted {
					t.Fatal("native operand role", literal, pos, operand)
				}
			}
		}
	}
	for _, payload := range []string{"", "tail ", "#id", "bad;value", "bad:value", "bad\tvalue", string([]byte{127}), strings.Repeat("a", 257)} {
		p := NewIrProgram("a")
		p.Instructions = []IrInstruction{{Opcode: OpComment, ID: -1, Operands: []IrOperand{IrText{payload}}}}
		if ValidateCoreProfile(p) == nil {
			t.Fatal("comment domain", payload)
		}
	}
	p := NewIrProgram("a")
	for i := 0; i < 256; i++ {
		p.Instructions = append(p.Instructions, IrInstruction{Opcode: OpComment, ID: -1, Operands: []IrOperand{IrText{strings.Repeat("a", 256)}}})
	}
	if ValidateCoreProfile(p) == nil {
		t.Fatal("printed byte budget")
	}
	for _, text := range []string{".version 1\n\n.entry a\n" + strings.Repeat("  NOP  ; #0\n", 257), ".version 1\n" + strings.Repeat("\n.data x 0 0\n", 33) + "\n.entry a\n", ".version 1\n\n.entry a\n  ADD v0,v1,v2,v3  ; #0\n"} {
		if _, e := ParseCoreProfile(text); e == nil {
			t.Fatal("preallocation budget")
		}
	}
	if (IrText{"note"}).String() != "note" {
		t.Fatal("text rendering")
	}
	// Legacy public helper error branches are part of package production coverage.
	if _, e := parseInstructionLine("", 1); e == nil {
		t.Fatal("empty legacy instruction")
	}
	if _, e := Parse(strings.Repeat("\n", maxLines+1)); e == nil {
		t.Fatal("legacy line budget")
	}
}

func FuzzParseCoreProfile(f *testing.F) {
	for _, seed := range []string{coreSparseGolden, "", ".version 1\n\n.entry a\n  COMMENT  ; #-1\n", ".version 1\n\n.entry a\n  COMMENT v0  ; #-1\n"} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, text string) {
		p, err := ParseCoreProfile(text)
		if err != nil {
			return
		}
		rendered, err := PrintCoreProfile(p)
		if err != nil || rendered != text {
			t.Fatal("accepted noncanonical text", err)
		}
	})
}

func TestGeneratedOperationOutcomes(t *testing.T) {
	// The existing generated pure outcome wrapper is shipped production code.
	// Check its observable result/error/panic contract instead of excluding it
	// from the measured package denominator.
	for _, mode := range []string{"success", "failure", "error", "unexpected", "panic"} {
		op := StartNew("unit", 7, func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("input", 3)
			if op.propertyBag["input"] != 3 {
				t.Fatal("property")
			}
			switch mode {
			case "success":
				return rf.Generate(true, false, 9)
			case "error":
				return rf.Fail(8, io.EOF)
			case "unexpected":
				return rf.Generate(false, true, 8)
			case "panic":
				panic("caught")
			default:
				return rf.Generate(false, false, 8)
			}
		})
		value, err := op.GetResult()
		if mode == "success" {
			if value != 9 || err != nil {
				t.Fatal(value, err)
			}
		} else if err == nil {
			t.Fatal("failure without error", mode)
		}
		if mode == "error" && err != io.EOF {
			t.Fatal("lost error")
		}
		if mode == "panic" && value != 7 {
			t.Fatal("lost fallback")
		}
	}
	func() {
		defer func() {
			if recover() != "again" {
				t.Error("repanic contract")
			}
		}()
		StartNew("unit", 0, func(*Operation[int], *ResultFactory[int]) *OperationResult[int] { panic("again") }).PanicOnUnexpected().GetResult()
	}()
	if !strings.Contains((&_capabilityViolationError{category: "x", action: "y", requested: "z"}).Error(), "x:y") {
		t.Fatal("diagnostic")
	}
}

func TestCoreMalformedCommentInstructionReturnsError(t *testing.T) {
	for _, line := range []string{"  COMMENT  ; #-1", "  COMMENT    v0  ; #-1", "  COMMENT    42  ; #-1", "  COMMENT    x, y  ; #-1", "  COMMENT    x  ; #-1"} {
		if _, err := ParseCoreProfile(".version 1\n\n.entry a\n" + line + "\n"); err == nil {
			t.Fatal("noncanonical opcode COMMENT accepted", line)
		}
	}
}
