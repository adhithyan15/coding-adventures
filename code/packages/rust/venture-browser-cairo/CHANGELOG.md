# Changelog

## Unreleased

- Load and atomically persist the shared bookmark catalog for generated Qt,
  Flutter, and Compose hosts, with durable restart acceptance in the common
  Cairo bridge.
- Extract the shared Venture browser controller, Cairo renderer, and native C
  ABI compatibility exports from the Qt adapter into a backend-neutral crate.
