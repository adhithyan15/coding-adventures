package irtointel4004compiler

import (
	"strconv"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	validator "github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-ir-validator"
)

type IrToIntel4004Compiler struct {
	Validator validator.IrValidator
}

func NewIrToIntel4004Compiler() *IrToIntel4004Compiler {
	return &IrToIntel4004Compiler{Validator: validator.IrValidator{}}
}

func (c *IrToIntel4004Compiler) Compile(program *ir.IrProgram) (string, error) {
	errors := c.Validator.Validate(program)
	if len(errors) > 0 {
		lines := make([]string, len(errors))
		for index, err := range errors {
			lines[index] = err.String()
		}
		return "", validator.IrValidationError{Rule: errors[0].Rule, Message: strings.Join(lines, "\n")}
	}
	return generate(program), nil
}

func generate(program *ir.IrProgram) string {
	lines := []string{"    ORG 0x000"}
	for _, instruction := range program.Instructions {
		lines = append(lines, emitInstruction(instruction)...)
	}
	return strings.Join(lines, "\n") + "\n"
}

func emitInstruction(instruction ir.IrInstruction) []string {
	switch instruction.Opcode {
	case ir.OpLabel:
		if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
			return []string{label.Name + ":"}
		}
	case ir.OpLoadImm:
		if len(instruction.Operands) >= 2 {
			if register, ok := instruction.Operands[0].(ir.IrRegister); ok {
				if immediate, ok := instruction.Operands[1].(ir.IrImmediate); ok {
					if immediate.Value <= 15 {
						return []string{"    LDM " + strconv.Itoa(immediate.Value), "    XCH " + preg(register.Index)}
					}
					return []string{"    FIM " + pair(register.Index) + ", " + strconv.Itoa(immediate.Value)}
				}
			}
		}
	case ir.OpLoadAddr:
		if len(instruction.Operands) >= 2 {
			if register, ok := instruction.Operands[0].(ir.IrRegister); ok {
				if label, ok := instruction.Operands[1].(ir.IrLabel); ok {
					return []string{"    FIM " + pair(register.Index) + ", " + label.Name}
				}
			}
		}
	case ir.OpLoadByte:
		if len(instruction.Operands) >= 2 {
			if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
				if base, ok := instruction.Operands[1].(ir.IrRegister); ok {
					return []string{"    SRC " + pair(base.Index), "    RDM", "    XCH " + preg(dest.Index)}
				}
			}
		}
	case ir.OpStoreByte:
		if len(instruction.Operands) >= 2 {
			if src, ok := instruction.Operands[0].(ir.IrRegister); ok {
				if base, ok := instruction.Operands[1].(ir.IrRegister); ok {
					return []string{"    LD " + preg(src.Index), "    SRC " + pair(base.Index), "    WRM"}
				}
			}
		}
	case ir.OpAdd:
		return emitThreeRegister("ADD", instruction.Operands)
	case ir.OpSub:
		return emitThreeRegister("SUB", instruction.Operands)
	case ir.OpAnd:
		return emitThreeRegister("AND", instruction.Operands)
	case ir.OpAddImm:
		if len(instruction.Operands) == 3 {
			if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
				if src, ok := instruction.Operands[1].(ir.IrRegister); ok {
					if immediate, ok := instruction.Operands[2].(ir.IrImmediate); ok {
						if immediate.Value == 0 {
							return []string{"    LD " + preg(src.Index), "    XCH " + preg(dest.Index)}
						}
						if immediate.Value <= 15 {
							scratch := "R1"
							if src.Index == 1 {
								scratch = "R14"
							}
							return []string{
								"    LDM " + strconv.Itoa(immediate.Value),
								"    XCH " + scratch,
								"    LD " + preg(src.Index),
								"    ADD " + scratch,
								"    XCH " + preg(dest.Index),
							}
						}
						return []string{
							"    FIM P7, " + strconv.Itoa(immediate.Value),
							"    LD " + preg(src.Index),
							"    ADD R14",
							"    XCH " + preg(dest.Index),
						}
					}
				}
			}
		}
	case ir.OpAndImm:
		if len(instruction.Operands) == 3 {
			if immediate, ok := instruction.Operands[2].(ir.IrImmediate); ok {
				if immediate.Value == 255 {
					return []string{"    ; AND_IMM 255 is a no-op on 4004 (8-bit pair)"}
				}
				if immediate.Value == 15 {
					return []string{"    ; AND_IMM 15 is a no-op on 4004 (4-bit register)"}
				}
				return []string{"    ; AND_IMM " + strconv.Itoa(immediate.Value) + " is unsupported on 4004"}
			}
		}
	case ir.OpCmpEq:
		if len(instruction.Operands) == 3 {
			if left, ok := instruction.Operands[1].(ir.IrRegister); ok {
				if right, ok := instruction.Operands[2].(ir.IrRegister); ok {
					if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
						return []string{
							"    LD " + preg(left.Index),
							"    SUB " + preg(right.Index),
							"    CMA",
							"    IAC",
							"    XCH " + preg(dest.Index),
						}
					}
				}
			}
		}
	case ir.OpCmpLt:
		if len(instruction.Operands) == 3 {
			if left, ok := instruction.Operands[1].(ir.IrRegister); ok {
				if right, ok := instruction.Operands[2].(ir.IrRegister); ok {
					if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
						return []string{
							"    LD " + preg(left.Index),
							"    SUB " + preg(right.Index),
							"    TCS",
							"    XCH " + preg(dest.Index),
						}
					}
				}
			}
		}
	case ir.OpCmpNe, ir.OpCmpGt:
		return []string{"    ; unsupported compare opcode on 4004"}
	case ir.OpJump:
		if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
			return []string{"    JUN " + label.Name}
		}
	case ir.OpBranchZ:
		return emitBranch("0x4", instruction.Operands)
	case ir.OpBranchNz:
		return emitBranch("0xC", instruction.Operands)
	case ir.OpCall:
		if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
			return []string{"    JMS " + label.Name}
		}
	case ir.OpRet:
		return []string{"    BBL 0"}
	case ir.OpHalt:
		return []string{"    HLT"}
	case ir.OpNop:
		return []string{"    NOP"}
	case ir.OpComment:
		if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
			return []string{"    ; " + label.Name}
		}
		return []string{"    ;"}
	}
	return []string{"    ; unsupported opcode"}
}

func emitThreeRegister(mnemonic string, operands []ir.IrOperand) []string {
	if len(operands) != 3 {
		return []string{"    ; " + mnemonic + ": invalid operands"}
	}
	dest, destOK := operands[0].(ir.IrRegister)
	left, leftOK := operands[1].(ir.IrRegister)
	right, rightOK := operands[2].(ir.IrRegister)
	if !destOK || !leftOK || !rightOK {
		return []string{"    ; " + mnemonic + ": invalid operands"}
	}
	return []string{"    LD " + preg(left.Index), "    " + mnemonic + " " + preg(right.Index), "    XCH " + preg(dest.Index)}
}

func emitBranch(condition string, operands []ir.IrOperand) []string {
	if len(operands) != 2 {
		return []string{"    ; BRANCH: invalid operands"}
	}
	register, registerOK := operands[0].(ir.IrRegister)
	label, labelOK := operands[1].(ir.IrLabel)
	if !registerOK || !labelOK {
		return []string{"    ; BRANCH: invalid operands"}
	}
	return []string{"    LD " + preg(register.Index), "    JCN " + condition + ", " + label.Name}
}

func preg(index int) string {
	return "R" + strconv.Itoa(index)
}

func pair(index int) string {
	return "P" + strconv.Itoa(index/2)
}
