# Berkeley SPICE v1 Release Readiness

This report freezes the first shipping cut of the cross-language Berkeley
SPICE core. The executable source of truth is
`code/grammars/spice/berkeley-v1-release-manifest.json`; this document explains
what that gate means for users.

## Release Gate

All three parser/engine ports must execute the shared corpus successfully:

- at least 14 numerical core decks, covering `.op`, `.dc`, `.ac`, `.tran`, and
  `.tf`, plus diode, NPN BJT, NJF, and Level-1 NMOS reference circuits;
- at least nine syntax decks, including accepted cards, supported diagnostics,
  and deliberate exclusions; and
- at least one successful and one failing command-line deck through
  `spice-netlist-parser run --json <deck|->`.

The core numerical corpus and syntax corpus landed in PRs #14485 and #14513.
The public CLI contract landed in PR #14539. This release gate binds those
artifacts together and is run by every Python, TypeScript, and Rust package.

## Command-Line And JSON Contract

Run either a deck path or standard input:

```text
spice-netlist-parser run --json deck.cir
cat deck.cir | spice-netlist-parser run --json -
```

Success returns exit status `0` and one canonical JSON object with
`analyses`, `schemaVersion`, and `title`. Each analysis has `index`, `kind`,
and deterministic table `records`. `schemaVersion` is currently `1`.

Deck read, parse, or execution failures return exit status `1` and write a
diagnostic beginning `SPICE_CLI_ERROR:` to stderr. The code and exit status are
stable; explanatory detail is intentionally not byte-for-byte portable across
the three implementation languages. Invalid invocation returns status `2`.

## Supported Berkeley v1 Surface

The supported-card matrix covers independent voltage/current sources, R/C/L,
diodes, BJTs, JFETs, and Level-1 MOS devices; D, NPN, PNP, NJF, PJF, and
Level-1 NMOS/PMOS models; `.op`, `.dc`, `.ac`, `.tran`, and `.tf`; titles and
comments; logical continuations; flat `.subckt` expansion; `.save`; and `.end`.
The exact list is intentionally machine-readable in the release manifest.

Nested subcircuits, MOS levels other than 1, vendor/ngspice extensions, and
`.control` command execution are deliberate exclusions. Other cards and model
parameters may exist in a parser implementation, but they are not Berkeley
SPICE v1 compatibility promises until a later corpus phase adds them.
