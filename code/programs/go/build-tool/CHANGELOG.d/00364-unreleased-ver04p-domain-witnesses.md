# Unreleased — VER04p domain witness gate

- The build tool now selects `rust/warrant-grammar` when the VER04p domain
  document, its bounded exact-rational diagnostic, or its unittest changes.
  That package's Unix and Windows BUILD files run the diagnostic tests, and
  repository-input tests pin both positive and negative path selection. This
  keeps the specification witnesses in CI without selecting unrelated float
  implementation packages.
