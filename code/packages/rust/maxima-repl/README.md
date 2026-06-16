# coding-adventures-maxima-repl

The interactive **`maxima`** binary — a Read-Eval-Print loop for the Maxima CAS.

`maxima-repl` wraps a persistent
[`MaximaSession`](../maxima-runtime/README.md) (which reuses the whole Macsyma
symbolic stack) and adds the console behaviours:

- **`(%i«n») ` / `... ` prompts** — the input prompt shows the next input index;
  the continuation prompt appears while a statement spans multiple lines.
- **Line continuation** — Maxima statements end with a terminator: `;` displays
  the result, `$` suppresses it. The REPL keeps reading physical lines until it
  sees a terminator *outside a string* with all brackets balanced.
- **Quit / EOF** — `quit;`, `quit()`, `exit`, or Ctrl-D end the session.
- **Non-fatal errors** — a surface error prints and the session continues.

It is the symbolic sibling of `octave-repl`; the only Maxima-specific part is the
`;`/`$`-terminator continuation rule (vs Octave's `end`/`endX` block rule),
because Maxima is statement-terminated at the REPL surface.

## Running

```
cargo run -p coding-adventures-maxima-repl --bin maxima
```

```text
Maxima (on the Macsyma symbolic stack) — end statements with ; or $, type quit; to exit.
(%i1) diff(x^3, x);
(%o1) 3*x^2
(%i2) x : 5$
(%i3) x + 1;
(%o3) 6
(%i4) quit;
```

## Where it fits in the stack

```
maxima-repl  ← you are here   (the `maxima` binary + continuation logic)
    │
    ▼
maxima-runtime  (MaximaSession facade)
    │
    ▼
macsyma-runtime · symbolic-vm · cas-* crates
```

## Testing

```
cargo test -p coding-adventures-maxima-repl
```
