package ls00

// handler_symbols.go — textDocument/documentSymbol
//
// Document symbols power the Outline panel in VS Code (Explorer > OUTLINE)
// and the "Go to Symbol in File" command (Ctrl+Shift+O).
//
// The outline shows a tree of named declarations:
//   ▼ main()          [function]
//     ▼ MyStruct      [struct]
//         field1      [field]
//         field2      [field]
//     factorial()     [function]
//
// The bridge walks the AST looking for declaration nodes (function_def,
// class_def, let_statement, etc.) and returns them as a tree of DocumentSymbols.
//
// # DocumentSymbol vs SymbolInformation
//
// LSP supports two response formats for documentSymbol:
//   1. [DocumentSymbol] — a tree (hierarchical, supports children)
//   2. [SymbolInformation] — a flat list (simpler, no hierarchy)
//
// We use DocumentSymbol (format 1) because it supports nesting. The editor
// can tell which format the server uses by checking whether the first item
// in the array has a "children" field (DocumentSymbol) or a "location" field
// (SymbolInformation).

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleDocumentSymbol processes the textDocument/documentSymbol request.
func (s *LspServer) handleDocumentSymbol(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)

	dsp, ok := s.bridge.(DocumentSymbolsProvider)
	if !ok {
		return []interface{}{}, nil
	}

	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		return []interface{}{}, nil
	}

	symbols, bridgeErr := dsp.DocumentSymbols(parseResult.AST)
	if bridgeErr != nil {
		return []interface{}{}, nil
	}

	return convertDocumentSymbols(symbols), nil
}

// convertDocumentSymbols recursively converts DocumentSymbol slices to
// JSON-serializable maps for the LSP response.
func convertDocumentSymbols(symbols []DocumentSymbol) []interface{} {
	result := make([]interface{}, len(symbols))
	for i, sym := range symbols {
		m := map[string]interface{}{
			"name":           sym.Name,
			"kind":           int(sym.Kind),
			"range":          rangeToLSP(sym.Range),
			"selectionRange": rangeToLSP(sym.SelectionRange),
		}
		if len(sym.Children) > 0 {
			m["children"] = convertDocumentSymbols(sym.Children)
		}
		result[i] = m
	}
	return result
}
