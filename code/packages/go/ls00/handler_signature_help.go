package ls00

// handler_signature_help.go — textDocument/signatureHelp
//
// Signature help shows a tooltip with a function's signature as the user types
// a call. In VS Code, it appears automatically when the user types "(" after
// a function name, and updates as commas are typed to highlight the current parameter.
//
// Example: typing `foo(|` (cursor inside the parens) shows:
//
//   foo(a: int, b: string) → bool
//       ─────── (active parameter highlighted)
//
// Typing a comma advances the highlight to the next parameter.
//
// # Bridge Requirements
//
// The bridge must:
//  1. Identify that the cursor is inside a function call expression.
//  2. Determine which function is being called.
//  3. Look up that function's signature in the symbol table.
//  4. Count the commas to determine which parameter is active.
//
// If the cursor is not inside a call (e.g., in a statement, not an expression),
// the bridge returns (nil, nil) and the tooltip is dismissed.

import (
	jsonrpc "github.com/coding-adventures/json-rpc"
)

// handleSignatureHelp processes the textDocument/signatureHelp request.
func (s *LspServer) handleSignatureHelp(id interface{}, params interface{}) (interface{}, *jsonrpc.ResponseError) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return nil, &jsonrpc.ResponseError{Code: jsonrpc.InvalidParams, Message: "invalid params"}
	}

	uri := parseURI(p)
	pos := parsePosition(p)

	shp, ok := s.bridge.(SignatureHelpProvider)
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

	sigHelp, bridgeErr := shp.SignatureHelp(parseResult.AST, pos)
	if bridgeErr != nil || sigHelp == nil {
		return nil, nil
	}

	// Convert SignatureHelpResult to LSP format.
	lspSigs := make([]interface{}, len(sigHelp.Signatures))
	for i, sig := range sigHelp.Signatures {
		lspParams := make([]interface{}, len(sig.Parameters))
		for j, param := range sig.Parameters {
			pp := map[string]interface{}{"label": param.Label}
			if param.Documentation != "" {
				pp["documentation"] = param.Documentation
			}
			lspParams[j] = pp
		}
		s := map[string]interface{}{
			"label":      sig.Label,
			"parameters": lspParams,
		}
		if sig.Documentation != "" {
			s["documentation"] = sig.Documentation
		}
		lspSigs[i] = s
	}

	return map[string]interface{}{
		"signatures":      lspSigs,
		"activeSignature": sigHelp.ActiveSignature,
		"activeParameter": sigHelp.ActiveParameter,
	}, nil
}
