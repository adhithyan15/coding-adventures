---
category: Repo policy / workflow reminders
---

# Discover package paths before reading focused X.509 APIs

A focused audit command guessed that the server-identity crate lived at a
package path derived from its Rust crate name, so the read failed before it
could inspect the API. Use `rg --files` to discover the tracked path first,
then pass only confirmed paths to focused readers such as `sed`.
