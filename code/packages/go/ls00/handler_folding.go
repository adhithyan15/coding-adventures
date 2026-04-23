package ls00

// handler_folding.go — textDocument/foldingRange
//
// Code folding lets the user collapse multi-line regions in the editor by
// clicking a triangle in the gutter, or pressing Ctrl+Shift+[ (fold) / ] (unfold).
//
// Folding regions are derived from the AST structure: any node that spans
// multiple lines is a candidate. Common foldable regions:
//   - Function bodies
//   - Loop bodies (for, while)
//   - Conditional blocks (if/else)
//   - Comment blocks (/* ... */)
//   - Import groups
//
// The bridge walks the AST and returns FoldingRange objects for any multi-line
// node. The framework passes these directly to the editor.
//
// # FoldingRange
//
// A FoldingRange specifies:
//   - startLine: the line where the fold marker appears (0-based)
//   - endLine:   the last line of the folded region (0-based)
//   - kind:      "region", "imports", or "comment" (optional)
//
// When folded, startLine is visible but lines startLine+1 through endLine are
// hidden. The editor appends "..." after the startLine's content.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleFoldingRange processes the textDocument/foldingRange request.
func (s *LspServer) handleFoldingRange(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)

	frp, ok := s.bridge.(FoldingRangesProvider)
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

	ranges, bridgeErr := frp.FoldingRanges(parseResult.AST)
	if bridgeErr != nil {
		return []interface{}{}, nil
	}

	result := make([]interface{}, len(ranges))
	for i, fr := range ranges {
		m := map[string]interface{}{
			"startLine": fr.StartLine,
			"endLine":   fr.EndLine,
		}
		if fr.Kind != "" {
			m["kind"] = fr.Kind
		}
		result[i] = m
	}

	return result, nil
}
