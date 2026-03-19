package bytecodecompiler

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

var OperatorMap = map[string]vm.OpCode{
	"+": vm.OpAdd,
	"-": vm.OpSub,
	"*": vm.OpMul,
	"/": vm.OpDiv,
}

type BytecodeCompiler struct {
	Instructions []vm.Instruction
	Constants    []interface{}
	Names        []string
}

func NewBytecodeCompiler() *BytecodeCompiler {
	return &BytecodeCompiler{
		Instructions: []vm.Instruction{},
		Constants:    []interface{}{},
		Names:        []string{},
	}
}

func (c *BytecodeCompiler) Compile(program parser.Program) vm.CodeObject {
	for _, stmt := range program.Statements {
		c.compileStatement(stmt)
	}
	c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpHalt})

	return vm.CodeObject{
		Instructions: c.Instructions,
		Constants:    c.Constants,
		Names:        c.Names,
	}
}

func (c *BytecodeCompiler) compileStatement(stmt parser.Statement) {
	switch n := stmt.(type) {
	case parser.Assignment:
		c.compileAssignment(n)
	case parser.ExpressionStmt:
		c.compileExpression(n.Expression)
		c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpPop})
	default:
		panic(fmt.Sprintf("Unknown statement type: %T", stmt))
	}
}

func (c *BytecodeCompiler) compileAssignment(node parser.Assignment) {
	c.compileExpression(node.Value)
	nameIndex := c.addName(node.Target.Name)
	c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpStoreName, Operand: nameIndex})
}

func (c *BytecodeCompiler) compileExpression(expr parser.Expression) {
	switch n := expr.(type) {
	case parser.NumberLiteral:
		idx := c.addConstant(n.Value)
		c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpLoadConst, Operand: idx})
	case parser.StringLiteral:
		idx := c.addConstant(n.Value)
		c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpLoadConst, Operand: idx})
	case parser.Name:
		idx := c.addName(n.Name)
		c.Instructions = append(c.Instructions, vm.Instruction{Opcode: vm.OpLoadName, Operand: idx})
	case parser.BinaryOp:
		c.compileExpression(n.Left)
		c.compileExpression(n.Right)
		opcode, ok := OperatorMap[n.Op]
		if !ok {
			panic(fmt.Sprintf("Unknown operator: %s", n.Op))
		}
		c.Instructions = append(c.Instructions, vm.Instruction{Opcode: opcode})
	default:
		panic(fmt.Sprintf("Unknown expression type: %T", expr))
	}
}

func (c *BytecodeCompiler) addConstant(value interface{}) int {
	for i, v := range c.Constants {
		if v == value {
			return i
		}
	}
	c.Constants = append(c.Constants, value)
	return len(c.Constants) - 1
}

func (c *BytecodeCompiler) addName(name string) int {
	for i, n := range c.Names {
		if n == name {
			return i
		}
	}
	c.Names = append(c.Names, name)
	return len(c.Names) - 1
}
