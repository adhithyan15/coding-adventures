// decoder.go --- WASM bytecode decoder and control flow map builder.
//
// ════════════════════════════════════════════════════════════════════════
// THE DECODING PROBLEM
// ════════════════════════════════════════════════════════════��═══════════
//
// WASM bytecodes are variable-length.  GenericVM expects fixed-format
// Instruction objects.  The decoder bridges this gap by converting raw
// bytecodes into an array of Instruction objects with decoded operands.
//
// ════════════════════════════════════════════════════════════════════════
// CONTROL FLOW MAP
// ════════════════════════════════════════════════════════════════════════
//
// The decoder also builds the control flow map — a lookup table mapping
// each block/loop/if instruction index to its matching end (and else).
// This is built once per function via an O(n) pre-scan.
package wasmexecution

import (
	"encoding/binary"
	"math"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
	wasmopcodes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// DecodedInstruction holds a decoded WASM instruction with metadata.
type DecodedInstruction struct {
	Opcode  byte
	Operand interface{}
	Offset  int // byte offset in the original bytecodes
	Size    int // total bytes consumed
}

// DecodeFunctionBody decodes all instructions in a function body.
func DecodeFunctionBody(body *wasmtypes.FunctionBody) []DecodedInstruction {
	code := body.Code
	var instructions []DecodedInstruction
	offset := 0

	for offset < len(code) {
		startOffset := offset
		opcodeByte := code[offset]
		offset++

		info, found := wasmopcodes.GetOpcode(opcodeByte)
		var operand interface{}

		if found {
			var consumed int
			operand, consumed = decodeImmediates(code, offset, info.Immediates)
			offset += consumed
		}

		instructions = append(instructions, DecodedInstruction{
			Opcode:  opcodeByte,
			Operand: operand,
			Offset:  startOffset,
			Size:    offset - startOffset,
		})
	}

	return instructions
}

// decodeImmediates decodes the immediate operands for an instruction.
func decodeImmediates(code []byte, offset int, immediates []string) (interface{}, int) {
	if len(immediates) == 0 {
		return nil, 0
	}

	if len(immediates) == 1 {
		val, size := decodeSingleImmediate(code, offset, immediates[0])
		return val, size
	}

	// Multiple immediates — return as a map.
	result := make(map[string]interface{})
	totalSize := 0
	pos := offset
	for _, imm := range immediates {
		val, size := decodeSingleImmediate(code, pos, imm)
		result[imm] = val
		pos += size
		totalSize += size
	}
	return result, totalSize
}

// decodeSingleImmediate decodes one immediate and returns value + bytes consumed.
func decodeSingleImmediate(code []byte, offset int, immType string) (interface{}, int) {
	switch immType {
	case "i32":
		// Signed LEB128 for i32.const.
		val, consumed, _ := wasmleb128.DecodeSigned(code, offset)
		return int(val), consumed

	case "labelidx", "funcidx", "typeidx", "localidx", "globalidx", "tableidx", "memidx":
		// Unsigned LEB128 for indices.
		val, consumed, _ := wasmleb128.DecodeUnsigned(code, offset)
		return int(val), consumed

	case "i64":
		val, consumed, _ := decodeSigned64(code, offset)
		return val, consumed

	case "f32":
		if offset+4 > len(code) {
			return float32(0), 0
		}
		bits := binary.LittleEndian.Uint32(code[offset:])
		return math.Float32frombits(bits), 4

	case "f64":
		if offset+8 > len(code) {
			return float64(0), 0
		}
		bits := binary.LittleEndian.Uint64(code[offset:])
		return math.Float64frombits(bits), 8

	case "blocktype":
		b := code[offset]
		if b == 0x40 || b == 0x7F || b == 0x7E || b == 0x7D || b == 0x7C {
			return int(b), 1
		}
		// Type index (signed LEB128) for multi-value blocks.
		val, consumed, _ := wasmleb128.DecodeSigned(code, offset)
		return int(val), consumed

	case "memarg":
		align, alignSize, _ := wasmleb128.DecodeUnsigned(code, offset)
		memOffset, offsetSize, _ := wasmleb128.DecodeUnsigned(code, offset+alignSize)
		return map[string]interface{}{
			"align":  int(align),
			"offset": int(memOffset),
		}, alignSize + offsetSize

	case "vec_labelidx":
		count, countSize, _ := wasmleb128.DecodeUnsigned(code, offset)
		pos := offset + countSize
		labels := make([]int, count)
		for i := 0; i < int(count); i++ {
			label, labelSize, _ := wasmleb128.DecodeUnsigned(code, pos)
			labels[i] = int(label)
			pos += labelSize
		}
		defaultLabel, defaultSize, _ := wasmleb128.DecodeUnsigned(code, pos)
		pos += defaultSize
		return map[string]interface{}{
			"labels":       labels,
			"defaultLabel": int(defaultLabel),
		}, pos - offset

	default:
		return nil, 0
	}
}

// BuildControlFlowMap builds the control flow map for decoded instructions.
//
// Scans through all instructions and maps each block/loop/if start to
// its matching end (and else for if).  Uses a stack for nesting tracking.
func BuildControlFlowMap(instructions []DecodedInstruction) map[int]ControlTarget {
	cfMap := make(map[int]ControlTarget)

	type stackEntry struct {
		index  int
		opcode byte
		elsePc int
	}
	var stack []stackEntry

	for i, instr := range instructions {
		switch instr.Opcode {
		case 0x02, 0x03, 0x04: // block, loop, if
			stack = append(stack, stackEntry{index: i, opcode: instr.Opcode, elsePc: -1})

		case 0x05: // else
			if len(stack) > 0 {
				stack[len(stack)-1].elsePc = i
			}

		case 0x0B: // end
			if len(stack) > 0 {
				opener := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				cfMap[opener.index] = ControlTarget{
					EndPC:  i,
					ElsePC: opener.elsePc,
				}
			}
		}
	}

	return cfMap
}

// ToVMInstructions converts decoded instructions to GenericVM format.
func ToVMInstructions(decoded []DecodedInstruction) []vm.Instruction {
	result := make([]vm.Instruction, len(decoded))
	for i, d := range decoded {
		result[i] = vm.Instruction{
			Opcode:  vm.OpCode(d.Opcode),
			Operand: d.Operand,
		}
	}
	return result
}
