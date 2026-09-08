// render.go — the site's HTML output.
//
// Deliberately dependency-free and hand-written rather than run through the
// repository's `forme` site generator: this site's content is generated per
// build from the packages, so it needs a template over live data rather than
// authored pages. Keeping it here also means the docs site cannot break
// because an unrelated site framework changed.

package main

import (
	"html/template"
	"os"
	"path/filepath"
	"strings"
)

const baseCSS = `
:root { --bg:#faf8f5; --fg:#1f1d1b; --muted:#6b645c; --line:#e6ded3;
        --card:#ffffff; --accent:#b4601a; --code:#f3efe9; }
@media (prefers-color-scheme: dark) {
  :root { --bg:#17150f; --fg:#f1ece3; --muted:#a49b8e; --line:#332e26;
          --card:#201d17; --accent:#e0942a; --code:#262119; }
}
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font-size:15px; line-height:1.55;
       font-family:-apple-system,Segoe UI,system-ui,Helvetica Neue,Arial,sans-serif; }
.wrap { max-width:1040px; margin:0 auto; padding:32px 24px 72px; }
a { color:var(--accent); }
h1 { font-size:28px; margin:0 0 6px; letter-spacing:-.01em; }
h2 { font-size:15px; text-transform:uppercase; letter-spacing:.08em; color:var(--muted);
     margin:34px 0 10px; font-weight:600; }
.sub { color:var(--muted); margin:0 0 26px; }
.grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(210px,1fr)); gap:10px; }
.card { display:block; padding:12px 14px; background:var(--card); border:1px solid var(--line);
        border-radius:10px; text-decoration:none; color:var(--fg); }
.card:hover { border-color:var(--accent); }
.card .n { font-weight:600; }
.card .m { color:var(--muted); font-size:13px; }
table { border-collapse:collapse; width:100%; margin:6px 0 18px; }
th,td { text-align:left; padding:7px 10px; border-bottom:1px solid var(--line); font-size:14px; }
th { color:var(--muted); font-weight:600; font-size:12px; text-transform:uppercase;
     letter-spacing:.06em; }
code,.pill { font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:12.5px; }
.pill { background:var(--code); border:1px solid var(--line); border-radius:5px;
        padding:1px 6px; margin-right:4px; display:inline-block; }
iframe.preview { background:var(--card); border:1px solid var(--line); border-radius:10px;
           width:100%; height:190px; display:block; }
.preview > * { max-width:100%; }
.note { color:var(--muted); font-size:13px; margin:8px 0 0; }
.err { background:var(--code); border:1px solid var(--line); border-radius:8px;
       padding:12px; white-space:pre-wrap; font-size:12.5px; }
`

var indexTmpl = template.Must(template.New("index").Parse(`<!doctype html>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Mosaic components</title><style>` + baseCSS + `</style>
<div class="wrap">
<h1>Mosaic components</h1>
<p class="sub">{{.Count}} components across {{.PkgCount}} packages. Every page on this site is
generated from the packages themselves, so it republishes whenever a component ships.</p>
{{range .Packages}}
<h2>{{.Name}}</h2>
<div class="grid">
{{range .Components}}<a class="card" href="{{.Package}}/{{.Name}}.html">
<div class="n">{{.Name}}</div>
<div class="m">{{len .Slots}} slot{{if ne (len .Slots) 1}}s{{end}}{{if .Emits}} · {{len .Emits}} event{{if ne (len .Emits) 1}}s{{end}}{{end}}</div>
</a>{{end}}
</div>
{{end}}
</div>
`))

var pageTmpl = template.Must(template.New("page").Parse(`<!doctype html>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{{.Name}} — Mosaic</title><style>` + baseCSS + `</style>
<div class="wrap">
<p class="sub"><a href="../index.html">← All components</a></p>
<h1>{{.Name}}</h1>
<p class="sub"><code>{{.Package}}</code>{{if .HasDark}} · light and dark themes{{end}}</p>

<h2>Preview</h2>
{{/* allow-same-origin is required: the emitted project loads its runtime as a
    module script, which a scripts-only sandbox blocks, and the frame renders
    empty. The content is this repository's own compiler output from its own
    sources, generated in the same build -- not third-party or user input. */}}
{{if .PreviewPath}}<iframe class="preview" src="{{.PreviewPath}}" title="{{.Name}} preview"
 loading="lazy" sandbox="allow-scripts allow-same-origin"></iframe>
<p class="note">The real emitted html project, running its own runtime — not a screenshot and not a
re-render. Slot values are samples: the first legal member of each <code>one-of</code> axis, and the
slot's own name for text, because this component has no stories yet.
<a href="{{.PreviewPath}}" target="_blank" rel="noopener">Open it on its own</a>.</p>
{{else}}<div class="err">This component could not be rendered.

{{.PreviewErr}}</div>{{end}}

<h2>Slots</h2>
{{if .Slots}}<table><tr><th>Name</th><th>Type</th><th>Values</th></tr>
{{range .Slots}}<tr><td><code>{{.Name}}</code></td><td><code>{{.Type}}</code></td>
<td>{{range .OneOf}}<span class="pill">{{.}}</span>{{end}}</td></tr>{{end}}
</table>{{else}}<p class="note">This component declares no slots.</p>{{end}}

<h2>Events</h2>
{{if .Emits}}<p>{{range .Emits}}<span class="pill">{{.Name}}</span>{{end}}</p>
{{else}}<p class="note">This component emits no events.</p>{{end}}
</div>
`))

type pkgGroup struct {
	Name       string
	Components []Component
}

func render(comps []Component, outDir string) error {
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return err
	}

	var groups []pkgGroup
	for _, c := range comps {
		if len(groups) == 0 || groups[len(groups)-1].Name != c.Package {
			groups = append(groups, pkgGroup{Name: c.Package})
		}
		g := &groups[len(groups)-1]
		g.Components = append(g.Components, c)
	}

	index, err := os.Create(filepath.Join(outDir, "index.html"))
	if err != nil {
		return err
	}
	defer index.Close()
	if err := indexTmpl.Execute(index, struct {
		Count    int
		PkgCount int
		Packages []pkgGroup
	}{len(comps), len(groups), groups}); err != nil {
		return err
	}

	for _, c := range comps {
		dir := filepath.Join(outDir, c.Package)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
		f, err := os.Create(filepath.Join(dir, c.Name+".html"))
		if err != nil {
			return err
		}
		err = pageTmpl.Execute(f, c)
		f.Close()
		if err != nil {
			return err
		}
	}

	// Pages' Jekyll pass drops files beginning with an underscore. Nothing
	// here starts with one today, but a component that did would vanish
	// silently and only in production.
	return os.WriteFile(filepath.Join(outDir, ".nojekyll"), nil, 0o644)
}

var _ = strings.TrimSpace
