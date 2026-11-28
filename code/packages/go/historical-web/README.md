# Historical Web (Go)

Dependency-free Go port of the 1991 CERN HTTP/0.9 and November 1992 HTML Tags
profiles. It is an in-memory library with no network or filesystem behavior.

```go
request, _ := historicalweb.EncodeHTTP09Request("/index.html")
address, _ := historicalweb.ParseHTTP09Request(request)
body, _ := historicalweb.DecodeHTTP09Response([]byte("<TITLE>Early Web</TITLE>"))
tree, _ := historicalweb.ParseHTMLTags1992(body)
_, _, _ = request, address, tree
```

`Element.Attributes` uses `*string` so a recognized valueless attribute is
represented by a nil value. The parser uses manual scanning and indexed stack
scope handling, preserving HWEB00's early omission and raw-text behavior.

The contract is [HWEB02](../../../specs/HWEB02-historical-web-go.md), with
historical source and portable vectors in [HWEB00](../../../specs/HWEB00-historical-web.md).
Run `BUILD` (or its commands in `BUILD_windows`) with Go 1.26.
