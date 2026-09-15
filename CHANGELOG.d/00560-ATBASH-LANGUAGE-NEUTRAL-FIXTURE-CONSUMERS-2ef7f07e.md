### Atbash language-neutral fixture consumers

- Added a bounded stdlib-only generator and fail-closed drift gate that turns
  all six normative Atbash objects into native tests for every established
  implementation lane, with complete expected-text assertions and no new
  production authority.
- Declared the Elixir Atbash hosted-Windows build exception after CI exposed
  that the Windows runner intentionally has no Erlang/Elixir toolchain.

