// preview.go — /preview/{backend}/{component_id}/{story_name} handler
//
// The preview endpoint is the heart of MosaicBook: it compiles a .mosaic file
// to a given backend and returns a self-contained HTML page suitable for
// display inside an <iframe> in the browser shell.
//
// # Backend wrapping strategies
//
// Each backend produces a different kind of output that requires a different
// HTML wrapper:
//
//   html backend:
//     The compiler emits an HTML fragment (e.g. <div class="card">…</div>).
//     We embed it directly in the <body> of a minimal full HTML page.
//
//   webcomponent backend:
//     The compiler emits a JavaScript file that defines a Custom Element via
//     customElements.define("component-name", class extends HTMLElement {…}).
//     We wrap it in a <script type="module"> and add a usage tag below.
//
//   react backend:
//     The compiler emits JSX/TSX.  We load React + ReactDOM + Babel from
//     unpkg CDN and use Babel's in-browser transform (type="text/babel") to
//     avoid a build step.  No Vite, no webpack — Phase 1 keeps it simple.
//
// # Error handling
//
// If compilation fails (compiler not on PATH, syntax error, etc.) we return
// an HTML error page with the error message displayed in a styled box instead
// of returning a 5xx JSON error.  This keeps the iframe useful as a
// debugging surface.

package main

import (
	"fmt"
	"html"
	"net/http"
	"strings"
)

// validBackends is the set of backends supported in Phase 1.
var validBackends = map[string]bool{
	"html":         true,
	"webcomponent": true,
	"react":        true,
}

// handlePreview handles GET /preview/{backend}/{component_id}/{story_name}.
//
// URL path segments after /preview/ are parsed manually so we don't need a
// third-party router — stdlib's ServeMux can route on /preview/ prefix and we
// split the rest here.
func (s *Server) handlePreview(w http.ResponseWriter, r *http.Request) {
	// Strip the /preview/ prefix and split into parts.
	// Path looks like: /preview/html/src%2FButton/Default
	// After TrimPrefix: "html/src%2FButton/Default"
	//
	// Note: the Go HTTP server URL-decodes path segments, so a %2F in the
	// component_id becomes "/" before we see it.  We receive the component_id
	// as a plain path segment with slashes.  For this reason we join all parts
	// after index 0 (backend) except the last (story_name) as the component_id.
	suffix := strings.TrimPrefix(r.URL.Path, "/preview/")
	parts := strings.SplitN(suffix, "/", 3)
	if len(parts) < 3 {
		http.Error(w, "invalid preview path; expected /preview/{backend}/{component_id}/{story_name}", http.StatusBadRequest)
		return
	}

	backend := parts[0]
	// The component_id can contain slashes (e.g. "src/Button").
	// We split into at most 3 to get [backend, component_id_with_slashes, story].
	// But SplitN(3) already gave us [backend, rest_of_path_and_story].
	// Re-split the third segment to peel off the story name at the end.
	componentAndStory := strings.SplitN(parts[1]+"/"+parts[2], "/", -1)
	if len(componentAndStory) < 2 {
		http.Error(w, "invalid preview path; missing story_name", http.StatusBadRequest)
		return
	}
	storyName := componentAndStory[len(componentAndStory)-1]
	componentID := strings.Join(componentAndStory[:len(componentAndStory)-1], "/")

	// Validate the backend before doing any work.
	if !validBackends[backend] {
		http.Error(w, fmt.Sprintf("unknown backend %q; supported: html, webcomponent, react", backend), http.StatusBadRequest)
		return
	}

	// Look up the component in our catalogue.
	s.mu.Lock()
	components := s.components
	s.mu.Unlock()

	var found *Component
	for i := range components {
		if components[i].ID == componentID {
			found = &components[i]
			break
		}
	}
	if found == nil {
		http.Error(w, fmt.Sprintf("component %q not found", componentID), http.StatusNotFound)
		return
	}

	// Compile the source file to the requested backend.
	compiled, compileErr := s.compileToString(found.SourcePath, backend)

	w.Header().Set("Content-Type", "text/html; charset=utf-8")

	if compileErr != nil {
		// Show a styled error page inside the iframe rather than a 5xx response.
		// This makes it easy to see compiler errors without opening DevTools.
		fmt.Fprint(w, errorPage(compileErr.Error(), backend, found.ID, storyName))
		return
	}

	// Wrap the compiled output in the appropriate HTML page for this backend.
	fmt.Fprint(w, wrapForBackend(compiled, backend, found.ID))
}

// wrapForBackend wraps compiled output in a self-contained HTML page for iframe
// display.  componentID is used to derive the component name for react/webcomponent
// wrapper usage tags.
func wrapForBackend(compiled string, backend string, componentID string) string {
	// Derive a component name from the ID (last path segment, PascalCase).
	compName := componentNameFromID(componentID)

	switch backend {

	case "html":
		// The compiler emits an HTML fragment; embed it verbatim in a full page.
		// The body margin and font-family match typical browser defaults so
		// previews look consistent without the shell's CSS bleeding through.
		return fmt.Sprintf(`<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>body{margin:8px;font-family:sans-serif}</style>
</head>
<body>%s</body></html>`, compiled)

	case "webcomponent":
		// The compiler emits a JS module that registers a Custom Element.
		// We derive the kebab-case tag name from the PascalCase component name.
		// e.g. "ProfileCard" → "profile-card"
		kebab := toKebabCase(compName)
		return fmt.Sprintf(`<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body>
<script type="module">
%s
</script>
<%s></%s>
</body></html>`, compiled, kebab, kebab)

	case "react":
		// The compiler emits JSX/TSX.  We use the Babel in-browser transform
		// (unpkg CDN) so there is no build step required.  The component is
		// rendered into #root using ReactDOM.createRoot.
		//
		// Caveat: in-browser Babel is slow (~2 s on first load) but perfectly
		// fine for Phase 1 development previews.
		return fmt.Sprintf(`<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<script src="https://unpkg.com/react@18/umd/react.development.js"></script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
<script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
</head>
<body>
<div id="root"></div>
<script type="text/babel">
%s
const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(%s, null));
</script>
</body></html>`, compiled, compName)

	default:
		// Should never reach here because handlePreview validates the backend.
		return fmt.Sprintf(`<!DOCTYPE html><html><body>Unknown backend: %s</body></html>`, html.EscapeString(backend))
	}
}

// errorPage returns a styled HTML error page for display inside the preview
// iframe.  Showing errors in-frame keeps the dev workflow smooth — no need to
// open browser DevTools to see what went wrong.
func errorPage(errMsg string, backend string, componentID string, storyName string) string {
	return fmt.Sprintf(`<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>
body{margin:16px;font-family:monospace;background:#1a1a1a;color:#f8f8f2}
.error-box{background:#2d1a1a;border:1px solid #ff5555;border-radius:6px;padding:16px}
h2{color:#ff5555;margin:0 0 12px}
pre{margin:0;white-space:pre-wrap;word-break:break-word;font-size:13px}
.meta{font-size:11px;color:#888;margin-bottom:12px}
</style>
</head>
<body>
<div class="error-box">
<h2>Compilation Error</h2>
<div class="meta">backend: %s &nbsp;|&nbsp; component: %s &nbsp;|&nbsp; story: %s</div>
<pre>%s</pre>
</div>
</body></html>`,
		html.EscapeString(backend),
		html.EscapeString(componentID),
		html.EscapeString(storyName),
		html.EscapeString(errMsg),
	)
}

// componentNameFromID derives a PascalCase component name from an ID like
// "src/ProfileCard" → "ProfileCard".  If the ID contains slashes we take the
// last segment (the filename without extension).
func componentNameFromID(id string) string {
	parts := strings.Split(id, "/")
	return parts[len(parts)-1]
}

// toKebabCase converts a PascalCase or camelCase name to kebab-case.
//
// Examples:
//
//	"ProfileCard"  → "profile-card"
//	"TaskBoard"    → "task-board"
//	"HTMLButton"   → "h-t-m-l-button"   (simplistic but handles most cases)
func toKebabCase(name string) string {
	var b strings.Builder
	for i, r := range name {
		if i > 0 && r >= 'A' && r <= 'Z' {
			b.WriteByte('-')
		}
		if r >= 'A' && r <= 'Z' {
			b.WriteByte(byte(r + 32)) // to lower
		} else {
			b.WriteRune(r)
		}
	}
	return b.String()
}
