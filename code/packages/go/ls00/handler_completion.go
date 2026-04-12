package ls00

// handler_completion.go — textDocument/completion
//
// Autocomplete is triggered when:
//   - The user pauses typing (configurable delay)
//   - The user presses Ctrl+Space explicitly
//   - The user types a trigger character (we configure "." and " ")
//
// The bridge enumerates symbols in scope at the cursor position and returns
// them as CompletionItems. The editor displays a dropdown list with an icon
// (🔵 function, 🔶 variable, 🔑 keyword, etc.) and filters as the user types.
//
// # CompletionItem Fields
//
// - label:         the text shown in the dropdown
// - kind:          the icon (CompletionFunction, CompletionVariable, etc.)
// - detail:        secondary text (e.g., the return type "→ bool")
// - documentation: shown when the user expands the item
// - insertText:    what to actually insert (defaults to label if absent)
// - insertTextFormat: 1=plain text, 2=snippet (allows tab stops with ${1:name})

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleCompletion processes the textDocument/completion request.
func (s *LspServer) handleCompletion(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)

	cp, ok := s.bridge.(CompletionProvider)
	if !ok {
		// Return empty completion list (not null — null means error in this context).
		return map[string]interface{}{"isIncomplete": false, "items": []interface{}{}}, nil
	}

	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		return map[string]interface{}{"isIncomplete": false, "items": []interface{}{}}, nil
	}

	items, bridgeErr := cp.Completion(parseResult.AST, pos)
	if bridgeErr != nil {
		return map[string]interface{}{"isIncomplete": false, "items": []interface{}{}}, nil
	}

	// Convert CompletionItems to LSP format.
	lspItems := make([]interface{}, len(items))
	for i, item := range items {
		ci := map[string]interface{}{
			"label": item.Label,
		}
		if item.Kind != 0 {
			ci["kind"] = int(item.Kind)
		}
		if item.Detail != "" {
			ci["detail"] = item.Detail
		}
		if item.Documentation != "" {
			ci["documentation"] = item.Documentation
		}
		if item.InsertText != "" {
			ci["insertText"] = item.InsertText
		}
		if item.InsertTextFormat != 0 {
			ci["insertTextFormat"] = item.InsertTextFormat
		}
		lspItems[i] = ci
	}

	// isIncomplete: if true, the editor will re-request as the user types more.
	// We return false (complete list) — the bridge gives us everything in scope.
	return map[string]interface{}{
		"isIncomplete": false,
		"items":        lspItems,
	}, nil
}
