package ls00

// handler_definition.go — textDocument/definition
//
// "Go to Definition" (F12 in VS Code) jumps the editor to the location where
// the symbol under the cursor was declared.
//
// Example: the cursor is on `foo` in `result := foo(x, y)`. Go to Definition
// jumps to `func foo(a int, b int) int { ... }`.
//
// This feature requires a symbol table: the bridge must track which name was
// declared at which location.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleDefinition processes the textDocument/definition request.
func (s *LspServer) handleDefinition(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)

	dp, ok := s.bridge.(DefinitionProvider)
	if !ok {
		return nil, nil
	}

	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		return nil, nil
	}

	location, bridgeErr := dp.Definition(parseResult.AST, pos, uri)
	if bridgeErr != nil || location == nil {
		return nil, nil
	}

	return locationToLSP(*location), nil
}
