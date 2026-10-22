package compilerir

import (
	"fmt"
	"regexp"
	"strings"
)

// IrText is inert COMMENT data in core-text-v1. Keeping a distinct operand
// avoids silently treating an execution label as comment text. Legacy callers
// can continue to use IrLabel with the broad Parse/Print APIs.
type IrText struct{ Value string }

func (t IrText) operandTag()    {}
func (t IrText) String() string { return t.Value }

// The core is an explicit role table, separate from enum numbering. Memory's
// final slot accepts a register OR an immediate; metadata is not an ID policy.
var coreRoles = map[IrOp]string{
	OpLoadImm: "ri", OpLoadAddr: "rl", OpLoadByte: "rrm", OpStoreByte: "rrm", OpLoadWord: "rrm", OpStoreWord: "rrm",
	OpAdd: "rrr", OpAddImm: "rri", OpSub: "rrr", OpAnd: "rrr", OpAndImm: "rri",
	OpCmpEq: "rrr", OpCmpNe: "rrr", OpCmpLt: "rrr", OpCmpGt: "rrr",
	OpLabel: "l", OpJump: "l", OpBranchZ: "rl", OpBranchNz: "rl", OpCall: "l", OpRet: "",
	OpSyscall: "i", OpHalt: "", OpNop: "", OpComment: "t",
}
var coreName = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]{0,63}$`)
var coreRegisterName = regexp.MustCompile(`^v[0-9]+$`)

func coreValidName(s string) bool {
	return len(s) <= 64 && coreName.MatchString(s) && !coreRegisterName.MatchString(s)
}
func coreValidText(s string) bool {
	if len(s) < 1 || len(s) > 256 || s[0] == ' ' || s[len(s)-1] == ' ' || s[0] == '#' {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < 32 || s[i] > 126 || s[i] == ';' || s[i] == ':' {
			return false
		}
	}
	return true
}

// ValidateCoreProfile checks the admitted data component without mutating it.
// Bounds precede loops or rendering. Only value operands are admitted, so nil,
// typed-nil and arbitrary pointer method sets never reach String or indexing.
func ValidateCoreProfile(p *IrProgram) error {
	if p == nil {
		return fmt.Errorf("core: nil program")
	}
	if p.Version != 1 || !coreValidName(p.EntryLabel) {
		return fmt.Errorf("core: version or entry")
	}
	if len(p.Data) > 32 || len(p.Instructions) > 256 {
		return fmt.Errorf("core: program budget")
	}
	dataNames := map[string]bool{}
	for _, d := range p.Data {
		if !coreValidName(d.Label) || dataNames[d.Label] || d.Size < 0 || d.Size > 65535 || d.Init < 0 || d.Init > 255 {
			return fmt.Errorf("core: data declaration")
		}
		dataNames[d.Label] = true
	}
	ids, labels := map[int]bool{}, map[string]bool{}
	for n, ins := range p.Instructions {
		roles, ok := coreRoles[ins.Opcode]
		if !ok || len(ins.Operands) != len(roles) {
			return fmt.Errorf("core: instruction %d opcode/arity", n)
		}
		if ins.Opcode == OpLabel || ins.Opcode == OpComment {
			if ins.ID != -1 {
				return fmt.Errorf("core: metadata ID")
			}
		} else {
			if ins.ID < 0 || int64(ins.ID) > 2147483647 || ids[ins.ID] {
				return fmt.Errorf("core: ordinary ID")
			}
			ids[ins.ID] = true
		}
		for i, operand := range ins.Operands {
			role := roles[i]
			valid := false
			switch x := operand.(type) {
			case IrRegister:
				valid = (role == 'r' || role == 'm') && x.Index >= 0 && x.Index <= 65535
			case IrImmediate:
				valid = (role == 'i' || role == 'm') && int64(x.Value) >= -2147483648 && int64(x.Value) <= 2147483647
			case IrLabel:
				valid = role == 'l' && coreValidName(x.Name)
			case IrText:
				valid = role == 't' && coreValidText(x.Value)
			}
			if !valid {
				return fmt.Errorf("core: instruction %d operand %d", n, i)
			}
		}
		if ins.Opcode == OpLabel {
			name := ins.Operands[0].(IrLabel).Name
			if labels[name] {
				return fmt.Errorf("core: duplicate label")
			}
			labels[name] = true
		}
	}
	// The structurally bounded representation renders at most about72KiB, even
	// at the largest per-comment size. Enforce the stricter frozen64KiB limit.
	if len(coreRender(p)) > 65536 {
		return fmt.Errorf("core: printed text budget")
	}
	return nil
}

// coreRender is called only after structural admission. It deliberately does
// not share legacy Print's padded zero-operand spelling or broad comment tags.
func coreRender(p *IrProgram) string {
	var b strings.Builder
	b.WriteString(".version 1\n")
	for _, d := range p.Data {
		fmt.Fprintf(&b, "\n.data %s %d %d\n", d.Label, d.Size, d.Init)
	}
	fmt.Fprintf(&b, "\n.entry %s\n", p.EntryLabel)
	for _, ins := range p.Instructions {
		switch ins.Opcode {
		case OpLabel:
			fmt.Fprintf(&b, "\n%s:\n", ins.Operands[0].(IrLabel).Name)
		case OpComment:
			fmt.Fprintf(&b, "  ; %s\n", ins.Operands[0].(IrText).Value)
		default:
			b.WriteString("  ")
			if len(ins.Operands) == 0 {
				b.WriteString(ins.Opcode.String())
			} else {
				fmt.Fprintf(&b, "%-11s", ins.Opcode.String())
				for i, a := range ins.Operands {
					if i > 0 {
						b.WriteString(", ")
					}
					b.WriteString(a.String())
				}
			}
			fmt.Fprintf(&b, "  ; #%d\n", ins.ID)
		}
	}
	return b.String()
}

// PrintCoreProfile returns exact canonical bytes or a controlled validation
// error. It never calls the permissive printer on unvalidated data.
func PrintCoreProfile(p *IrProgram) (string, error) {
	if err := ValidateCoreProfile(p); err != nil {
		return "", err
	}
	return coreRender(p), nil
}

// ParseCoreProfile uses a bounded legacy parser as an internal lexer, then
// constructs genuine text operands in production and validates the whole
// program. Byte equality rejects every permissive trim/default/discard alias.
func ParseCoreProfile(text string) (*IrProgram, error) {
	if len(text) > 65536 {
		return nil, fmt.Errorf("core: source budget")
	}
	for i := 0; i < len(text); i++ {
		if text[i] != '\n' && (text[i] < 32 || text[i] > 126) {
			return nil, fmt.Errorf("core: ASCII/LF required")
		}
	}
	if strings.Count(text, "\n") > 1024 {
		return nil, fmt.Errorf("core: line budget")
	}
	// Count candidate records before the broad parser materializes slices.
	// Canonical equality below still owns spelling/order; this pass owns work.
	declarations, instructions := 0, 0
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSpace(line)
		switch {
		case line == "", strings.HasPrefix(line, ".version"), strings.HasPrefix(line, ".entry"):
			continue
		case strings.HasPrefix(line, ".data"):
			declarations++
		default:
			instructions++
			if !strings.HasPrefix(line, ";") {
				body, _, _ := strings.Cut(line, ";")
				if strings.Count(body, ",") > 2 {
					return nil, fmt.Errorf("core: operand budget")
				}
			}
		}
		if declarations > 32 || instructions > 256 {
			return nil, fmt.Errorf("core: record budget")
		}
	}
	p, err := Parse(text)
	if err != nil {
		return nil, err
	}
	// Normalize only native empty-vector representation, never semantic values.
	if p.Data == nil {
		p.Data = []IrDataDecl{}
	}
	if p.Instructions == nil {
		p.Instructions = []IrInstruction{}
	}
	for i := range p.Instructions {
		ins := &p.Instructions[i]
		if ins.Operands == nil {
			ins.Operands = []IrOperand{}
		}
		if ins.Opcode == OpComment {
			// Legacy Parse also recognizes COMMENT as an ordinary opcode. Such
			// a line may have no operand, or a numeric/register operand. Admit
			// the projection only after safe arity and dynamic-type checks;
			// canonical equality subsequently rejects opcode-form comments.
			if len(ins.Operands) != 1 {
				return nil, fmt.Errorf("core: comment arity")
			}
			label, ok := ins.Operands[0].(IrLabel)
			if !ok {
				return nil, fmt.Errorf("core: comment operand")
			}
			ins.Operands = []IrOperand{IrText{label.Name}}
		}
	}
	canonical, err := PrintCoreProfile(p)
	if err != nil {
		return nil, err
	}
	if canonical != text {
		return nil, fmt.Errorf("core: noncanonical text")
	}
	return p, nil
}

// CoreIDSequence models a checked suffix allocation independently of legacy
// IDGenerator. All checks precede allocation, so failure has no partial values
// and returns the unchanged supplied start. int64 preserves exhausted next.
func CoreIDSequence(start int64, count int) ([]int64, int64, error) {
	if start < 0 || start > 2147483647 || count < 0 || count > 256 || int64(count) > 2147483648-start {
		return nil, start, fmt.Errorf("core: ID allocation domain")
	}
	values := make([]int64, count)
	for i := range values {
		values[i] = start + int64(i)
	}
	return values, start + int64(count), nil
}
