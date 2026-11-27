# Unix history simulator (Elixir)

This package models a small First Edition Unix image entirely in memory. Its component API is reusable without the shell adapter:

- `new/1` creates independent kernel state.
- `resolve/2` walks the image and returns an inode or stable error.
- `open/2`, `read/3`, and `close/2` model the descriptor table and kernel calls.
- `list/2` returns directory entries in image order.
- `run_command/1` adapts `cat` and `ls` into portable output and ordered event maps.

Run `mix test` for component checks. The repository BUILD also replays the
shared nine-case fixture through an external adapter; it is test tooling, not a
shipped command. The filesystem, descriptors, and kernel operations use only
Elixir and Erlang standard libraries.
