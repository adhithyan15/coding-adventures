# Inserting a function above another orphans its doc comment (clippy `-D warnings` failure)

Bit me twice in one session, in two different crates. When you insert a new
function immediately *before* an existing one, you land between that function
and its `///` doc comment:

```rust
/// Shared helper: build the `style="..."` attribute …   <- now documents nothing
                                                          <- blank line
/// UI35 — lower a `HostDraggable` …                      <- your new fn
fn emit_host_draggable(…) { … }

fn build_style_attr(…) { … }                              <- lost its docs
```

`cargo test` passes. `cargo clippy -- -D warnings` fails with `empty line after
doc comment`, so it only shows up in CI unless you lint locally — which is
exactly why the repo rule is to run clippy in BOTH feature configs before
pushing. Insert *after* the preceding function's body instead, anchoring on its
closing brace rather than on the next function's signature; or if you do anchor
on a signature, move the doc block down with the insertion. Also worth knowing:
the clippy message names the function it thinks you meant to document, which is
your *new* function, not the one that actually lost its docs — read the line
number, not the name.
