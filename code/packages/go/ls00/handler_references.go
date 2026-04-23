package ls00

// handler_references.go — textDocument/references
//
// "Find All References" shows every location in the codebase where a symbol
// is used. In VS Code, right-click on a symbol → "Find All References".
//
// The result is a list of Locations (file URI + range). The editor displays
// them in the "References" panel with one-click navigation.
//
// Like Go to Definition, this requires a symbol table in the bridge.
// The includeDeclaration flag (from the LSP request context) controls whether
// the declaration location is included in the results.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleReferences processes the textDocument/references request.
func (s *LspServer) handleReferences(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)

	// Extract includeDeclaration from the context object.
	// The LSP sends: {"context": {"includeDeclaration": true}}
	includeDecl := false
	if ctx, ok := p["context"].(map[string]interface{}); ok {
		if incl, ok := ctx["includeDeclaration"].(bool); ok {
			includeDecl = incl
		}
	}

	rp, ok := s.bridge.(ReferencesProvider)
	if !ok {
		return []interface{}{}, nil // Return empty list, not null
	}

	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		return []interface{}{}, nil
	}

	locations, bridgeErr := rp.References(parseResult.AST, pos, uri, includeDecl)
	if bridgeErr != nil {
		return []interface{}{}, nil
	}

	// Convert to LSP format.
	result := make([]interface{}, len(locations))
	for i, loc := range locations {
		result[i] = locationToLSP(loc)
	}
	return result, nil
}
