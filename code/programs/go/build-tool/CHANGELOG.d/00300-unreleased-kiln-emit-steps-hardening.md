## Unreleased — `-emit-kiln-steps` review hardening (KILN04 stage 4a)

- Only packages that have a BUILD file for the target OS are exported (a
  package whose only file is `BUILD_windows` is no longer listed for Linux or
  macOS; the real repository now exports 5554 of its 5555 packages on a
  non-Windows host).
- A step line that is empty, not valid UTF-8 or contains U+FFFD is an error
  naming the package (encoding/json would have mapped different invalid bytes
  to the same text); a package directory outside the root is an error.
- `-emit-kiln-steps` and `-emit-kiln-graph` naming the same file (after
  `Abs` and `Clean`) is a usage error.
- Test-only package variable `requireKilnConsumers` lets the whole CLI path of
  both exports run on a fixture tree; it is not a flag.
- Tests: `TestPoCStepsCollide`, BUILD_windows-only omission, outside-root
  refusal, same-path refusal, and the CLI success path with byte-stability.
