---
category: BUILD files & dependency management
---

# A go:embed byte slice without an embed symbol needs a blank embed import

Go's `//go:embed` directive on a `[]byte` variable still requires importing
the `embed` package. When no `embed.FS`, `embed.FS` value or other package
symbol appears in source, an ordinary `import "embed"` is reported unused by
`go vet`; use `_ "embed"` to satisfy the directive without a fake symbol use.
