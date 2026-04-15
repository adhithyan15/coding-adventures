defmodule Ls00 do
  @moduledoc """
  A generic Language Server Protocol (LSP) framework.

  ## What is the Language Server Protocol?

  When you open a source file in VS Code and see red squiggles under syntax
  errors, autocomplete suggestions, or "Go to Definition" -- none of that is
  built into the editor. It comes from a *language server*: a separate process
  that communicates with the editor over the Language Server Protocol.

  LSP was invented by Microsoft to solve the M x N problem:

      M editors x N languages = M x N integrations to write

  With LSP, each language writes one server, and every LSP-aware editor gets
  all features automatically. This package is the *generic* half -- it handles
  all the protocol boilerplate. A language author only writes the
  `Ls00.LanguageBridge` behaviour that connects their lexer/parser to this
  framework.

  ## Architecture

      Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs

  ## JSON-RPC over stdio

  Like the Debug Adapter Protocol (DAP), LSP speaks JSON-RPC over stdio.
  Each message is Content-Length-framed (same format as HTTP headers). The
  underlying transport is handled by the `coding_adventures_json_rpc` package.

  ## How to use this package

  1. Implement the `Ls00.LanguageBridge` behaviour (and any optional callbacks)
     for your language.
  2. Call `Ls00.Server.new(bridge_module, :stdio, :stdio)`.
  3. Call `Ls00.Server.serve(server)` -- it blocks until the editor closes the
     connection.

  ## Module Map

  | Module               | Role                                          |
  |----------------------|-----------------------------------------------|
  | `Ls00`               | Top-level convenience (this file)             |
  | `Ls00.Types`         | Structs + typespecs for all LSP data types    |
  | `Ls00.LanguageBridge`| `@behaviour` with required + optional callbacks|
  | `Ls00.DocumentManager`| Document tracking + UTF-16 conversion        |
  | `Ls00.ParseCache`    | Cache with {uri, version} key                 |
  | `Ls00.Capabilities`  | build_capabilities + semantic token encoding   |
  | `Ls00.LspErrors`     | Error code constants                          |
  | `Ls00.Server`        | LspServer wiring everything together          |
  | `Ls00.Handlers`      | All handler functions                         |
  """
end
