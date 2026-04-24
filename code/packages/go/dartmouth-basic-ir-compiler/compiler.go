package dartmouthbasicircompiler

import (
	"fmt"
	"math"
	"strconv"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	dartmouthbasicparser "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

const (
	syscallWriteByte = 1

	variableRegisterCount = 286
)

type BuildConfig struct {
	CharEncoding       string
	SyscallArgRegister int
	VariableBase       int
}

func ReleaseConfig() BuildConfig {
	return BuildConfig{
		CharEncoding:       "ascii",
		SyscallArgRegister: 4,
		VariableBase:       5,
	}
}

type CompileResult struct {
	Program *ir.IrProgram
	VarRegs map[string]int
}

type CompileError struct {
	Message string
}

func (e *CompileError) Error() string {
	return e.Message
}

type Compiler struct {
	config     BuildConfig
	idGen      *ir.IDGenerator
	program    *ir.IrProgram
	nextReg    int
	labelCount int
	varRegs    map[string]int
}

func CompileDartmouthBasic(ast *parser.ASTNode, config *BuildConfig) (*CompileResult, error) {
	if ast == nil {
		return nil, &CompileError{Message: "program AST is nil"}
	}
	if ast.RuleName != "program" {
		return nil, &CompileError{Message: fmt.Sprintf("expected program AST, got %q", ast.RuleName)}
	}

	cfg := ReleaseConfig()
	if config != nil {
		cfg = *config
	}
	if cfg.CharEncoding == "" {
		cfg.CharEncoding = "ascii"
	}
	if cfg.SyscallArgRegister < 0 {
		return nil, &CompileError{Message: "syscall arg register must be >= 0"}
	}
	if cfg.VariableBase <= cfg.SyscallArgRegister {
		return nil, &CompileError{Message: "variable base must live after the syscall arg register"}
	}

	compiler := &Compiler{
		config:  cfg,
		idGen:   ir.NewIDGenerator(),
		program: ir.NewIrProgram("_start"),
		nextReg: cfg.VariableBase + variableRegisterCount,
		varRegs: fixedVariableRegisters(cfg.VariableBase),
	}
	if err := compiler.compile(ast); err != nil {
		return nil, err
	}
	return &CompileResult{Program: compiler.program, VarRegs: compiler.varRegs}, nil
}

func CompileSource(source string, config *BuildConfig) (*CompileResult, error) {
	ast, err := dartmouthbasicparser.ParseDartmouthBasic(source)
	if err != nil {
		return nil, err
	}
	return CompileDartmouthBasic(ast, config)
}

func fixedVariableRegisters(variableBase int) map[string]int {
	result := map[string]int{}
	for letter := 0; letter < 26; letter++ {
		name := string(rune('A' + letter))
		result[name] = scalarRegister(variableBase, name)
		for digit := 0; digit < 10; digit++ {
			twoChar := fmt.Sprintf("%s%d", name, digit)
			result[twoChar] = scalarRegister(variableBase, twoChar)
		}
	}
	return result
}

func scalarRegister(variableBase int, name string) int {
	upper := strings.ToUpper(name)
	if len(upper) == 1 {
		return variableBase + int(upper[0]-'A')
	}
	letterIndex := int(upper[0] - 'A')
	digitIndex := int(upper[1] - '0')
	return variableBase + 26 + letterIndex*10 + digitIndex
}

func (c *Compiler) compile(ast *parser.ASTNode) error {
	c.emitLabel("_start")
	for _, child := range childNodes(ast) {
		if child.RuleName != "line" {
			continue
		}
		if err := c.compileLine(child); err != nil {
			return err
		}
	}
	c.emit(ir.OpHalt)
	return nil
}

func (c *Compiler) compileLine(node *parser.ASTNode) error {
	lineNumber := ""
	var statement *parser.ASTNode
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "LINE_NUM" {
				lineNumber = value.Value
			}
		case *parser.ASTNode:
			if value.RuleName == "statement" {
				statement = value
			}
		}
	}
	if lineNumber == "" {
		return nil
	}
	c.emitLabel("_line_" + lineNumber)
	if statement == nil {
		return nil
	}
	return c.compileStatement(statement)
}

func (c *Compiler) compileStatement(node *parser.ASTNode) error {
	for _, child := range childNodes(node) {
		switch child.RuleName {
		case "rem_stmt":
			c.emit(ir.OpComment, ir.IrLabel{Name: "REM"})
			return nil
		case "let_stmt":
			return c.compileLet(child)
		case "print_stmt":
			return c.compilePrint(child)
		case "goto_stmt":
			return c.compileGoto(child)
		case "if_stmt":
			return c.compileIf(child)
		case "end_stmt", "stop_stmt":
			c.emit(ir.OpHalt)
			return nil
		case "gosub_stmt", "return_stmt", "for_stmt", "next_stmt", "input_stmt",
			"read_stmt", "data_stmt", "restore_stmt", "dim_stmt", "def_stmt":
			return &CompileError{Message: fmt.Sprintf("%s is not supported in the Go Dartmouth BASIC RISC-V lane yet", strings.ToUpper(strings.TrimSuffix(child.RuleName, "_stmt")))}
		}
	}
	return nil
}

func (c *Compiler) compileLet(node *parser.ASTNode) error {
	var variableNode *parser.ASTNode
	var expressionNode *parser.ASTNode
	seenEquals := false
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "EQ" {
				seenEquals = true
			}
		case *parser.ASTNode:
			if value.RuleName == "variable" && !seenEquals && variableNode == nil {
				variableNode = value
			} else if isExpressionNode(value) && seenEquals && expressionNode == nil {
				expressionNode = value
			}
		}
	}
	if variableNode == nil || expressionNode == nil {
		return &CompileError{Message: "malformed LET statement"}
	}
	name, err := extractScalarVariableName(variableNode)
	if err != nil {
		return err
	}
	valueReg, err := c.compileExpr(expressionNode)
	if err != nil {
		return err
	}
	c.copyRegister(scalarRegister(c.config.VariableBase, name), valueReg)
	return nil
}

func (c *Compiler) compilePrint(node *parser.ASTNode) error {
	var printList *parser.ASTNode
	for _, child := range childNodes(node) {
		if child.RuleName == "print_list" {
			printList = child
			break
		}
	}
	if printList != nil {
		for _, item := range childNodes(printList) {
			if item.RuleName != "print_item" {
				continue
			}
			if err := c.compilePrintItem(item); err != nil {
				return err
			}
		}
	}
	c.emitPrintByte('\n')
	return nil
}

func (c *Compiler) compilePrintItem(node *parser.ASTNode) error {
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "STRING" {
				for _, r := range value.Value {
					if r > 127 {
						return &CompileError{Message: fmt.Sprintf("non-ASCII print character %q is not supported yet", r)}
					}
					c.emitPrintByte(byte(r))
				}
				return nil
			}
		case *parser.ASTNode:
			if isExpressionNode(value) {
				return &CompileError{Message: "numeric PRINT is not supported in the Go Dartmouth BASIC RISC-V lane yet"}
			}
		}
	}
	return nil
}

func (c *Compiler) compileGoto(node *parser.ASTNode) error {
	lineNumber, err := extractLineNumber(node)
	if err != nil {
		return err
	}
	c.emit(ir.OpJump, ir.IrLabel{Name: "_line_" + lineNumber})
	return nil
}

func (c *Compiler) compileIf(node *parser.ASTNode) error {
	var exprs []*parser.ASTNode
	var relop lexer.Token
	var haveRelop bool
	lineNumber := ""
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "NUMBER" {
				lineNumber = value.Value
			}
		case *parser.ASTNode:
			if value.RuleName == "relop" {
				token, ok := firstToken(value)
				if ok {
					relop = token
					haveRelop = true
				}
			} else if isExpressionNode(value) {
				exprs = append(exprs, value)
			}
		}
	}
	if len(exprs) < 2 || !haveRelop || lineNumber == "" {
		return &CompileError{Message: "malformed IF statement"}
	}
	leftReg, err := c.compileExpr(exprs[0])
	if err != nil {
		return err
	}
	rightReg, err := c.compileExpr(exprs[1])
	if err != nil {
		return err
	}
	conditionReg := c.newReg()
	target := ir.IrLabel{Name: "_line_" + lineNumber}
	switch relop.Value {
	case "<":
		c.emit(ir.OpCmpLt, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchNz, ir.IrRegister{Index: conditionReg}, target)
	case ">":
		c.emit(ir.OpCmpGt, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchNz, ir.IrRegister{Index: conditionReg}, target)
	case "=":
		c.emit(ir.OpCmpEq, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchNz, ir.IrRegister{Index: conditionReg}, target)
	case "<>":
		c.emit(ir.OpCmpNe, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchNz, ir.IrRegister{Index: conditionReg}, target)
	case "<=":
		c.emit(ir.OpCmpGt, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchZ, ir.IrRegister{Index: conditionReg}, target)
	case ">=":
		c.emit(ir.OpCmpLt, ir.IrRegister{Index: conditionReg}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		c.emit(ir.OpBranchZ, ir.IrRegister{Index: conditionReg}, target)
	default:
		return &CompileError{Message: fmt.Sprintf("unsupported IF relational operator %q", relop.Value)}
	}
	return nil
}

func (c *Compiler) compileExpr(node *parser.ASTNode) (int, error) {
	switch node.RuleName {
	case "expr":
		return c.compileAddChain(node)
	case "term":
		return c.compileMulChain(node)
	case "power":
		if len(node.Children) > 1 {
			return 0, &CompileError{Message: "power operator is not supported in the Go Dartmouth BASIC RISC-V lane yet"}
		}
		return c.compileExprNode(node.Children[0])
	case "unary":
		if len(node.Children) == 2 {
			if token, ok := node.Children[0].(lexer.Token); ok && tokenTypeName(token) == "MINUS" {
				valueReg, err := c.compileExprNode(node.Children[1])
				if err != nil {
					return 0, err
				}
				zeroReg := c.newImmediateRegister(0)
				result := c.newReg()
				c.emit(ir.OpSub, ir.IrRegister{Index: result}, ir.IrRegister{Index: zeroReg}, ir.IrRegister{Index: valueReg})
				return result, nil
			}
		}
		return c.compileExprNode(node.Children[0])
	case "primary":
		return c.compilePrimary(node)
	case "variable":
		name, err := extractScalarVariableName(node)
		if err != nil {
			return 0, err
		}
		return scalarRegister(c.config.VariableBase, name), nil
	default:
		if len(node.Children) == 1 {
			return c.compileExprNode(node.Children[0])
		}
	}
	return 0, &CompileError{Message: fmt.Sprintf("unsupported expression node %q", node.RuleName)}
}

func (c *Compiler) compileAddChain(node *parser.ASTNode) (int, error) {
	if len(node.Children) == 0 {
		return 0, &CompileError{Message: "empty expr node"}
	}
	leftReg, err := c.compileExprNode(node.Children[0])
	if err != nil {
		return 0, err
	}
	for index := 1; index < len(node.Children)-1; index += 2 {
		operator, ok := node.Children[index].(lexer.Token)
		if !ok {
			continue
		}
		rightReg, err := c.compileExprNode(node.Children[index+1])
		if err != nil {
			return 0, err
		}
		result := c.newReg()
		switch tokenTypeName(operator) {
		case "PLUS":
			c.emit(ir.OpAdd, ir.IrRegister{Index: result}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		case "MINUS":
			c.emit(ir.OpSub, ir.IrRegister{Index: result}, ir.IrRegister{Index: leftReg}, ir.IrRegister{Index: rightReg})
		default:
			return 0, &CompileError{Message: fmt.Sprintf("unsupported additive operator %q", operator.Value)}
		}
		leftReg = result
	}
	return leftReg, nil
}

func (c *Compiler) compileMulChain(node *parser.ASTNode) (int, error) {
	if len(node.Children) == 0 {
		return 0, &CompileError{Message: "empty term node"}
	}
	leftReg, err := c.compileExprNode(node.Children[0])
	if err != nil {
		return 0, err
	}
	for index := 1; index < len(node.Children)-1; index += 2 {
		operator, ok := node.Children[index].(lexer.Token)
		if !ok {
			continue
		}
		rightReg, err := c.compileExprNode(node.Children[index+1])
		if err != nil {
			return 0, err
		}
		switch tokenTypeName(operator) {
		case "STAR":
			leftReg = c.emitMultiply(leftReg, rightReg)
		case "SLASH":
			leftReg = c.emitDivide(leftReg, rightReg)
		default:
			return 0, &CompileError{Message: fmt.Sprintf("unsupported multiplicative operator %q", operator.Value)}
		}
	}
	return leftReg, nil
}

func (c *Compiler) compilePrimary(node *parser.ASTNode) (int, error) {
	if len(node.Children) == 0 {
		return 0, &CompileError{Message: "empty primary node"}
	}
	switch value := node.Children[0].(type) {
	case lexer.Token:
		if tokenTypeName(value) != "NUMBER" {
			return 0, &CompileError{Message: fmt.Sprintf("unsupported primary token %q", value.Value)}
		}
		literal, err := parseIntegerLiteral(value.Value)
		if err != nil {
			return 0, err
		}
		return c.newImmediateRegister(literal), nil
	case *parser.ASTNode:
		return c.compileExpr(value)
	default:
		return 0, &CompileError{Message: "unsupported primary expression"}
	}
}

func (c *Compiler) compileExprNode(node any) (int, error) {
	switch value := node.(type) {
	case *parser.ASTNode:
		return c.compileExpr(value)
	case lexer.Token:
		if tokenTypeName(value) == "NUMBER" {
			literal, err := parseIntegerLiteral(value.Value)
			if err != nil {
				return 0, err
			}
			return c.newImmediateRegister(literal), nil
		}
		return 0, &CompileError{Message: fmt.Sprintf("unexpected token %q in expression", value.Value)}
	default:
		return 0, &CompileError{Message: "unsupported expression fragment"}
	}
}

func (c *Compiler) emitMultiply(leftReg int, rightReg int) int {
	zero := c.newImmediateRegister(0)
	left := c.newReg()
	right := c.newReg()
	result := c.newImmediateRegister(0)
	leftNeg := c.newReg()
	rightNeg := c.newReg()
	sign := c.newReg()
	temp := c.newReg()

	c.copyRegister(left, leftReg)
	c.copyRegister(right, rightReg)

	leftPositive := c.newLabel("mul_left_positive")
	rightPositive := c.newLabel("mul_right_positive")
	loopLabel := c.newLabel("mul_loop")
	doneLabel := c.newLabel("mul_done")
	signedDone := c.newLabel("mul_signed_done")

	c.emit(ir.OpCmpLt, ir.IrRegister{Index: leftNeg}, ir.IrRegister{Index: left}, ir.IrRegister{Index: zero})
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: leftNeg}, ir.IrLabel{Name: leftPositive})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: left})
	c.copyRegister(left, temp)
	c.emitLabel(leftPositive)

	c.emit(ir.OpCmpLt, ir.IrRegister{Index: rightNeg}, ir.IrRegister{Index: right}, ir.IrRegister{Index: zero})
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: rightNeg}, ir.IrLabel{Name: rightPositive})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: right})
	c.copyRegister(right, temp)
	c.emitLabel(rightPositive)

	c.emit(ir.OpAdd, ir.IrRegister{Index: sign}, ir.IrRegister{Index: leftNeg}, ir.IrRegister{Index: rightNeg})
	c.emit(ir.OpAndImm, ir.IrRegister{Index: sign}, ir.IrRegister{Index: sign}, ir.IrImmediate{Value: 1})

	c.emitLabel(loopLabel)
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: right}, ir.IrLabel{Name: doneLabel})
	c.emit(ir.OpAdd, ir.IrRegister{Index: result}, ir.IrRegister{Index: result}, ir.IrRegister{Index: left})
	c.emit(ir.OpAddImm, ir.IrRegister{Index: right}, ir.IrRegister{Index: right}, ir.IrImmediate{Value: -1})
	c.emit(ir.OpJump, ir.IrLabel{Name: loopLabel})
	c.emitLabel(doneLabel)
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: sign}, ir.IrLabel{Name: signedDone})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: result})
	c.copyRegister(result, temp)
	c.emitLabel(signedDone)

	return result
}

func (c *Compiler) emitDivide(leftReg int, rightReg int) int {
	zero := c.newImmediateRegister(0)
	dividend := c.newReg()
	divisor := c.newReg()
	quotient := c.newImmediateRegister(0)
	leftNeg := c.newReg()
	rightNeg := c.newReg()
	sign := c.newReg()
	temp := c.newReg()
	compare := c.newReg()

	c.copyRegister(dividend, leftReg)
	c.copyRegister(divisor, rightReg)

	leftPositive := c.newLabel("div_left_positive")
	rightPositive := c.newLabel("div_right_positive")
	divZero := c.newLabel("div_zero")
	loopLabel := c.newLabel("div_loop")
	doneLabel := c.newLabel("div_done")
	finalLabel := c.newLabel("div_final")

	c.emit(ir.OpCmpLt, ir.IrRegister{Index: leftNeg}, ir.IrRegister{Index: dividend}, ir.IrRegister{Index: zero})
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: leftNeg}, ir.IrLabel{Name: leftPositive})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: dividend})
	c.copyRegister(dividend, temp)
	c.emitLabel(leftPositive)

	c.emit(ir.OpCmpLt, ir.IrRegister{Index: rightNeg}, ir.IrRegister{Index: divisor}, ir.IrRegister{Index: zero})
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: rightNeg}, ir.IrLabel{Name: rightPositive})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: divisor})
	c.copyRegister(divisor, temp)
	c.emitLabel(rightPositive)

	c.emit(ir.OpAdd, ir.IrRegister{Index: sign}, ir.IrRegister{Index: leftNeg}, ir.IrRegister{Index: rightNeg})
	c.emit(ir.OpAndImm, ir.IrRegister{Index: sign}, ir.IrRegister{Index: sign}, ir.IrImmediate{Value: 1})

	c.emit(ir.OpBranchZ, ir.IrRegister{Index: divisor}, ir.IrLabel{Name: divZero})
	c.emitLabel(loopLabel)
	c.emit(ir.OpCmpLt, ir.IrRegister{Index: compare}, ir.IrRegister{Index: dividend}, ir.IrRegister{Index: divisor})
	c.emit(ir.OpBranchNz, ir.IrRegister{Index: compare}, ir.IrLabel{Name: doneLabel})
	c.emit(ir.OpSub, ir.IrRegister{Index: dividend}, ir.IrRegister{Index: dividend}, ir.IrRegister{Index: divisor})
	c.emit(ir.OpAddImm, ir.IrRegister{Index: quotient}, ir.IrRegister{Index: quotient}, ir.IrImmediate{Value: 1})
	c.emit(ir.OpJump, ir.IrLabel{Name: loopLabel})

	c.emitLabel(divZero)
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: quotient}, ir.IrImmediate{Value: -1})
	c.emit(ir.OpJump, ir.IrLabel{Name: finalLabel})

	c.emitLabel(doneLabel)
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: sign}, ir.IrLabel{Name: finalLabel})
	c.emit(ir.OpSub, ir.IrRegister{Index: temp}, ir.IrRegister{Index: zero}, ir.IrRegister{Index: quotient})
	c.copyRegister(quotient, temp)
	c.emitLabel(finalLabel)

	return quotient
}

func (c *Compiler) emitPrintByte(value byte) {
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: c.config.SyscallArgRegister}, ir.IrImmediate{Value: int(value)})
	c.emit(ir.OpSyscall, ir.IrImmediate{Value: syscallWriteByte})
}

func (c *Compiler) copyRegister(destination int, source int) {
	c.emit(ir.OpAddImm, ir.IrRegister{Index: destination}, ir.IrRegister{Index: source}, ir.IrImmediate{Value: 0})
}

func (c *Compiler) newImmediateRegister(value int) int {
	register := c.newReg()
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: register}, ir.IrImmediate{Value: value})
	return register
}

func (c *Compiler) newReg() int {
	register := c.nextReg
	c.nextReg++
	return register
}

func (c *Compiler) newLabel(prefix string) string {
	label := fmt.Sprintf("__db_%s_%d", prefix, c.labelCount)
	c.labelCount++
	return label
}

func (c *Compiler) emit(opcode ir.IrOp, operands ...ir.IrOperand) int {
	id := c.idGen.Next()
	c.program.AddInstruction(ir.IrInstruction{Opcode: opcode, Operands: operands, ID: id})
	return id
}

func (c *Compiler) emitLabel(name string) {
	c.program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: name}}, ID: -1})
}

func extractScalarVariableName(node *parser.ASTNode) (string, error) {
	name := ""
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "NAME" && name == "" {
				name = strings.ToUpper(value.Value)
			}
		case *parser.ASTNode:
			if value.RuleName == "expr" {
				return "", &CompileError{Message: "array variables are not supported in the Go Dartmouth BASIC RISC-V lane yet"}
			}
		}
	}
	if name == "" {
		return "", &CompileError{Message: "could not extract scalar variable name"}
	}
	return name, nil
}

func extractLineNumber(node *parser.ASTNode) (string, error) {
	for _, child := range node.Children {
		if token, ok := child.(lexer.Token); ok && tokenTypeName(token) == "NUMBER" {
			return token.Value, nil
		}
	}
	return "", &CompileError{Message: "expected line number"}
}

func parseIntegerLiteral(value string) (int, error) {
	parsed, err := strconv.Atoi(value)
	if err == nil {
		return parsed, nil
	}
	floatValue, floatErr := strconv.ParseFloat(value, 64)
	if floatErr != nil {
		return 0, &CompileError{Message: fmt.Sprintf("numeric literal %q is not supported yet", value)}
	}
	if math.Trunc(floatValue) != floatValue {
		return 0, &CompileError{Message: fmt.Sprintf("non-integer numeric literal %q is not supported yet", value)}
	}
	return int(floatValue), nil
}

func tokenTypeName(token lexer.Token) string {
	if token.TypeName != "" {
		return token.TypeName
	}
	return strings.ToUpper(token.Type.String())
}

func childNodes(node *parser.ASTNode) []*parser.ASTNode {
	nodes := []*parser.ASTNode{}
	for _, child := range node.Children {
		if inner, ok := child.(*parser.ASTNode); ok {
			nodes = append(nodes, inner)
		}
	}
	return nodes
}

func firstToken(node *parser.ASTNode) (lexer.Token, bool) {
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			return value, true
		case *parser.ASTNode:
			if token, ok := firstToken(value); ok {
				return token, true
			}
		}
	}
	return lexer.Token{}, false
}

func isExpressionNode(node *parser.ASTNode) bool {
	switch node.RuleName {
	case "expr", "term", "power", "unary", "primary", "variable":
		return true
	default:
		return false
	}
}
