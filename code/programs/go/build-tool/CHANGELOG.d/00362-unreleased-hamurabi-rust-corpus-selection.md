# 00362 — Unreleased: pin Rust Hamurabi corpus selection

Added a build-tool regression test proving that edits under the exact
`hamurabi-v1` fixture path select the Rust Hamurabi package, that the versioned
lookalikes do not match, and that a missing registered consumer fails closed.
This keeps the new normal Cargo integration test connected to repository input
selection.
