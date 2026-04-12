package ls00

// handler_semantic_tokens.go — textDocument/semanticTokens/full
//
// Semantic tokens are the "second pass" of syntax highlighting.
//
// # Two-Pass Highlighting
//
// VS Code's first-pass highlighter uses TextMate grammar rules — regex-based
// patterns that run fast but can't understand context. For example, a variable
// named `string` gets highlighted as an IDENTIFIER, which might look like a
// keyword if the grammar doesn't know better.
//
// The second pass is semantic tokens: the language server (which has fully parsed
// the file) can assign accurate types. The variable `string` gets type "variable"
// and the editor colors it differently from the built-in keyword "string".
//
// # The Compact Encoding
//
// Instead of sending one JSON object per token, LSP uses a compact flat integer
// array. See EncodeSemanticTokens() in capabilities.go for the detailed encoding.
//
// Request: {"method": "textDocument/semanticTokens/full", "params": {"textDocument": {"uri": "..."}}}
//
// Response: {"data": [0, 0, 3, 15, 0,  0, 4, 5, 12, 1,  ...]}
//
// The editor decodes the data array using the legend declared in capabilities.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleSemanticTokensFull processes the textDocument/semanticTokens/full request.
func (s *LspServer) handleSemanticTokensFull(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)

	stp, ok := s.bridge.(SemanticTokensProvider)
	if !ok {
		return map[string]interface{}{"data": []int{}}, nil
	}

	doc, ok := s.docManager.Get(uri)
	if !ok {
		return map[string]interface{}{"data": []int{}}, nil
	}

	// Tokenize the source to get raw tokens for the bridge.
	// We call Tokenize() here (rather than using cached tokens) because
	// the token stream is fast to produce and we need the current source.
	tokens, err := s.bridge.Tokenize(doc.Text)
	if err != nil {
		return map[string]interface{}{"data": []int{}}, nil
	}

	// Ask the bridge to map tokens to semantic types.
	semTokens, bridgeErr := stp.SemanticTokens(doc.Text, tokens)
	if bridgeErr != nil {
		return map[string]interface{}{"data": []int{}}, nil
	}

	// Encode using the compact LSP delta format.
	data := EncodeSemanticTokens(semTokens)

	return map[string]interface{}{"data": data}, nil
}
