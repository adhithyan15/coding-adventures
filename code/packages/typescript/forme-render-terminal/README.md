# @coding-adventures/forme-render-terminal

The first product-integrated non-web Forme backend. The default pure stage
turns routed `ContentNode` values into `TerminalBuffer` values; the named
`packageTerminal` pure stage collects them into an in-memory `DeployArtifact`.

```text
Stream<ContentNode> → render-terminal → Stream<TerminalBuffer>
                                        → package-terminal → DeployArtifact
```

The renderer consumes the same resolved Style IR and per-route Interactivity
IR as the HTML renderer. It applies supported color and text-emphasis rules as
renderer-owned ANSI SGR, strips authored terminal control bytes, preserves
portable text and HTML fallback text, and reports every unsupported style
property, raw node, asset reference, and dropped island in deterministic
`TerminalDegradation` data. It never resolves or executes island modules.
Hostile inputs fail closed: bounded descriptor snapshots reject proxies,
accessors, sparse arrays, cycles, and malformed AST/buffer shapes without
traversing symbol metadata; AST and raw-HTML traversal is bounded; authored
control and Unicode-format characters are removed; and terminal text is capped
at 8 MiB per route.

The packager emits `<root>/<route>.ansi` and
`<root>/<route>.degradations.json` entries. It performs no I/O and declares no
capabilities; an emitter or deploy runner decides whether to persist or publish
the artifact. Routes and roots are checked as portable filesystem paths,
including Windows device names, segment limits, traversal, prototype-pollution
sink names, and case-insensitive file/ancestor collisions. Per-file, buffer,
file-count, and 64 MiB aggregate limits bound packaging. Buffer provenance
hashes source identity plus backend configuration and rendered evidence; the
artifact build ID hashes the canonical ordered file paths and exact contents.

The Coding Adventures blog fans the router stream into both HTML and terminal
renderers with the same classless theme. Its clean and warm product acceptance
proves deterministic multi-backend output and fallback preservation.

## Validation

```bash
npm run build
npm run test:coverage
```

Coverage exceeds 85% for statements and branches and 90% for functions and
lines.
