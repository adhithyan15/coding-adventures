# cowsay (Python)

A Python port of the classic `cowsay` / `cowthink`: it wraps a message in a
speech (or thought) bubble and draws an ASCII cow underneath.

```
 _______
< Hello >
 -------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## How it fits in the stack

- Command-line parsing is driven by the shared spec `code/specs/cowsay.json`
  through this language's `cli-builder` package — the same spec every port uses.
- Cow templates are the Perl-heredoc `.cow` files in `code/specs/cows/`.

## Usage

```sh
python3 main.py -f tux "Hello"
```

## Cow-file safety

`-f NAME` is untrusted input that chooses a file to open and print, so it is
validated before use (issue #12169): only a bare file stem is accepted (no `/`,
`\`, `..`, `:` or NUL), and the resolved file must stay inside
`code/specs/cows/`. Anything else silently draws `default.cow`, exactly like an
unknown cow name. The reasoning is written up inline next to the code.

## Testing

```sh
uv venv && uv pip install pytest ruff && uv run --no-project python -m pytest tests/ -v && uv run --no-project ruff check .
```
