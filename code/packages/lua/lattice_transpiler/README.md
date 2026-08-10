# coding-adventures-lattice-transpiler (Lua)

End-to-end Lattice → CSS transpiler.  Takes a Lattice source string (or
file) and returns compiled CSS text.  Wires together `lattice_parser` and
`lattice_ast_to_css` into a single convenience API.

## Usage

```lua
local transpiler = require("coding_adventures.lattice_transpiler")

local css, err = transpiler.transpile([[
  $primary: #4a90d9;

  @mixin button($bg, $fg: white) {
    background: $bg;
    color: $fg;
    padding: 8px 16px;
  }

  .btn {
    @include button($primary);
    &:hover { opacity: 0.9; }
  }
]])

if err then
  io.stderr:write("Error: " .. err .. "\n")
else
  io.write(css)
end
```

Output:

```css
.btn {
  background: #4a90d9;
  color: white;
  padding: 8px 16px;
}

.btn:hover {
  opacity: 0.9;
}
```

## API

### `M.transpile(source) → css, err`

Transpile a Lattice source string.  Returns `(css_string, nil)` on
success or `(nil, error_message)` on failure.

### `M.transpile_file(path) → css, err`

Read a file and transpile it.  Returns `(css_string, nil)` on success
or `(nil, error_message)` if the file cannot be opened or the source
is invalid.

## Pipeline

```
Lattice source
    │
    ▼  lattice_parser.parse()
  AST
    │
    ▼  lattice_ast_to_css.compile()
  CSS text
```

## Standalone build contract

Both build front doors work from an empty LuaRocks tree by installing the exact
checked-in chain `directed_graph -> state_machine -> grammar_tools -> lexer ->
lattice_lexer -> parser -> lattice_parser -> lattice_ast_to_css ->
lattice_transpiler`. Dependency fetching is disabled at every step, which makes
the end-to-end package independent of ambient rocks and remote repositories.
The build then transpiles a real Lattice rule using only the installed tree,
which proves the deployed grammar-data boundary as well as module loading.
