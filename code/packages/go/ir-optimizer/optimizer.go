package iroptimizer

import (
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

type IrPass interface {
	Name() string
	Run(program *ir.IrProgram) *ir.IrProgram
}

type OptimizationResult struct {
	Program                *ir.IrProgram
	PassesRun              []string
	InstructionsBefore     int
	InstructionsAfter      int
	InstructionsEliminated int
}

type IrOptimizer struct {
	passes []IrPass
}

func NewIrOptimizer(passes ...IrPass) *IrOptimizer {
	return &IrOptimizer{passes: passes}
}

func DefaultPasses() *IrOptimizer {
	return NewIrOptimizer(
		DeadCodeEliminator{},
		ConstantFolder{},
		PeepholeOptimizer{},
	)
}

func NoOp() *IrOptimizer {
	return NewIrOptimizer()
}

func (o *IrOptimizer) Optimize(program *ir.IrProgram) OptimizationResult {
	current := cloneProgram(program)
	passesRun := []string{}
	before := len(current.Instructions)
	for _, pass := range o.passes {
		current = pass.Run(current)
		passesRun = append(passesRun, pass.Name())
	}
	after := len(current.Instructions)
	return OptimizationResult{
		Program:                current,
		PassesRun:              passesRun,
		InstructionsBefore:     before,
		InstructionsAfter:      after,
		InstructionsEliminated: before - after,
	}
}

func cloneProgram(program *ir.IrProgram) *ir.IrProgram {
	next := ir.NewIrProgram(program.EntryLabel)
	next.Version = program.Version
	next.Data = append([]ir.IrDataDecl{}, program.Data...)
	next.Instructions = make([]ir.IrInstruction, len(program.Instructions))
	for index, instruction := range program.Instructions {
		operands := make([]ir.IrOperand, len(instruction.Operands))
		copy(operands, instruction.Operands)
		next.Instructions[index] = ir.IrInstruction{
			Opcode:   instruction.Opcode,
			Operands: operands,
			ID:       instruction.ID,
		}
	}
	return next
}

type DeadCodeEliminator struct{}

func (DeadCodeEliminator) Name() string { return "DeadCodeEliminator" }

func (DeadCodeEliminator) Run(program *ir.IrProgram) *ir.IrProgram {
	next := ir.NewIrProgram(program.EntryLabel)
	next.Version = program.Version
	next.Data = append([]ir.IrDataDecl{}, program.Data...)
	reachable := true
	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			reachable = true
		}
		if reachable {
			operands := append([]ir.IrOperand{}, instruction.Operands...)
			next.Instructions = append(next.Instructions, ir.IrInstruction{
				Opcode:   instruction.Opcode,
				Operands: operands,
				ID:       instruction.ID,
			})
		}
		if instruction.Opcode == ir.OpJump || instruction.Opcode == ir.OpRet || instruction.Opcode == ir.OpHalt {
			reachable = false
		}
	}
	return next
}

type ConstantFolder struct{}

func (ConstantFolder) Name() string { return "ConstantFolder" }

func (ConstantFolder) Run(program *ir.IrProgram) *ir.IrProgram {
	next := cloneProgram(program)
	pending := map[int]int{}
	for index, instruction := range next.Instructions {
		if instruction.Opcode == ir.OpLabel {
			pending = map[int]int{}
			continue
		}

		if instruction.Opcode == ir.OpLoadImm {
			if len(instruction.Operands) >= 2 {
				if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
					if imm, ok := instruction.Operands[1].(ir.IrImmediate); ok {
						pending[dest.Index] = imm.Value
					}
				}
			}
			continue
		}

		if instruction.Opcode == ir.OpAddImm || instruction.Opcode == ir.OpAndImm {
			if len(instruction.Operands) == 3 {
				dest, destOK := instruction.Operands[0].(ir.IrRegister)
				src, srcOK := instruction.Operands[1].(ir.IrRegister)
				immediate, immOK := instruction.Operands[2].(ir.IrImmediate)
				base, hasBase := pending[src.Index]
				if destOK && srcOK && immOK && hasBase && dest.Index == src.Index {
					value := base
					if instruction.Opcode == ir.OpAddImm {
						value += immediate.Value
					} else {
						value &= immediate.Value
					}
					next.Instructions[index] = ir.IrInstruction{
						Opcode:   ir.OpLoadImm,
						Operands: []ir.IrOperand{dest, ir.IrImmediate{Value: value}},
						ID:       instruction.ID,
					}
					pending[dest.Index] = value
					continue
				}
			}
		}

		if len(instruction.Operands) > 0 {
			if dest, ok := instruction.Operands[0].(ir.IrRegister); ok {
				delete(pending, dest.Index)
			}
		}
	}
	return next
}

type PeepholeOptimizer struct{}

func (PeepholeOptimizer) Name() string { return "PeepholeOptimizer" }

func (PeepholeOptimizer) Run(program *ir.IrProgram) *ir.IrProgram {
	next := cloneProgram(program)
	out := []ir.IrInstruction{}
	for index := 0; index < len(next.Instructions); index++ {
		current := next.Instructions[index]
		if index+1 < len(next.Instructions) {
			merged, ok := tryMerge(current, next.Instructions[index+1])
			if ok {
				out = append(out, merged)
				index++
				continue
			}
		}
		out = append(out, current)
	}
	next.Instructions = out
	return next
}

func tryMerge(current ir.IrInstruction, next ir.IrInstruction) (ir.IrInstruction, bool) {
	if current.Opcode == ir.OpAddImm && next.Opcode == ir.OpAddImm && len(current.Operands) == 3 && len(next.Operands) == 3 {
		cDest, cDestOK := current.Operands[0].(ir.IrRegister)
		cSrc, cSrcOK := current.Operands[1].(ir.IrRegister)
		cImm, cImmOK := current.Operands[2].(ir.IrImmediate)
		nDest, nDestOK := next.Operands[0].(ir.IrRegister)
		nSrc, nSrcOK := next.Operands[1].(ir.IrRegister)
		nImm, nImmOK := next.Operands[2].(ir.IrImmediate)
		if cDestOK && cSrcOK && cImmOK && nDestOK && nSrcOK && nImmOK && cDest.Index == cSrc.Index && nDest.Index == nSrc.Index && cDest.Index == nDest.Index {
			return ir.IrInstruction{
				Opcode: ir.OpAddImm,
				Operands: []ir.IrOperand{
					cDest,
					cSrc,
					ir.IrImmediate{Value: cImm.Value + nImm.Value},
				},
				ID: current.ID,
			}, true
		}
	}
	return ir.IrInstruction{}, false
}
