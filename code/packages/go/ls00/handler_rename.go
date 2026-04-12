package ls00

// handler_rename.go — textDocument/rename
//
// Rename (F2 in VS Code) renames a symbol everywhere it appears in the file
// (or across the workspace for multi-file projects).
//
// The protocol:
//  1. Editor sends textDocument/rename with the cursor position and the new name.
//  2. Server returns a WorkspaceEdit: a map of {uri → [TextEdit, ...]}
//  3. Editor applies all the text edits atomically.
//
// The bridge must find ALL occurrences of the symbol (not just the one under
// the cursor) and return a TextEdit for each one. The framework wraps these
// in a WorkspaceEdit keyed by URI.
//
// # Validation
//
// The bridge should validate that the new name is:
//   - A valid identifier in the language
//   - Not already in use in the same scope (to prevent name collisions)
//
// If validation fails, the bridge should return an error. The server propagates
// this as a JSON-RPC error response and the editor shows an error dialog.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleRename processes the textDocument/rename request.
func (s *LspServer) handleRename(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)
	newName, _ := p["newName"].(string)

	if newName == "" {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "newName is required"}
	}

	rp, ok := s.bridge.(RenameProvider)
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: RequestFailed, Message: "rename not supported"}
	}

	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		return nil, &jsonrpc.ResponseError{Code: RequestFailed, Message: "no AST available"}
	}

	edit, bridgeErr := rp.Rename(parseResult.AST, pos, newName)
	if bridgeErr != nil {
		return nil, &jsonrpc.ResponseError{Code: RequestFailed, Message: bridgeErr.Error()}
	}

	if edit == nil {
		return nil, &jsonrpc.ResponseError{Code: RequestFailed, Message: "symbol not found at position"}
	}

	// Convert WorkspaceEdit to LSP format.
	// changes: {uri: [TextEdit, ...], ...}
	lspChanges := map[string]interface{}{}
	for editURI, edits := range edit.Changes {
		lspEdits := make([]interface{}, len(edits))
		for i, te := range edits {
			lspEdits[i] = map[string]interface{}{
				"range":   rangeToLSP(te.Range),
				"newText": te.NewText,
			}
		}
		lspChanges[editURI] = lspEdits
	}

	return map[string]interface{}{"changes": lspChanges}, nil
}
