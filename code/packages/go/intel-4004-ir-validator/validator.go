package intel4004irvalidator

import (
	"fmt"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

const (
	maxRAMBytes         = 160
	maxCallDepth        = 2
	maxVirtualRegisters = 12
	minLoadImmediate    = 0
	maxLoadImmediate    = 255
)

type IrValidationError struct {
	Rule    string
	Message string
}

func (e IrValidationError) Error() string {
	return e.Message
}

func (e IrValidationError) String() string {
	return fmt.Sprintf("[%s] %s", e.Rule, e.Message)
}

type IrValidator struct{}

func (IrValidator) Validate(program *ir.IrProgram) []IrValidationError {
	errors := []IrValidationError{}
	errors = append(errors, checkNoWordOps(program)...)
	errors = append(errors, checkStaticRAM(program)...)
	errors = append(errors, checkCallDepth(program)...)
	errors = append(errors, checkRegisterCount(program)...)
	errors = append(errors, checkOperandRange(program)...)
	return errors
}

func checkNoWordOps(program *ir.IrProgram) []IrValidationError {
	errors := []IrValidationError{}
	sawLoad := false
	sawStore := false
	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLoadWord && !sawLoad {
			errors = append(errors, IrValidationError{Rule: "no_word_ops", Message: "LOAD_WORD is not supported on Intel 4004. Replace it with byte-sized accesses."})
			sawLoad = true
		}
		if instruction.Opcode == ir.OpStoreWord && !sawStore {
			errors = append(errors, IrValidationError{Rule: "no_word_ops", Message: "STORE_WORD is not supported on Intel 4004. Replace it with byte-sized accesses."})
			sawStore = true
		}
	}
	return errors
}

func checkStaticRAM(program *ir.IrProgram) []IrValidationError {
	total := 0
	for _, decl := range program.Data {
		total += decl.Size
	}
	if total <= maxRAMBytes {
		return nil
	}
	return []IrValidationError{{
		Rule:    "static_ram",
		Message: fmt.Sprintf("Static RAM usage %d bytes exceeds the Intel 4004 limit of %d bytes.", total, maxRAMBytes),
	}}
}

func checkCallDepth(program *ir.IrProgram) []IrValidationError {
	graph := map[string][]string{}
	currentLabel := ""
	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpLabel {
			if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
				currentLabel = label.Name
				if _, exists := graph[currentLabel]; !exists {
					graph[currentLabel] = nil
				}
			}
		}
		if instruction.Opcode == ir.OpCall && currentLabel != "" {
			if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
				graph[currentLabel] = append(graph[currentLabel], label.Name)
				if _, exists := graph[label.Name]; !exists {
					graph[label.Name] = nil
				}
			}
		}
	}

	if cycle := findCycle(graph); len(cycle) > 0 {
		return []IrValidationError{{
			Rule:    "call_depth",
			Message: fmt.Sprintf("Recursive call graphs are not supported on Intel 4004. Found cycle: %v.", cycle),
		}}
	}

	maxDepthSeen := 0
	var walk func(node string, depth int, visited map[string]bool) int
	walk = func(node string, depth int, visited map[string]bool) int {
		if visited[node] {
			return depth
		}
		nextVisited := map[string]bool{}
		for key, value := range visited {
			nextVisited[key] = value
		}
		nextVisited[node] = true
		children := graph[node]
		if len(children) == 0 {
			return depth
		}
		maxChild := depth
		for _, child := range children {
			childDepth := walk(child, depth+1, nextVisited)
			if childDepth > maxChild {
				maxChild = childDepth
			}
		}
		return maxChild
	}
	for label := range graph {
		if depth := walk(label, 0, map[string]bool{}); depth > maxDepthSeen {
			maxDepthSeen = depth
		}
	}
	if maxDepthSeen > maxCallDepth {
		return []IrValidationError{{
			Rule:    "call_depth",
			Message: fmt.Sprintf("Call graph depth %d exceeds the Intel 4004 hardware stack limit of %d nested calls.", maxDepthSeen, maxCallDepth),
		}}
	}
	return nil
}

func findCycle(graph map[string][]string) []string {
	visiting := map[string]bool{}
	visited := map[string]bool{}
	path := []string{}

	var visit func(node string) []string
	visit = func(node string) []string {
		if visiting[node] {
			start := -1
			for index, entry := range path {
				if entry == node {
					start = index
					break
				}
			}
			if start >= 0 {
				return append(append([]string{}, path[start:]...), node)
			}
			return []string{node, node}
		}
		if visited[node] {
			return nil
		}
		visiting[node] = true
		visited[node] = true
		path = append(path, node)
		for _, child := range graph[node] {
			if cycle := visit(child); len(cycle) > 0 {
				return cycle
			}
		}
		path = path[:len(path)-1]
		delete(visiting, node)
		return nil
	}

	for node := range graph {
		if cycle := visit(node); len(cycle) > 0 {
			return cycle
		}
	}
	return nil
}

func checkRegisterCount(program *ir.IrProgram) []IrValidationError {
	registers := map[int]bool{}
	for _, instruction := range program.Instructions {
		for _, operand := range instruction.Operands {
			if register, ok := operand.(ir.IrRegister); ok {
				registers[register.Index] = true
			}
		}
	}
	if len(registers) <= maxVirtualRegisters {
		return nil
	}
	return []IrValidationError{{
		Rule:    "register_count",
		Message: fmt.Sprintf("Program uses %d distinct virtual registers but Intel 4004 supports at most %d.", len(registers), maxVirtualRegisters),
	}}
}

func checkOperandRange(program *ir.IrProgram) []IrValidationError {
	errors := []IrValidationError{}
	for _, instruction := range program.Instructions {
		if instruction.Opcode != ir.OpLoadImm || len(instruction.Operands) < 2 {
			continue
		}
		immediate, ok := instruction.Operands[1].(ir.IrImmediate)
		if !ok {
			continue
		}
		if immediate.Value < minLoadImmediate || immediate.Value > maxLoadImmediate {
			errors = append(errors, IrValidationError{
				Rule:    "operand_range",
				Message: fmt.Sprintf("LOAD_IMM immediate %d is out of range for Intel 4004. Valid range is [%d, %d].", immediate.Value, minLoadImmediate, maxLoadImmediate),
			})
		}
	}
	return errors
}
