package ls00

// handler_hover.go — textDocument/hover
//
// Hover shows a popup tooltip when the user moves their mouse over a symbol.
// VS Code shows it after the user pauses on a symbol for ~500ms.
//
// # Request Flow
//
//  1. Editor sends: {"method": "textDocument/hover", "params": {"textDocument": {"uri": "..."}, "position": {"line": 5, "character": 10}}}
//  2. Server looks up the document in the DocumentManager.
//  3. Server gets or computes the parse result from the ParseCache.
//  4. If the bridge implements HoverProvider, call bridge.Hover(ast, pos).
//  5. Return the hover result or null (null means "nothing to show here").
//
// # Hover Content Format
//
// The hover result's Contents field is Markdown. VS Code renders it with syntax
// highlighting, bold, italic, code blocks, etc. A good hover result looks like:
//
//   **function** `foo(a int, b string) bool`
//
//   ---
//   Returns true if the condition is met.
//
// The optional Range tells the editor which text to highlight while the hover is shown.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleHover processes the textDocument/hover request.
func (s *LspServer) handleHover(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)

	// Check if the bridge supports hover.
	hp, ok := s.bridge.(HoverProvider)
	if !ok {
		// Capability not advertised, but the editor sent the request anyway.
		// Return null (no hover content) — this is not an error.
		return nil, nil
	}

	// Get the current parse result.
	_, parseResult, err := s.getParseResult(uri)
	if err != nil {
		return nil, err.(*jsonrpc.ResponseError)
	}

	if parseResult.AST == nil {
		// No AST available (fatal parse error). Nothing to hover over.
		return nil, nil
	}

	// Delegate to the bridge.
	hoverResult, bridgeErr := hp.Hover(parseResult.AST, pos)
	if bridgeErr != nil {
		// Internal bridge error — return null hover rather than failing the request.
		// A hover failure is not worth crashing the session over.
		return nil, nil
	}

	if hoverResult == nil {
		// Bridge found nothing at this position (no symbol under cursor, etc.).
		// Return null — the editor will dismiss any existing hover popup.
		return nil, nil
	}

	// Build the LSP hover response.
	// Contents is a MarkupContent object with kind="markdown".
	result := map[string]interface{}{
		"contents": map[string]interface{}{
			"kind":  "markdown",
			"value": hoverResult.Contents,
		},
	}

	// Include the range if the bridge provided one.
	// When present, the editor highlights the symbol text while the hover is open.
	if hoverResult.Range != nil {
		result["range"] = rangeToLSP(*hoverResult.Range)
	}

	return result, nil
}
