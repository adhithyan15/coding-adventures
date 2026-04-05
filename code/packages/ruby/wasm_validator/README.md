# wasm-validator (Ruby)

WebAssembly 1.0 structural validator for Ruby. Checks parsed WASM modules for semantic correctness before instantiation.

## What It Validates

- WASM 1.0 single-memory and single-table constraints
- Memory limit bounds (max 65536 pages = 4 GiB)
- Memory limit ordering (min <= max)
- Export name uniqueness

## Usage

```ruby
require "coding_adventures_wasm_validator"

validated = CodingAdventures::WasmValidator.validate(wasm_module)
# Returns a ValidatedModule with resolved func_types
# Raises ValidationError on failures
```

## Dependencies

- wasm-types (WASM type definitions)

## Development

```bash
bundle install
bundle exec rake test
```
