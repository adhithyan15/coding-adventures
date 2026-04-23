package nibtypechecker

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	typecheckerprotocol "github.com/adhithyan15/coding-adventures/code/packages/go/type-checker-protocol"
)

type TypedAST struct {
	Root  *parser.ASTNode
	Types map[*parser.ASTNode]NibType
}

func (t *TypedAST) TypeOf(node *parser.ASTNode) (NibType, bool) {
	value, ok := t.Types[node]
	return value, ok
}

type NibTypeChecker struct {
	base      *typecheckerprotocol.GenericTypeChecker[*parser.ASTNode]
	nodeTypes map[*parser.ASTNode]NibType
}

func NewNibTypeChecker() *NibTypeChecker {
	checker := &NibTypeChecker{
		nodeTypes: map[*parser.ASTNode]NibType{},
	}
	checker.base = typecheckerprotocol.NewGenericTypeChecker[*parser.ASTNode](
		func(node *parser.ASTNode) string {
			if node == nil {
				return ""
			}
			return node.RuleName
		},
		func(subject any) (int, int) {
			token := firstToken(subject)
			if token == nil {
				return 1, 1
			}
			return token.Line, token.Column
		},
	)
	return checker
}

func CheckNib(ast *parser.ASTNode) typecheckerprotocol.TypeCheckResult[*TypedAST] {
	return NewNibTypeChecker().Check(ast)
}

func CheckSource(source string) typecheckerprotocol.TypeCheckResult[*TypedAST] {
	ast, err := nibparser.ParseNib(source)
	if err != nil {
		return typecheckerprotocol.TypeCheckResult[*TypedAST]{
			TypedAST: nil,
			Errors: []typecheckerprotocol.TypeErrorDiagnostic{{
				Message: err.Error(),
				Line:    1,
				Column:  1,
			}},
			OK: false,
		}
	}
	return CheckNib(ast)
}

func (n *NibTypeChecker) Check(ast *parser.ASTNode) typecheckerprotocol.TypeCheckResult[*TypedAST] {
	n.base.Reset()
	n.nodeTypes = map[*parser.ASTNode]NibType{}
	scope := NewScopeChain()
	if ast != nil {
		n.checkProgram(ast, scope)
	}
	errors := n.base.Errors()
	return typecheckerprotocol.TypeCheckResult[*TypedAST]{
		TypedAST: &TypedAST{
			Root:  ast,
			Types: n.nodeTypes,
		},
		Errors: errors,
		OK:     len(errors) == 0,
	}
}

var expressionRules = map[string]bool{
	"expr": true, "or_expr": true, "and_expr": true, "eq_expr": true, "cmp_expr": true,
	"add_expr": true, "bitwise_expr": true, "unary_expr": true, "primary": true, "call_expr": true,
}

func childNodes(node *parser.ASTNode) []*parser.ASTNode {
	if node == nil {
		return nil
	}
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
		if expressionRules[child.RuleName] {
			out = append(out, child)
		}
	}
	return out
}

func firstToken(subject any) *lexer.Token {
	switch value := subject.(type) {
	case lexer.Token:
		return &value
	case *parser.ASTNode:
		if value == nil {
			return nil
		}
		for _, child := range value.Children {
			if token := firstToken(child); token != nil {
				return token
			}
		}
	}
	return nil
}

func tokenTypeName(token lexer.Token) string {
	if token.TypeName != "" {
		return token.TypeName
	}
	return token.Type.String()
}

func isNumericLiteralExpr(subject any) bool {
	switch node := subject.(type) {
	case lexer.Token:
		return tokenTypeName(node) == "INT_LIT" || tokenTypeName(node) == "HEX_LIT"
	case *parser.ASTNode:
		if node == nil || len(node.Children) == 0 {
			return false
		}
		sawASTChild := false
		for _, child := range node.Children {
			switch value := child.(type) {
			case *parser.ASTNode:
				sawASTChild = true
				if !isNumericLiteralExpr(value) {
					return false
				}
			case lexer.Token:
				typeName := tokenTypeName(value)
				if typeName == "NAME" || typeName == "true" || typeName == "false" || value.Value == "true" || value.Value == "false" {
					return false
				}
				switch typeName {
				case "EQ_EQ", "NEQ", "LEQ", "GEQ", "LT", "GT", "LAND", "LOR":
					return false
				}
			}
		}
		return sawASTChild
	default:
		return false
	}
}

func (n *NibTypeChecker) checkProgram(node *parser.ASTNode, scope *ScopeChain) {
	functionNodes := []*parser.ASTNode{}
	for _, child := range childNodes(node) {
		if len(child.Children) == 0 {
			continue
		}
		decl, ok := child.Children[0].(*parser.ASTNode)
		if !ok {
			continue
		}
		switch decl.RuleName {
		case "const_decl":
			n.collectConstOrStatic(decl, scope, true)
		case "static_decl":
			n.collectConstOrStatic(decl, scope, false)
		case "fn_decl":
			if n.collectFunctionSignature(decl, scope) {
				functionNodes = append(functionNodes, decl)
			}
		}
	}
	for _, functionNode := range functionNodes {
		n.checkFunctionBody(functionNode, scope)
	}
}

func (n *NibTypeChecker) collectConstOrStatic(node *parser.ASTNode, scope *ScopeChain, isConst bool) {
	var nameToken *lexer.Token
	var typeNode *parser.ASTNode
	tokenIndex := 0
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "type" {
				typeNode = value
			}
		case lexer.Token:
			if tokenIndex == 1 && tokenTypeName(value) == "NAME" {
				copy := value
				nameToken = &copy
			}
			tokenIndex++
		}
	}
	if nameToken == nil || typeNode == nil {
		return
	}
	if nibType, ok := n.resolveTypeNode(typeNode); ok {
		scope.DefineGlobal(nameToken.Value, SymbolRecord{
			Name:     nameToken.Value,
			NibType:  nibType,
			HasType:  true,
			IsConst:  isConst,
			IsStatic: !isConst,
		})
	}
}

func (n *NibTypeChecker) collectFunctionSignature(node *parser.ASTNode, scope *ScopeChain) bool {
	functionName := ""
	params := [][2]any{}
	var returnType NibType
	hasReturn := false

	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if functionName == "" && tokenTypeName(value) == "NAME" {
				functionName = value.Value
			}
		case *parser.ASTNode:
			if value.RuleName == "param_list" {
				params = n.extractParams(value)
			} else if value.RuleName == "type" {
				if parsed, ok := n.resolveTypeNode(value); ok {
					returnType = parsed
					hasReturn = true
				}
			}
		}
	}

	if functionName == "" {
		return false
	}

	scope.DefineGlobal(functionName, SymbolRecord{
		Name:         functionName,
		IsFn:         true,
		FnParams:     params,
		FnReturnType: returnType,
		HasReturn:    hasReturn,
	})
	return true
}

func (n *NibTypeChecker) extractParams(node *parser.ASTNode) [][2]any {
	params := [][2]any{}
	for _, child := range childNodes(node) {
		if child.RuleName != "param" {
			continue
		}
		name := ""
		var typeNode *parser.ASTNode
		for _, paramChild := range child.Children {
			switch value := paramChild.(type) {
			case *parser.ASTNode:
				if value.RuleName == "type" {
					typeNode = value
				}
			case lexer.Token:
				if tokenTypeName(value) == "NAME" && name == "" {
					name = value.Value
				}
			}
		}
		if name != "" && typeNode != nil {
			if nibType, ok := n.resolveTypeNode(typeNode); ok {
				params = append(params, [2]any{name, nibType})
			}
		}
	}
	return params
}

func (n *NibTypeChecker) checkFunctionBody(node *parser.ASTNode, globalScope *ScopeChain) {
	symbol, ok := n.functionSymbolFor(node, globalScope)
	if !ok {
		return
	}
	globalScope.Push()
	for _, pair := range symbol.FnParams {
		name, _ := pair[0].(string)
		nibType, _ := pair[1].(NibType)
		globalScope.Define(name, SymbolRecord{Name: name, NibType: nibType, HasType: true})
	}
	for _, child := range childNodes(node) {
		if child.RuleName == "block" {
			n.checkBlock(child, globalScope, symbol.FnReturnType, symbol.HasReturn, false)
		}
	}
	globalScope.Pop()
}

func (n *NibTypeChecker) functionSymbolFor(node *parser.ASTNode, scope *ScopeChain) (SymbolRecord, bool) {
	for _, child := range node.Children {
		if token, ok := child.(lexer.Token); ok && tokenTypeName(token) == "NAME" {
			return scope.Lookup(token.Value)
		}
	}
	return SymbolRecord{}, false
}

func (n *NibTypeChecker) checkBlock(node *parser.ASTNode, scope *ScopeChain, expectedReturn NibType, hasReturn bool, createScope bool) {
	if createScope {
		scope.Push()
	}
	for _, child := range childNodes(node) {
		if child.RuleName != "stmt" || len(child.Children) == 0 {
			continue
		}
		stmt, ok := child.Children[0].(*parser.ASTNode)
		if ok {
			n.checkStatement(stmt, scope, expectedReturn, hasReturn)
		}
	}
	if createScope {
		scope.Pop()
	}
}

func (n *NibTypeChecker) checkStatement(node *parser.ASTNode, scope *ScopeChain, expectedReturn NibType, hasReturn bool) {
	switch node.RuleName {
	case "let_stmt":
		n.checkLetStatement(node, scope)
	case "assign_stmt":
		n.checkAssignStatement(node, scope)
	case "return_stmt":
		n.checkReturnStatement(node, scope, expectedReturn, hasReturn)
	case "for_stmt":
		n.checkForStatement(node, scope, expectedReturn, hasReturn)
	case "if_stmt":
		n.checkIfStatement(node, scope, expectedReturn, hasReturn)
	case "expr_stmt":
		if exprs := expressionChildren(node); len(exprs) > 0 {
			n.checkExpression(exprs[0], scope)
		}
	}
}

func (n *NibTypeChecker) checkLetStatement(node *parser.ASTNode, scope *ScopeChain) {
	var nameToken *lexer.Token
	var typeNode *parser.ASTNode
	exprs := expressionChildren(node)
	var expr *parser.ASTNode
	if len(exprs) > 0 {
		expr = exprs[0]
	}
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "type" {
				typeNode = value
			}
		case lexer.Token:
			if tokenTypeName(value) == "NAME" && nameToken == nil {
				copy := value
				nameToken = &copy
			}
		}
	}
	if nameToken == nil || typeNode == nil || expr == nil {
		return
	}
	declaredType, ok := n.resolveTypeNode(typeNode)
	if !ok {
		return
	}
	if exprType, ok := n.checkExpression(expr, scope); ok && !isNumericLiteralExpr(expr) && !TypesAreCompatible(declaredType, exprType) {
		n.base.Error(fmt.Sprintf("Cannot initialize '%s' of type '%s' with expression of type '%s'.", nameToken.Value, declaredType, exprType), expr)
	}
	scope.Define(nameToken.Value, SymbolRecord{Name: nameToken.Value, NibType: declaredType, HasType: true})
}

func (n *NibTypeChecker) checkAssignStatement(node *parser.ASTNode, scope *ScopeChain) {
	var nameToken *lexer.Token
	exprs := expressionChildren(node)
	var expr *parser.ASTNode
	if len(exprs) > 0 {
		expr = exprs[0]
	}
	for _, child := range node.Children {
		if token, ok := child.(lexer.Token); ok && tokenTypeName(token) == "NAME" {
			copy := token
			nameToken = &copy
			break
		}
	}
	if nameToken == nil || expr == nil {
		return
	}
	symbol, ok := scope.Lookup(nameToken.Value)
	if !ok || !symbol.HasType {
		n.base.Error(fmt.Sprintf("'%s' is not defined.", nameToken.Value), *nameToken)
		return
	}
	if exprType, ok := n.checkExpression(expr, scope); ok && !isNumericLiteralExpr(expr) && !TypesAreCompatible(symbol.NibType, exprType) {
		n.base.Error(fmt.Sprintf("Cannot assign expression of type '%s' to '%s' of type '%s'.", exprType, nameToken.Value, symbol.NibType), expr)
	}
}

func (n *NibTypeChecker) checkReturnStatement(node *parser.ASTNode, scope *ScopeChain, expectedReturn NibType, hasReturn bool) {
	exprs := expressionChildren(node)
	if len(exprs) == 0 {
		return
	}
	if exprType, ok := n.checkExpression(exprs[0], scope); ok && hasReturn && !TypesAreCompatible(expectedReturn, exprType) {
		n.base.Error(fmt.Sprintf("Return type mismatch: expected '%s' but got '%s'.", expectedReturn, exprType), exprs[0])
	}
}

func (n *NibTypeChecker) checkForStatement(node *parser.ASTNode, scope *ScopeChain, expectedReturn NibType, hasReturn bool) {
	var loopVar *lexer.Token
	var loopTypeNode *parser.ASTNode
	var blockNode *parser.ASTNode
	exprs := expressionChildren(node)
	for _, child := range node.Children {
		switch value := child.(type) {
		case *parser.ASTNode:
			if value.RuleName == "type" && loopTypeNode == nil {
				loopTypeNode = value
			} else if value.RuleName == "block" {
				blockNode = value
			}
		case lexer.Token:
			if tokenTypeName(value) == "NAME" && loopVar == nil {
				copy := value
				loopVar = &copy
			}
		}
	}
	for _, boundExpr := range exprs {
		if boundType, ok := n.checkExpression(boundExpr, scope); ok && !IsNumeric(boundType) {
			n.base.Error(fmt.Sprintf("For-loop bounds must be numeric, but got '%s'.", boundType), boundExpr)
		}
	}
	if loopVar == nil || loopTypeNode == nil || blockNode == nil {
		return
	}
	loopType, ok := n.resolveTypeNode(loopTypeNode)
	if !ok {
		return
	}
	scope.Push()
	scope.Define(loopVar.Value, SymbolRecord{Name: loopVar.Value, NibType: loopType, HasType: true})
	n.checkBlock(blockNode, scope, expectedReturn, hasReturn, false)
	scope.Pop()
}

func (n *NibTypeChecker) checkIfStatement(node *parser.ASTNode, scope *ScopeChain, expectedReturn NibType, hasReturn bool) {
	if exprs := expressionChildren(node); len(exprs) > 0 {
		if exprType, ok := n.checkExpression(exprs[0], scope); ok && exprType != TypeBool {
			n.base.Error(fmt.Sprintf("The condition of 'if' must have type 'bool', but got '%s'.", exprType), exprs[0])
		}
	}
	for _, child := range childNodes(node) {
		if child.RuleName == "block" {
			n.checkBlock(child, scope, expectedReturn, hasReturn, true)
		}
	}
}

func (n *NibTypeChecker) checkExpression(node any, scope *ScopeChain) (NibType, bool) {
	switch value := node.(type) {
	case lexer.Token:
		return n.checkTokenExpression(value, scope)
	case *parser.ASTNode:
		var result NibType
		var ok bool
		switch value.RuleName {
		case "call_expr":
			result, ok = n.checkCallExpression(value, scope)
		case "primary":
			result, ok = n.checkPrimary(value, scope)
		case "add_expr":
			result, ok = n.checkAddExpression(value, scope)
		case "or_expr", "and_expr", "eq_expr", "cmp_expr", "bitwise_expr", "unary_expr", "expr":
			result, ok = n.checkCompoundExpression(value, scope)
		default:
			if len(value.Children) == 1 {
				result, ok = n.checkExpression(value.Children[0], scope)
			}
		}
		if ok {
			n.nodeTypes[value] = result
		}
		return result, ok
	default:
		return "", false
	}
}

func (n *NibTypeChecker) checkTokenExpression(token lexer.Token, scope *ScopeChain) (NibType, bool) {
	switch tokenTypeName(token) {
	case "INT_LIT", "HEX_LIT":
		return TypeU4, true
	case "true", "false":
		return TypeBool, true
	}
	if token.Value == "true" || token.Value == "false" {
		return TypeBool, true
	}
	if tokenTypeName(token) != "NAME" {
		return "", false
	}
	symbol, ok := scope.Lookup(token.Value)
	if !ok || !symbol.HasType {
		n.base.Error(fmt.Sprintf("'%s' is not defined.", token.Value), token)
		return "", false
	}
	if symbol.IsFn {
		n.base.Error(fmt.Sprintf("'%s' is a function. Use parentheses to call it.", token.Value), token)
		return "", false
	}
	return symbol.NibType, true
}

func (n *NibTypeChecker) checkCompoundExpression(node *parser.ASTNode, scope *ScopeChain) (NibType, bool) {
	if len(node.Children) == 1 {
		return n.checkExpression(node.Children[0], scope)
	}
	if node.RuleName == "or_expr" || node.RuleName == "and_expr" {
		for _, expr := range expressionChildren(node) {
			if exprType, ok := n.checkExpression(expr, scope); ok && exprType != TypeBool {
				n.base.Error("Logical operators require bool operands.", expr)
			}
		}
		return TypeBool, true
	}
	if node.RuleName == "eq_expr" || node.RuleName == "cmp_expr" {
		types := []NibType{}
		for _, expr := range expressionChildren(node) {
			if exprType, ok := n.checkExpression(expr, scope); ok {
				types = append(types, exprType)
			}
		}
		if len(types) >= 2 && types[0] != types[1] {
			n.base.Error(fmt.Sprintf("Comparison operands must have the same type. Got '%s' and '%s'.", types[0], types[1]), node)
		}
		if node.RuleName == "cmp_expr" && len(types) > 0 && !IsNumeric(types[0]) {
			n.base.Error(fmt.Sprintf("Comparison operands must be numeric, but got '%s'.", types[0]), node)
		}
		return TypeBool, true
	}
	if node.RuleName == "bitwise_expr" {
		types := []NibType{}
		for _, expr := range expressionChildren(node) {
			if exprType, ok := n.checkExpression(expr, scope); ok {
				types = append(types, exprType)
			}
		}
		if len(types) >= 2 && types[0] != types[1] {
			n.base.Error(fmt.Sprintf("Bitwise operands must have the same type. Got '%s' and '%s'.", types[0], types[1]), node)
		}
		if len(types) > 0 {
			return types[0], true
		}
		return "", false
	}
	if node.RuleName == "unary_expr" && len(node.Children) >= 2 {
		if operator, ok := node.Children[0].(lexer.Token); ok {
			if operandType, ok := n.checkExpression(node.Children[1], scope); ok {
				if operator.Value == "!" {
					if operandType != TypeBool {
						n.base.Error(fmt.Sprintf("Logical NOT requires a bool operand, but got '%s'.", operandType), node.Children[1])
					}
					return TypeBool, true
				}
				return operandType, true
			}
		}
	}
	if exprs := expressionChildren(node); len(exprs) > 0 {
		return n.checkExpression(exprs[0], scope)
	}
	return "", false
}

func (n *NibTypeChecker) checkAddExpression(node *parser.ASTNode, scope *ScopeChain) (NibType, bool) {
	if len(node.Children) == 1 {
		return n.checkExpression(node.Children[0], scope)
	}
	operandTypes := []NibType{}
	for _, expr := range expressionChildren(node) {
		if exprType, ok := n.checkExpression(expr, scope); ok {
			operandTypes = append(operandTypes, exprType)
		}
	}
	hasBCD := false
	resultType := NibType("")
	for _, operandType := range operandTypes {
		if operandType == TypeBCD {
			hasBCD = true
		}
		if resultType == "" {
			resultType = operandType
		}
	}
	operandIndex := 0
	for index := 1; index < len(node.Children)-1; index += 2 {
		operator, ok := node.Children[index].(lexer.Token)
		if !ok {
			continue
		}
		var left, right NibType
		if operandIndex < len(operandTypes) {
			left = operandTypes[operandIndex]
		}
		if operandIndex+1 < len(operandTypes) {
			right = operandTypes[operandIndex+1]
		}
		if hasBCD && !IsBCDOpAllowed(operator.Value) {
			n.base.Error(fmt.Sprintf("BCD operands only support '+%%' and '-', but got '%s'.", operator.Value), operator)
		}
		if left != "" && right != "" && left != right {
			n.base.Error(fmt.Sprintf("Operands of '%s' must have the same type. Got '%s' and '%s'.", operator.Value, left, right), operator)
		}
		if resultType == "" {
			if left != "" {
				resultType = left
			} else {
				resultType = right
			}
		}
		operandIndex++
	}
	if resultType != "" {
		return resultType, true
	}
	return "", false
}

func (n *NibTypeChecker) checkPrimary(node *parser.ASTNode, scope *ScopeChain) (NibType, bool) {
	if len(node.Children) == 0 {
		return "", false
	}
	first := node.Children[0]
	if token, ok := first.(lexer.Token); ok {
		return n.checkTokenExpression(token, scope)
	}
	if child, ok := first.(*parser.ASTNode); ok {
		if child.RuleName == "call_expr" {
			return n.checkCallExpression(child, scope)
		}
		return n.checkExpression(child, scope)
	}
	return "", false
}

func (n *NibTypeChecker) checkCallExpression(node *parser.ASTNode, scope *ScopeChain) (NibType, bool) {
	functionName := ""
	args := []*parser.ASTNode{}
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if functionName == "" && tokenTypeName(value) == "NAME" {
				functionName = value.Value
			}
		case *parser.ASTNode:
			if value.RuleName == "arg_list" {
				args = append(args, expressionChildren(value)...)
			}
		}
	}
	if functionName == "" {
		return "", false
	}
	symbol, ok := scope.Lookup(functionName)
	if !ok || !symbol.IsFn {
		n.base.Error(fmt.Sprintf("Function '%s' is not defined.", functionName), node)
		return "", false
	}
	if len(args) != len(symbol.FnParams) {
		n.base.Error(fmt.Sprintf("Function '%s' expects %d argument(s) but got %d.", functionName, len(symbol.FnParams), len(args)), node)
	}
	for index, arg := range args {
		argType, ok := n.checkExpression(arg, scope)
		if !ok || index >= len(symbol.FnParams) {
			continue
		}
		paramType, _ := symbol.FnParams[index][1].(NibType)
		if !TypesAreCompatible(paramType, argType) {
			n.base.Error(fmt.Sprintf("Argument %d to '%s' expected '%s' but got '%s'.", index+1, functionName, paramType, argType), arg)
		}
	}
	if symbol.HasReturn {
		return symbol.FnReturnType, true
	}
	return "", false
}

func (n *NibTypeChecker) resolveTypeNode(node *parser.ASTNode) (NibType, bool) {
	for _, child := range node.Children {
		if token, ok := child.(lexer.Token); ok {
			if parsed, ok := ParseTypeName(token.Value); ok {
				return parsed, true
			}
			n.base.Error(fmt.Sprintf("Unknown type '%s'.", token.Value), token)
			return "", false
		}
	}
	return "", false
}
