# Changelog

## Unreleased

### Added

- Initial `conduit-hello` demo program.
- `GET /` route returning `"Hello from Conduit!"`.
- `GET /hello/:name` route returning `"Hello <name>"` — demonstrates Sinatra-style
  named route parameters resolved by the Rust `web-core` router.
- `BUILD` file for the project build tool.
- `README.md` explaining the stack and how to run.
