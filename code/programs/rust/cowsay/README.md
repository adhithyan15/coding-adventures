# cowsay (Rust)

A Rust port of the classic `cowsay` / `cowthink`: it wraps a message in a
speech (or thought) bubble and draws an ASCII cow underneath. On Apple targets
it can also render the result to a PNG through the paint-metal pipeline
(`--png <PATH>`).

## How it fits in the stack

- Command-line parsing is driven by the shared spec `code/specs/cowsay.json`
  through the `cli-builder` crate — the same spec every port uses.
- Cow templates are the Perl-heredoc `.cow` files in `code/specs/cows/`.
- The PNG path describes the art as `layout-ir` text and renders it with
  `layout-to-paint` + `paint-metal`.

## Usage

```sh
cargo run -- -f tux "Hello"
cargo run -- --png cow.png --random-text   # macOS only
```

## Cow-file safety

`-f NAME` is untrusted input that chooses a file to open and print, so it is
validated before use (issue #12169): only a bare file stem is accepted (no `/`,
`\`, `..`, `:` or NUL), and the canonicalized file must stay inside
`code/specs/cows/`. Anything else silently draws `default.cow`, exactly like an
unknown cow name. See the comment block above `is_safe_cow_name` in
`src/main.rs`.

## Testing

```sh
cargo test
```
