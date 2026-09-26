### Fixed — list story fixtures reach the generated shell (#15428)

`csharp_literal_for_fixture` renders a text-list fixture as a typed
collection initializer:

- `new List<string> { … }` for a list of text;
- `new List<IReadOnlyList<string>> { new List<string> { … } }` for a list of
  rows.

Both are the types the shell's stub already uses. A shape that does not
match the slot keeps the stub.

