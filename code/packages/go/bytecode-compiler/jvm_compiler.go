package bytecodecompiler

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

const (
	ICONST_0 = 0x03
	ICONST_1 = 0x04
	ICONST_2 = 0x05
	ICONST_3 = 0x06
	ICONST_4 = 0x07
	ICONST_5 = 0x08
	BIPUSH   = 0x10
	LDC      = 0x12
	ILOAD    = 0x15
	ILOAD_0  = 0x1A
	ILOAD_1  = 0x1B
	ILOAD_2  = 0x1C
	ILOAD_3  = 0x1D
	ISTORE   = 0x36
	ISTORE_0 = 0x3B
	ISTORE_1 = 0x3C
	ISTORE_2 = 0x3D
	ISTORE_3 = 0x3E
	POP      = 0x57
	IADD     = 0x60
	ISUB     = 0x64
	IMUL     = 0x68
	IDIV     = 0x6C
	RETURN   = 0xB1
)

var JvmOperatorMap = map[string]int{
	"+": IADD,
	"-": ISUB,
	"*": IMUL,
	"/": IDIV,
}

type JVMCodeObject struct {
	Bytecode   []byte
	Constants  []interface{}
	NumLocals  int
	LocalNames []string
}

type JVMCompiler struct {
	Bytecode  []byte
	Constants []interface{}
	Locals    []string
}

func NewJVMCompiler() *JVMCompiler {
	result, _ := StartNew[*JVMCompiler]("bytecode-compiler.NewJVMCompiler", nil,
		func(_ *Operation[*JVMCompiler], rf *ResultFactory[*JVMCompiler]) *OperationResult[*JVMCompiler] {
			return rf.Generate(true, false, &JVMCompiler{
				Bytecode:  []byte{},
				Constants: []interface{}{},
				Locals:    []string{},
			})
		}).GetResult()
	return result
}

func (c *JVMCompiler) Compile(program parser.Program) JVMCodeObject {
	result, _ := StartNew[JVMCodeObject]("bytecode-compiler.JVMCompile", JVMCodeObject{},
		func(_ *Operation[JVMCodeObject], rf *ResultFactory[JVMCodeObject]) *OperationResult[JVMCodeObject] {
			for _, stmt := range program.Statements {
				c.compileStatement(stmt)
			}
			c.Bytecode = append(c.Bytecode, RETURN)
			return rf.Generate(true, false, JVMCodeObject{
				Bytecode:   c.Bytecode,
				Constants:  c.Constants,
				NumLocals:  len(c.Locals),
				LocalNames: c.Locals,
			})
		}).GetResult()
	return result
}

func (c *JVMCompiler) compileStatement(stmt parser.Statement) {
	switch n := stmt.(type) {
	case parser.Assignment:
		c.compileAssignment(n)
	case parser.ExpressionStmt:
		c.compileExpression(n.Expression)
		c.Bytecode = append(c.Bytecode, POP)
	default:
		panic(fmt.Sprintf("Unknown statement type: %T", stmt))
	}
}

func (c *JVMCompiler) compileAssignment(node parser.Assignment) {
	c.compileExpression(node.Value)
	slot := c.getLocalSlot(node.Target.Name)
	c.emitIstore(slot)
}

func (c *JVMCompiler) compileExpression(expr parser.Expression) {
	switch n := expr.(type) {
	case parser.NumberLiteral:
		c.emitNumber(n.Value)
	case parser.StringLiteral:
		idx := c.addConstant(n.Value)
		c.Bytecode = append(c.Bytecode, LDC)
		c.Bytecode = append(c.Bytecode, byte(idx))
	case parser.Name:
		slot := c.getLocalSlot(n.Name)
		c.emitIload(slot)
	case parser.BinaryOp:
		c.compileExpression(n.Left)
		c.compileExpression(n.Right)
		opcode, ok := JvmOperatorMap[n.Op]
		if !ok {
			panic(fmt.Sprintf("Unknown operator: %s", n.Op))
		}
		c.Bytecode = append(c.Bytecode, byte(opcode))
	default:
		panic(fmt.Sprintf("Unknown expression type: %T", expr))
	}
}

func (c *JVMCompiler) emitNumber(value int) {
	if value >= 0 && value <= 5 {
		c.Bytecode = append(c.Bytecode, byte(ICONST_0+value))
	} else if value >= -128 && value <= 127 {
		c.Bytecode = append(c.Bytecode, BIPUSH)
		c.Bytecode = append(c.Bytecode, byte(value&0xFF))
	} else {
		idx := c.addConstant(value)
		c.Bytecode = append(c.Bytecode, LDC)
		c.Bytecode = append(c.Bytecode, byte(idx))
	}
}

func (c *JVMCompiler) emitIstore(slot int) {
	if slot <= 3 {
		c.Bytecode = append(c.Bytecode, byte(ISTORE_0+slot))
	} else {
		c.Bytecode = append(c.Bytecode, ISTORE)
		c.Bytecode = append(c.Bytecode, byte(slot))
	}
}

func (c *JVMCompiler) emitIload(slot int) {
	if slot <= 3 {
		c.Bytecode = append(c.Bytecode, byte(ILOAD_0+slot))
	} else {
		c.Bytecode = append(c.Bytecode, ILOAD)
		c.Bytecode = append(c.Bytecode, byte(slot))
	}
}

func (c *JVMCompiler) addConstant(value interface{}) int {
	for i, v := range c.Constants {
		if v == value {
			return i
		}
	}
	c.Constants = append(c.Constants, value)
	return len(c.Constants) - 1
}

func (c *JVMCompiler) getLocalSlot(name string) int {
	for i, n := range c.Locals {
		if n == name {
			return i
		}
	}
	c.Locals = append(c.Locals, name)
	return len(c.Locals) - 1
}
