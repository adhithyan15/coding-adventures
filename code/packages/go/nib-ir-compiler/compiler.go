package nibircompiler

import (
	"strconv"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

const (
	regZero                  = 0
	regScratch               = 1
	regArgBase               = 2
	defaultLocalRegisterBase = 8
)

type CompileResult struct {
	Program *ir.IrProgram
}

type Compiler struct {
	config        BuildConfig
	typedAST      *nibtypechecker.TypedAST
	idGen         *ir.IDGenerator
	program       *ir.IrProgram
	loopCount     int
	ifCount       int
	registerFloor int
	constValues   map[string]int
}

func CompileNib(typedAST *nibtypechecker.TypedAST, config BuildConfig) CompileResult {
	compiler := &Compiler{
		config:      config,
		typedAST:    typedAST,
		idGen:       ir.NewIDGenerator(),
		program:     ir.NewIrProgram("_start"),
		constValues: map[string]int{},
	}
	compiler.compile(typedAST.Root)
	return CompileResult{Program: compiler.program}
}

func (c *Compiler) compile(ast *parser.ASTNode) {
	for _, child := range childNodes(ast) {
		inner := unwrapTopDecl(child)
		if inner == nil {
			continue
		}
		if inner.RuleName == "const_decl" {
			info := c.extractDeclInfo(inner)
			if info.name != "" {
				c.constValues[info.name] = info.initValue
			}
		} else if inner.RuleName == "static_decl" {
			c.emitStaticData(inner)
		}
	}
	c.emitEntryPoint(ast)
	for _, child := range childNodes(ast) {
		if inner := unwrapTopDecl(child); inner != nil && inner.RuleName == "fn_decl" {
			c.compileFunction(inner)
		}
	}
}

func (c *Compiler) emit(opcode ir.IrOp, operands ...ir.IrOperand) int {
	id := c.idGen.Next()
	c.program.AddInstruction(ir.IrInstruction{Opcode: opcode, Operands: operands, ID: id})
	return id
}

func (c *Compiler) emitLabel(name string) {
	c.program.AddInstruction(ir.IrInstruction{Opcode: ir.OpLabel, Operands: []ir.IrOperand{ir.IrLabel{Name: name}}, ID: -1})
}

func (c *Compiler) emitComment(text string) {
	if c.config.InsertDebugComments {
		c.program.AddInstruction(ir.IrInstruction{Opcode: ir.OpComment, Operands: []ir.IrOperand{ir.IrLabel{Name: text}}, ID: -1})
	}
}

type declInfo struct {
	name      string
	nibType   nibtypechecker.NibType
	hasType   bool
	initValue int
}

func (c *Compiler) extractDeclInfo(node *parser.ASTNode) declInfo {
	info := declInfo{}
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "type" && !info.hasType {
				if parsed, ok := resolveTypeNode(value); ok {
					info.nibType = parsed
					info.hasType = true
				}
			} else if isExpressionNode(value) {
				info.initValue = extractConstInt(value)
			}
		case lexer.Token:
			if info.name == "" && tokenTypeName(value) == "NAME" {
				info.name = value.Value
			}
			if tokenTypeName(value) == "INT_LIT" || tokenTypeName(value) == "HEX_LIT" {
				if parsed, ok := parseLiteral(value.Value, tokenTypeName(value)); ok {
					info.initValue = parsed
				}
			}
		}
	}
	return info
}

func (c *Compiler) emitStaticData(node *parser.ASTNode) {
	info := c.extractDeclInfo(node)
	if info.name == "" || !info.hasType {
		return
	}
	c.emitComment("static " + info.name + ": " + string(info.nibType))
	c.program.AddData(ir.IrDataDecl{Label: info.name, Size: typeSizeBytes(info.nibType), Init: info.initValue})
}

func (c *Compiler) emitEntryPoint(ast *parser.ASTNode) {
	c.emitLabel("_start")
	c.emitComment("program entry point: initialize v0=0, call main, halt")
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: regZero}, ir.IrImmediate{Value: 0})
	if hasFunctionNamed(ast, "main") {
		c.emit(ir.OpCall, ir.IrLabel{Name: "_fn_main"})
	}
	c.emit(ir.OpHalt)
}

func (c *Compiler) compileFunction(node *parser.ASTNode) {
	functionName := ""
	var blockNode *parser.ASTNode
	params := extractParams(node)
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "block" {
				blockNode = value
			}
		case lexer.Token:
			if functionName == "" && tokenTypeName(value) == "NAME" {
				functionName = value.Value
			}
		}
	}
	if functionName == "" || blockNode == nil {
		return
	}
	c.emitComment("function: " + functionName)
	c.emitLabel("_fn_" + functionName)
	registers := map[string]int{}
	nextRegister := regArgBase
	if c.config.CopyParametersToLocals {
		nextRegister = c.firstLocalRegister(len(params))
		for index, param := range params {
			argRegister := regArgBase + index
			registers[param[0]] = nextRegister
			c.emit(ir.OpAddImm, ir.IrRegister{Index: nextRegister}, ir.IrRegister{Index: argRegister}, ir.IrImmediate{Value: 0})
			nextRegister++
		}
	} else {
		for _, param := range params {
			registers[param[0]] = nextRegister
			nextRegister++
		}
	}
	c.registerFloor = nextRegister
	c.compileBlock(blockNode, registers, nextRegister)
	c.emit(ir.OpRet)
	c.registerFloor = 0
}

func (c *Compiler) localRegisterBase() int {
	if c.config.LocalRegisterBase > 0 {
		return c.config.LocalRegisterBase
	}
	return defaultLocalRegisterBase
}

func (c *Compiler) firstLocalRegister(paramCount int) int {
	base := c.localRegisterBase()
	argLimit := regArgBase + paramCount
	if base < argLimit {
		return argLimit
	}
	return base
}

func (c *Compiler) nextAvailableRegister(nextRegister int) int {
	if nextRegister < c.registerFloor {
		return c.registerFloor
	}
	return nextRegister
}

func (c *Compiler) claimRegisterBlock(nextRegister int, count int) int {
	base := c.nextAvailableRegister(nextRegister)
	if count > 0 && base+count > c.registerFloor {
		c.registerFloor = base + count
	}
	return base
}

func (c *Compiler) compileBlock(block *parser.ASTNode, registers map[string]int, nextRegister int) int {
	current := c.nextAvailableRegister(nextRegister)
	for _, child := range childNodes(block) {
		if child.RuleName == "stmt" && len(child.Children) > 0 {
			if inner, ok := child.Children[0].(*parser.ASTNode); ok {
				current = c.compileStatement(inner, registers, current)
			}
		}
	}
	return current
}

func (c *Compiler) compileStatement(node *parser.ASTNode, registers map[string]int, nextRegister int) int {
	switch node.RuleName {
	case "let_stmt":
		return c.compileLet(node, registers, nextRegister)
	case "assign_stmt":
		c.compileAssign(node, registers)
	case "return_stmt":
		c.compileReturn(node, registers)
	case "for_stmt":
		return c.compileFor(node, registers, nextRegister)
	case "if_stmt":
		c.compileIf(node, registers, nextRegister)
	case "expr_stmt":
		if exprs := expressionChildren(node); len(exprs) > 0 {
			c.compileExpr(exprs[0], registers)
		}
	}
	return nextRegister
}

func (c *Compiler) compileLet(node *parser.ASTNode, registers map[string]int, nextRegister int) int {
	name := ""
	var typeNode *parser.ASTNode
	var expression *parser.ASTNode
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "type" {
				typeNode = value
			} else if isExpressionNode(value) {
				expression = value
			}
		case lexer.Token:
			if name == "" && tokenTypeName(value) == "NAME" {
				name = value.Value
			}
		}
	}
	if name == "" || expression == nil {
		return nextRegister
	}
	destination := c.claimRegisterBlock(nextRegister, 1)
	registers[name] = destination
	resultRegister := c.compileExpr(expression, registers)
	if resultRegister != destination {
		c.emit(ir.OpAddImm, ir.IrRegister{Index: destination}, ir.IrRegister{Index: resultRegister}, ir.IrImmediate{Value: 0})
	}
	if typeNode != nil {
		if nibType, ok := resolveTypeNode(typeNode); ok {
			c.emitComment("let " + name + ": " + string(nibType))
		}
	}
	return destination + 1
}

func (c *Compiler) compileAssign(node *parser.ASTNode, registers map[string]int) {
	name := ""
	var expression *parser.ASTNode
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if isExpressionNode(value) {
				expression = value
			}
		case lexer.Token:
			if name == "" && tokenTypeName(value) == "NAME" {
				name = value.Value
			}
		}
	}
	target, ok := registers[name]
	if !ok || expression == nil {
		return
	}
	valueRegister := c.compileExpr(expression, registers)
	c.emit(ir.OpAddImm, ir.IrRegister{Index: target}, ir.IrRegister{Index: valueRegister}, ir.IrImmediate{Value: 0})
}

func (c *Compiler) compileReturn(node *parser.ASTNode, registers map[string]int) {
	if exprs := expressionChildren(node); len(exprs) > 0 {
		valueRegister := c.compileExpr(exprs[0], registers)
		if valueRegister != regScratch {
			c.emit(ir.OpAddImm, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: valueRegister}, ir.IrImmediate{Value: 0})
		}
	}
	c.emit(ir.OpRet)
}

func (c *Compiler) compileFor(node *parser.ASTNode, registers map[string]int, nextRegister int) int {
	loopVar := ""
	var blockNode *parser.ASTNode
	exprs := expressionChildren(node)
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "block" {
				blockNode = value
			}
		case lexer.Token:
			if loopVar == "" && tokenTypeName(value) == "NAME" {
				loopVar = value.Value
			}
		}
	}
	if loopVar == "" || len(exprs) < 2 || blockNode == nil {
		return nextRegister
	}
	baseRegister := c.claimRegisterBlock(nextRegister, 2)
	loopRegister := baseRegister
	limitRegister := baseRegister + 1
	startLabel := "loop_" + strconv.Itoa(c.loopCount) + "_start"
	endLabel := "loop_" + strconv.Itoa(c.loopCount) + "_end"
	c.loopCount++

	registers[loopVar] = loopRegister
	startValueRegister := c.compileExpr(exprs[0], registers)
	if startValueRegister != loopRegister {
		c.emit(ir.OpAddImm, ir.IrRegister{Index: loopRegister}, ir.IrRegister{Index: startValueRegister}, ir.IrImmediate{Value: 0})
	}
	limitValueRegister := c.compileExpr(exprs[1], registers)
	if limitValueRegister != limitRegister {
		c.emit(ir.OpAddImm, ir.IrRegister{Index: limitRegister}, ir.IrRegister{Index: limitValueRegister}, ir.IrImmediate{Value: 0})
	}

	c.emitLabel(startLabel)
	c.emit(ir.OpCmpLt, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: loopRegister}, ir.IrRegister{Index: limitRegister})
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: regScratch}, ir.IrLabel{Name: endLabel})

	nested := copyRegisters(registers)
	c.compileBlock(blockNode, nested, baseRegister+2)
	c.emit(ir.OpAddImm, ir.IrRegister{Index: loopRegister}, ir.IrRegister{Index: loopRegister}, ir.IrImmediate{Value: 1})
	c.emit(ir.OpJump, ir.IrLabel{Name: startLabel})
	c.emitLabel(endLabel)
	return baseRegister + 2
}

func (c *Compiler) compileIf(node *parser.ASTNode, registers map[string]int, nextRegister int) {
	exprs := expressionChildren(node)
	if len(exprs) == 0 {
		return
	}
	conditionRegister := c.compileExpr(exprs[0], registers)
	elseLabel := "if_" + strconv.Itoa(c.ifCount) + "_else"
	endLabel := "if_" + strconv.Itoa(c.ifCount) + "_end"
	c.ifCount++
	c.emit(ir.OpBranchZ, ir.IrRegister{Index: conditionRegister}, ir.IrLabel{Name: elseLabel})

	blocks := []*parser.ASTNode{}
	for _, child := range childNodes(node) {
		if child.RuleName == "block" {
			blocks = append(blocks, child)
		}
	}
	if len(blocks) > 0 {
		c.compileBlock(blocks[0], copyRegisters(registers), nextRegister)
	}
	c.emit(ir.OpJump, ir.IrLabel{Name: endLabel})
	c.emitLabel(elseLabel)
	if len(blocks) > 1 {
		c.compileBlock(blocks[1], copyRegisters(registers), nextRegister)
	}
	c.emitLabel(endLabel)
}

func (c *Compiler) compileExpr(node any, registers map[string]int) int {
	switch value := node.(type) {
	case lexer.Token:
		return c.compileTokenExpr(value, registers)
	case *parser.ASTNode:
		switch value.RuleName {
		case "call_expr":
			return c.compileCallExpr(value, registers)
		case "primary":
			return c.compilePrimary(value, registers)
		case "add_expr":
			return c.compileAddExpr(value, registers)
		case "or_expr", "and_expr", "eq_expr", "cmp_expr", "bitwise_expr", "unary_expr", "expr":
			return c.compileCompoundExpr(value, registers)
		default:
			if len(value.Children) == 1 {
				return c.compileExpr(value.Children[0], registers)
			}
		}
	}
	return regScratch
}

func (c *Compiler) compileTokenExpr(token lexer.Token, registers map[string]int) int {
	typeName := tokenTypeName(token)
	if typeName == "INT_LIT" || typeName == "HEX_LIT" {
		if parsed, ok := parseLiteral(token.Value, typeName); ok {
			c.emit(ir.OpLoadImm, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: parsed})
		}
		return regScratch
	}
	if token.Value == "true" || token.Value == "false" {
		value := 0
		if token.Value == "true" {
			value = 1
		}
		c.emit(ir.OpLoadImm, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: value})
		return regScratch
	}
	if registerIndex, ok := registers[token.Value]; ok {
		return registerIndex
	}
	if value, ok := c.constValues[token.Value]; ok {
		c.emit(ir.OpLoadImm, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: value})
		return regScratch
	}
	return regScratch
}

func (c *Compiler) compilePrimary(node *parser.ASTNode, registers map[string]int) int {
	if len(node.Children) == 0 {
		return regScratch
	}
	return c.compileExpr(node.Children[0], registers)
}

func (c *Compiler) compileCallExpr(node *parser.ASTNode, registers map[string]int) int {
	functionName := ""
	args := []*parser.ASTNode{}
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "arg_list" {
				args = append(args, expressionChildren(value)...)
			}
		case lexer.Token:
			if functionName == "" && tokenTypeName(value) == "NAME" {
				functionName = value.Value
			}
		}
	}
	if functionName == "" {
		return regScratch
	}
	stagedArgBase := c.claimRegisterBlock(regArgBase+len(args), len(args))
	for index, arg := range args {
		valueRegister := c.compileExpr(arg, registers)
		destination := stagedArgBase + index
		if valueRegister != destination {
			c.emit(ir.OpAddImm, ir.IrRegister{Index: destination}, ir.IrRegister{Index: valueRegister}, ir.IrImmediate{Value: 0})
		}
	}
	for index := range args {
		source := stagedArgBase + index
		destination := regArgBase + index
		if source != destination {
			c.emit(ir.OpAddImm, ir.IrRegister{Index: destination}, ir.IrRegister{Index: source}, ir.IrImmediate{Value: 0})
		}
	}
	c.emit(ir.OpCall, ir.IrLabel{Name: "_fn_" + functionName})
	return regScratch
}

func (c *Compiler) compileCompoundExpr(node *parser.ASTNode, registers map[string]int) int {
	if len(node.Children) == 1 {
		return c.compileExpr(node.Children[0], registers)
	}
	if node.RuleName == "unary_expr" && len(node.Children) >= 2 {
		if operator, ok := node.Children[0].(lexer.Token); ok && (operator.Value == "!" || operator.Value == "~") {
			operandRegister := c.compileExpr(node.Children[1], registers)
			return c.emitUnary(operator.Value, operandRegister, node)
		}
	}
	leftRegister := c.compileExpr(node.Children[0], registers)
	for index := 1; index < len(node.Children)-1; index += 2 {
		operatorToken, ok := node.Children[index].(lexer.Token)
		if !ok {
			continue
		}
		rightRegister := c.compileExpr(node.Children[index+1], registers)
		leftRegister = c.emitBinary(operatorToken.Value, leftRegister, rightRegister)
	}
	return leftRegister
}

func (c *Compiler) compileAddExpr(node *parser.ASTNode, registers map[string]int) int {
	if len(node.Children) == 1 {
		return c.compileExpr(node.Children[0], registers)
	}
	leftRegister := c.compileExpr(node.Children[0], registers)
	for index := 1; index < len(node.Children)-1; index += 2 {
		operatorToken, ok := node.Children[index].(lexer.Token)
		if !ok {
			continue
		}
		rightRegister := c.compileExpr(node.Children[index+1], registers)
		nibType, _ := c.typedAST.TypeOf(node)
		leftRegister = c.emitAddOp(operatorToken.Value, leftRegister, rightRegister, nibType)
	}
	return leftRegister
}

func (c *Compiler) emitUnary(operator string, operandRegister int, node *parser.ASTNode) int {
	if operator == "!" {
		c.emit(ir.OpCmpEq, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: operandRegister}, ir.IrRegister{Index: regZero})
		return regScratch
	}
	nibType, _ := c.typedAST.TypeOf(node)
	mask := 0x0F
	if nibType == nibtypechecker.TypeU8 {
		mask = 0xFF
	}
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: mask})
	c.emit(ir.OpSub, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: operandRegister})
	return regScratch
}

func (c *Compiler) emitBinary(operator string, leftRegister int, rightRegister int) int {
	switch operator {
	case "==":
		c.emit(ir.OpCmpEq, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case "!=":
		c.emit(ir.OpCmpNe, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case "<":
		c.emit(ir.OpCmpLt, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case ">":
		c.emit(ir.OpCmpGt, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case "<=":
		c.emit(ir.OpCmpGt, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: rightRegister}, ir.IrRegister{Index: leftRegister})
		return regScratch
	case ">=":
		c.emit(ir.OpCmpLt, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: rightRegister}, ir.IrRegister{Index: leftRegister})
		return regScratch
	case "&&":
		c.emit(ir.OpAnd, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case "||":
		c.emit(ir.OpAdd, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		c.emit(ir.OpCmpNe, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regZero})
		return regScratch
	case "&":
		c.emit(ir.OpAnd, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	default:
		return leftRegister
	}
}

func (c *Compiler) emitAddOp(operator string, leftRegister int, rightRegister int, nibType nibtypechecker.NibType) int {
	switch operator {
	case "+%":
		c.emit(ir.OpAdd, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		if nibType == nibtypechecker.TypeBCD {
			c.emitComment("bcd +%: backend should emit DAA after ADD")
			c.emit(ir.OpAndImm, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: 255})
		} else if nibType == nibtypechecker.TypeU4 {
			c.emit(ir.OpAndImm, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: 15})
		} else {
			c.emit(ir.OpAndImm, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: regScratch}, ir.IrImmediate{Value: 255})
		}
		return regScratch
	case "-":
		c.emit(ir.OpSub, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	case "+", "+?":
		c.emit(ir.OpAdd, ir.IrRegister{Index: regScratch}, ir.IrRegister{Index: leftRegister}, ir.IrRegister{Index: rightRegister})
		return regScratch
	default:
		return leftRegister
	}
}

func unwrapTopDecl(child *parser.ASTNode) *parser.ASTNode {
	for _, grandchild := range child.Children {
		if node, ok := grandchild.(*parser.ASTNode); ok {
			return node
		}
	}
	return nil
}

func childNodes(node *parser.ASTNode) []*parser.ASTNode {
	out := []*parser.ASTNode{}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			out = append(out, childNode)
		}
	}
	return out
}

func expressionChildren(node *parser.ASTNode) []*parser.ASTNode {
	out := []*parser.ASTNode{}
	for _, child := range childNodes(node) {
		if isExpressionNode(child) {
			out = append(out, child)
		}
	}
	return out
}

func isExpressionNode(node *parser.ASTNode) bool {
	switch node.RuleName {
	case "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr", "add_expr", "bitwise_expr", "unary_expr", "primary", "call_expr":
		return true
	default:
		return false
	}
}

func tokenTypeName(token lexer.Token) string {
	if token.TypeName != "" {
		return token.TypeName
	}
	return token.Type.String()
}

func parseLiteral(value string, typeName string) (int, bool) {
	parsed, ok := parseUintLiteral(value, typeName)
	if !ok {
		return 0, false
	}
	if converted, ok := checkedIntFromUint(parsed); ok {
		return converted, true
	}
	return 0, false
}

func parseUintLiteral(value string, typeName string) (uint64, bool) {
	if typeName == "HEX_LIT" {
		parsed, err := strconv.ParseUint(value[2:], 16, 16)
		if err != nil {
			return 0, false
		}
		return parsed, true
	}
	parsed, err := strconv.ParseUint(value, 10, 16)
	if err != nil {
		return 0, false
	}
	return parsed, true
}

func checkedIntFromUint(value uint64) (int, bool) {
	maxInt := uint64(^uint(0) >> 1)
	if value > maxInt {
		return 0, false
	}
	return int(value), true
}

func extractConstInt(subject any) int {
	switch value := subject.(type) {
	case lexer.Token:
		typeName := tokenTypeName(value)
		if typeName == "INT_LIT" || typeName == "HEX_LIT" {
			if parsed, ok := parseLiteral(value.Value, typeName); ok {
				return parsed
			}
		}
	case *parser.ASTNode:
		for _, child := range value.Children {
			if intValue := extractConstInt(child); intValue != 0 {
				return intValue
			}
		}
	}
	return 0
}

func resolveTypeNode(node *parser.ASTNode) (nibtypechecker.NibType, bool) {
	for _, child := range node.Children {
		if token, ok := child.(lexer.Token); ok {
			return nibtypechecker.ParseTypeName(token.Value)
		}
	}
	return "", false
}

func hasFunctionNamed(ast *parser.ASTNode, name string) bool {
	for _, child := range childNodes(ast) {
		inner := unwrapTopDecl(child)
		if inner == nil || inner.RuleName != "fn_decl" {
			continue
		}
		for _, part := range inner.Children {
			if token, ok := part.(lexer.Token); ok && tokenTypeName(token) == "NAME" && token.Value == name {
				return true
			}
		}
	}
	return false
}

func extractParams(node *parser.ASTNode) [][2]string {
	params := [][2]string{}
	for _, child := range childNodes(node) {
		if child.RuleName == "param_list" {
			for _, param := range childNodes(child) {
				if param.RuleName != "param" {
					continue
				}
				name := ""
				typeName := ""
				for _, part := range param.Children {
					switch value := part.(type) {
					case lexer.Token:
						if name == "" && tokenTypeName(value) == "NAME" {
							name = value.Value
						}
					case *parser.ASTNode:
						if value.RuleName == "type" {
							if parsed, ok := resolveTypeNode(value); ok {
								typeName = string(parsed)
							}
						}
					}
				}
				if name != "" {
					params = append(params, [2]string{name, typeName})
				}
			}
		}
	}
	return params
}

func typeSizeBytes(nibType nibtypechecker.NibType) int {
	if nibType == nibtypechecker.TypeU8 {
		return 2
	}
	return 1
}

func copyRegisters(source map[string]int) map[string]int {
	result := map[string]int{}
	for key, value := range source {
		result[key] = value
	}
	return result
}
