package ls00

// handler_text_document.go — didOpen, didChange, didClose, didSave
//
// These four notifications form the core of LSP's document synchronization.
// The editor sends them as the user opens, edits, and closes files.
//
// # Text Document Synchronization
//
// The LSP textDocumentSync capability controls how changes are transmitted:
//   - Mode 0 (None):        the server gets no sync events at all
//   - Mode 1 (Full):        each change sends the ENTIRE file content
//   - Mode 2 (Incremental): each change sends only the CHANGED RANGES
//
// We advertise Mode 2 (incremental). This is more efficient for large files:
// typing one character sends ~100 bytes instead of the entire file. However,
// we handle both modes in ApplyChanges (range=nil means full replacement).
//
// # Why Publish Diagnostics Immediately?
//
// When the user types a syntax error, they expect a red squiggle immediately.
// After every open and change event, we:
//  1. Apply the change to the DocumentManager
//  2. Parse the new content (or get the cached parse if unchanged)
//  3. Push diagnostics via textDocument/publishDiagnostics
//
// This "push" model (server → editor) is unlike most LSP features (which are
// pull: editor asks, server responds). The server decides WHEN to push diagnostics;
// the editor just updates its display when it receives them.

// handleDidOpen is called when the editor opens a file.
//
// Params: {"textDocument": {"uri": "...", "languageId": "...", "version": 1, "text": "..."}}
func (s *LspServer) handleDidOpen(params interface{}) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return
	}

	td, ok := p["textDocument"].(map[string]interface{})
	if !ok {
		return
	}

	uri, _ := td["uri"].(string)
	text, _ := td["text"].(string)
	version := 1
	if v, ok := td["version"].(float64); ok {
		version = int(v)
	}

	if uri == "" {
		return
	}

	// Register the document with the manager.
	s.docManager.Open(uri, text, version)

	// Parse immediately and push diagnostics so the editor shows squiggles
	// as soon as the file is opened (even before any edits).
	result := s.parseCache.GetOrParse(uri, version, text, s.bridge)
	s.publishDiagnostics(uri, version, result.Diagnostics)
}

// handleDidChange is called when the user edits a file.
//
// Params: {"textDocument": {"uri": "...", "version": 2}, "contentChanges": [...]}
//
// Each content change is either:
//   - Full replacement: {"text": "..."}
//   - Incremental:      {"range": {...}, "text": "..."}
func (s *LspServer) handleDidChange(params interface{}) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return
	}

	uri := parseURI(p)
	if uri == "" {
		return
	}

	version := 0
	if td, ok := p["textDocument"].(map[string]interface{}); ok {
		if v, ok := td["version"].(float64); ok {
			version = int(v)
		}
	}

	// Parse the content changes array.
	changesRaw, _ := p["contentChanges"].([]interface{})
	changes := make([]TextChange, 0, len(changesRaw))

	for _, changeRaw := range changesRaw {
		changeMap, ok := changeRaw.(map[string]interface{})
		if !ok {
			continue
		}

		newText, _ := changeMap["text"].(string)
		change := TextChange{NewText: newText}

		// If "range" is present, this is an incremental change.
		if rangeRaw, hasRange := changeMap["range"]; hasRange && rangeRaw != nil {
			r := parseLSPRange(rangeRaw)
			change.Range = &r
		}
		// If "range" is absent, change.Range remains nil → full replacement.

		changes = append(changes, change)
	}

	// Apply the changes to the document manager.
	if err := s.docManager.ApplyChanges(uri, changes, version); err != nil {
		// Document wasn't open (e.g., a race condition). Ignore.
		return
	}

	// Get the updated text and re-parse.
	doc, ok := s.docManager.Get(uri)
	if !ok {
		return
	}

	result := s.parseCache.GetOrParse(uri, doc.Version, doc.Text, s.bridge)
	s.publishDiagnostics(uri, version, result.Diagnostics)
}

// handleDidClose is called when the editor closes a file.
//
// Params: {"textDocument": {"uri": "..."}}
//
// We remove the document from our manager and evict its parse cache entry.
// After this, the editor will no longer send changes for this URI.
func (s *LspServer) handleDidClose(params interface{}) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return
	}

	uri := parseURI(p)
	if uri == "" {
		return
	}

	s.docManager.Close(uri)
	s.parseCache.Evict(uri)

	// Clear diagnostics for the closed file by publishing an empty list.
	// If we don't do this, the editor keeps showing the squiggles even after
	// the file is closed. The LSP spec says servers SHOULD clear diagnostics
	// when a file is closed (unless workspace-wide diagnostics are in use).
	s.publishDiagnostics(uri, 0, []Diagnostic{})
}

// handleDidSave is called when the editor saves a file.
//
// Params: {"textDocument": {"uri": "..."}, "text": "..."}
//
// We use didSave as an opportunity to re-parse with the saved content
// (which may differ from the in-memory content if the save included formatting).
// For now, we just acknowledge the save — the document content was already
// updated via didChange events.
func (s *LspServer) handleDidSave(params interface{}) {
	p, ok := params.(map[string]interface{})
	if !ok {
		return
	}

	// If the client sends full text in didSave (willSaveWaitUntil flow), apply it.
	// Otherwise, just republish diagnostics from the current parse state.
	uri := parseURI(p)
	if uri == "" {
		return
	}

	if text, ok := p["text"].(string); ok && text != "" {
		doc, docOk := s.docManager.Get(uri)
		if docOk {
			s.docManager.Close(uri)
			s.docManager.Open(uri, text, doc.Version)
			result := s.parseCache.GetOrParse(uri, doc.Version, text, s.bridge)
			s.publishDiagnostics(uri, doc.Version, result.Diagnostics)
		}
	}
}

// parseLSPRange parses a raw JSON range object from the LSP protocol.
//
// The LSP sends ranges as:
//   {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 5}}
func parseLSPRange(raw interface{}) Range {
	m, ok := raw.(map[string]interface{})
	if !ok {
		return Range{}
	}

	startMap, _ := m["start"].(map[string]interface{})
	endMap, _ := m["end"].(map[string]interface{})

	startLine, _ := startMap["line"].(float64)
	startChar, _ := startMap["character"].(float64)
	endLine, _ := endMap["line"].(float64)
	endChar, _ := endMap["character"].(float64)

	return Range{
		Start: Position{Line: int(startLine), Character: int(startChar)},
		End:   Position{Line: int(endLine), Character: int(endChar)},
	}
}
