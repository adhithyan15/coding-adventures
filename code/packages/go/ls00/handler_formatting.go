package ls00

// handler_formatting.go — textDocument/formatting
//
// Document formatting (Format on Save, or Shift+Alt+F in VS Code) reformats
// the entire document to conform to the language's style guide.
//
// The bridge receives the full source text and returns a list of TextEdits
// that transform it to the canonical formatted form. Typically this is a
// single edit replacing the entire file with the formatted content, but it
// could also be a minimal diff (many edits replacing only the changed lines).
//
// # Format-on-Save
//
// VS Code can be configured to format automatically on save:
//   "editor.formatOnSave": true
//
// When enabled, the editor sends textDocument/formatting each time the user
// saves. The server must respond promptly (before the editor times out).
//
// # Formatter Implementation Styles
//
//  1. Full replacement: return one TextEdit covering the entire file.
//     Simple to implement, but causes the editor to re-render the whole file.
//
//  2. Minimal diff: compute the diff between original and formatted, return
//     only the changed ranges. More complex but preserves cursor position
//     and undo history better.
//
// Most language formatters (gofmt, prettier, black) use full replacement.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleFormatting processes the textDocument/formatting request.
func (s *LspServer) handleFormatting(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)

	fp, ok := s.bridge.(FormatProvider)
	if !ok {
		return []interface{}{}, nil
	}

	doc, ok := s.docManager.Get(uri)
	if !ok {
		return []interface{}{}, nil
	}

	edits, bridgeErr := fp.Format(doc.Text)
	if bridgeErr != nil {
		return nil, &jsonrpc.ResponseError{Code: RequestFailed, Message: "formatting failed: " + bridgeErr.Error()}
	}

	lspEdits := make([]interface{}, len(edits))
	for i, edit := range edits {
		lspEdits[i] = map[string]interface{}{
			"range":   rangeToLSP(edit.Range),
			"newText": edit.NewText,
		}
	}

	return lspEdits, nil
}
