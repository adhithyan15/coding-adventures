defmodule Ls00.Server do
  @moduledoc """
  LspServer -- the main coordinator that wires everything together.

  The server connects:
    - The `Ls00.LanguageBridge` behaviour (language-specific logic)
    - The `Ls00.DocumentManager` (tracks open file contents)
    - The `Ls00.ParseCache` (avoids redundant parses)
    - The `CodingAdventures.JsonRpc.Server` (protocol layer)

  It registers all LSP request and notification handlers with the JSON-RPC
  server, then calls `serve/1` to start the blocking read-dispatch-write loop.

  ## Server Lifecycle

      Client (editor)              Server (us)
        |                               |
        |--initialize-------------->    |  store clientInfo, return capabilities
        | <-----------------result--    |
        |                               |
        |--initialized (notif)----->    |  no-op (handshake complete)
        |                               |
        |--textDocument/didOpen---->    |  open doc, parse, push diagnostics
        |--textDocument/didChange-->    |  apply change, re-parse, push diagnostics
        |--textDocument/hover------>    |  get parse result, call bridge.hover
        | <-----------------result--    |
        |                               |
        |--shutdown---------------->    |  set shutdown flag, return null
        |--exit (notif)------------>    |  System.halt(0) or System.halt(1)

  ## Sending Notifications to the Editor

  The JSON-RPC Server handles request/response pairs. But the LSP server also
  needs to PUSH notifications to the editor (e.g., publishDiagnostics). We do
  this by holding a reference to the JSON-RPC MessageWriter.

  ## Usage

      server = Ls00.Server.new(MyBridge, :stdio, :stdio)
      Ls00.Server.serve(server)   # blocks until EOF
  """

  alias Ls00.Handlers
  alias CodingAdventures.JsonRpc

  @doc """
  Create a new LspServer that reads from `in_device` and writes to `out_device`.

  `bridge_module` is a module that implements the `Ls00.LanguageBridge` behaviour.

  ## Example

      server = Ls00.Server.new(MyBridge, :stdio, :stdio)
  """
  @spec new(module(), any(), any()) :: JsonRpc.Server.t()
  def new(bridge_module, in_device, out_device) do
    writer = JsonRpc.Writer.new(out_device)

    # Initialize the handler state. This bundles document manager, parse cache,
    # and server flags into one struct that is threaded through every handler.
    handler_state = %Handlers{
      bridge_module: bridge_module,
      writer: writer
    }

    # We store the handler state in a mutable reference (Agent) so the JSON-RPC
    # server's stateless handler closures can update it. The JSON-RPC server
    # expects `fn(id, params) -> result` for request handlers and
    # `fn(params) -> :ok` for notification handlers. We close over the Agent
    # pid to thread state through these closures.
    {:ok, agent} = Agent.start_link(fn -> handler_state end)

    rpc_server = JsonRpc.Server.new(in_device, out_device)

    # ── Lifecycle ──────────────────────────────────────────────────────────
    rpc_server = JsonRpc.Server.on_request(rpc_server, "initialize", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        {result, new_state} = Handlers.handle_initialize(state, id, params)
        {result, new_state}
      end)
    end)

    rpc_server = JsonRpc.Server.on_notification(rpc_server, "initialized", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_initialized(state, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "shutdown", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        {result, new_state} = Handlers.handle_shutdown(state, id, params)
        {result, new_state}
      end)
    end)

    rpc_server = JsonRpc.Server.on_notification(rpc_server, "exit", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_exit(state, params)
      end)
    end)

    # ── Text document synchronization ──────────────────────────────────────
    rpc_server = JsonRpc.Server.on_notification(rpc_server, "textDocument/didOpen", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_did_open(state, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_notification(rpc_server, "textDocument/didChange", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_did_change(state, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_notification(rpc_server, "textDocument/didClose", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_did_close(state, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_notification(rpc_server, "textDocument/didSave", fn params ->
      Agent.update(agent, fn state ->
        Handlers.handle_did_save(state, params)
      end)
    end)

    # ── Feature requests ───────────────────────────────────────────────────
    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/hover", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_hover(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/definition", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_definition(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/references", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_references(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/completion", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_completion(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/rename", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_rename(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/documentSymbol", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_document_symbol(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/semanticTokens/full", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_semantic_tokens_full(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/foldingRange", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_folding_range(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/signatureHelp", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_signature_help(state, id, params)
      end)
    end)

    rpc_server = JsonRpc.Server.on_request(rpc_server, "textDocument/formatting", fn id, params ->
      Agent.get_and_update(agent, fn state ->
        Handlers.handle_formatting(state, id, params)
      end)
    end)

    rpc_server
  end

  @doc """
  Start the blocking JSON-RPC read-dispatch-write loop.

  This call blocks until the editor closes the connection (EOF on stdin).
  """
  @spec serve(JsonRpc.Server.t()) :: :ok
  def serve(rpc_server) do
    JsonRpc.Server.serve(rpc_server)
  end
end
