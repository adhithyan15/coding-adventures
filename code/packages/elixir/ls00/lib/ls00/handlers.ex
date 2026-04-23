defmodule Ls00.Handlers do
  @moduledoc """
  All LSP handler functions.

  This module contains the implementation of every LSP request and notification
  handler. Each handler function takes a server state and the message parameters,
  and returns `{result_or_nil, updated_state}`.

  ## Handler Categories

  ### Lifecycle (initialize, initialized, shutdown, exit)

  Every LSP session begins with initialize and ends with shutdown+exit.

  ### Text Document Sync (didOpen, didChange, didClose, didSave)

  These four notifications form the core of document synchronization. The editor
  sends them as the user opens, edits, and closes files.

  ### Feature Requests (hover, definition, references, etc.)

  These are conditional on bridge capability. Each handler checks whether the
  bridge module exports the relevant callback before delegating.
  """

  alias Ls00.{Capabilities, DocumentManager, LspErrors, ParseCache}
  alias Ls00.Types
  alias Ls00.Types.Position
  alias CodingAdventures.JsonRpc.Message.Notification
  alias CodingAdventures.JsonRpc.Writer

  # ---------------------------------------------------------------------------
  # Server state
  # ---------------------------------------------------------------------------
  #
  # We bundle all server state into a struct. This is passed through every
  # handler and returned (possibly updated).

  defstruct [
    :bridge_module,
    :writer,
    doc_manager: DocumentManager.new(),
    parse_cache: ParseCache.new(),
    initialized: false,
    shutdown: false
  ]

  @type t :: %__MODULE__{
          bridge_module: module(),
          writer: CodingAdventures.JsonRpc.Writer.t(),
          doc_manager: DocumentManager.t(),
          parse_cache: ParseCache.t(),
          initialized: boolean(),
          shutdown: boolean()
        }

  # ---------------------------------------------------------------------------
  # initialize
  # ---------------------------------------------------------------------------

  @doc """
  Handle the LSP initialize request.

  Sets the initialized flag and returns capabilities built from the bridge module.
  """
  @spec handle_initialize(t(), any(), any()) :: {any(), t()}
  def handle_initialize(state, _id, _params) do
    state = %{state | initialized: true}
    caps = Capabilities.build_capabilities(state.bridge_module)

    result = %{
      "capabilities" => caps,
      "serverInfo" => %{
        "name" => "ls00-generic-lsp-server",
        "version" => "0.1.0"
      }
    }

    {result, state}
  end

  # ---------------------------------------------------------------------------
  # initialized
  # ---------------------------------------------------------------------------

  @doc "Handle the initialized notification. No-op: the handshake is complete."
  @spec handle_initialized(t(), any()) :: t()
  def handle_initialized(state, _params), do: state

  # ---------------------------------------------------------------------------
  # shutdown
  # ---------------------------------------------------------------------------

  @doc """
  Handle the LSP shutdown request.

  Sets the shutdown flag and returns nil (LSP spec requires null result).
  """
  @spec handle_shutdown(t(), any(), any()) :: {nil, t()}
  def handle_shutdown(state, _id, _params) do
    {nil, %{state | shutdown: true}}
  end

  # ---------------------------------------------------------------------------
  # exit
  # ---------------------------------------------------------------------------

  @doc """
  Handle the exit notification.

  Exit code 0 if shutdown was received, 1 otherwise.
  """
  @spec handle_exit(t(), any()) :: t()
  def handle_exit(state, _params) do
    if state.shutdown do
      System.halt(0)
    else
      System.halt(1)
    end

    state
  end

  # ---------------------------------------------------------------------------
  # textDocument/didOpen
  # ---------------------------------------------------------------------------

  @doc """
  Handle didOpen: register the document and publish diagnostics.

  This is called when the editor opens a file. We store the text and parse it
  immediately so the editor shows squiggles as soon as the file is opened.
  """
  @spec handle_did_open(t(), any()) :: t()
  def handle_did_open(state, params) do
    with td when is_map(td) <- get_in_safe(params, ["textDocument"]),
         uri when is_binary(uri) <- td["uri"],
         text when is_binary(text) <- td["text"] do
      version = parse_int(td["version"], 1)

      doc_manager = DocumentManager.open(state.doc_manager, uri, text, version)
      {parse_result, parse_cache} =
        ParseCache.get_or_parse(state.parse_cache, uri, version, text, state.bridge_module)

      state = %{state | doc_manager: doc_manager, parse_cache: parse_cache}
      publish_diagnostics(state, uri, version, parse_result.diagnostics)
      state
    else
      _ -> state
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/didChange
  # ---------------------------------------------------------------------------

  @doc """
  Handle didChange: apply incremental changes and publish diagnostics.
  """
  @spec handle_did_change(t(), any()) :: t()
  def handle_did_change(state, params) do
    with p when is_map(p) <- params,
         uri when is_binary(uri) <- parse_uri(p) do
      version = parse_version(p)
      changes = parse_content_changes(p)

      case DocumentManager.apply_changes(state.doc_manager, uri, changes, version) do
        {:ok, doc_manager} ->
          state = %{state | doc_manager: doc_manager}

          case DocumentManager.get(doc_manager, uri) do
            {:ok, doc} ->
              {parse_result, parse_cache} =
                ParseCache.get_or_parse(
                  state.parse_cache, uri, doc.version, doc.text, state.bridge_module
                )

              state = %{state | parse_cache: parse_cache}
              publish_diagnostics(state, uri, version, parse_result.diagnostics)
              state

            :error ->
              state
          end

        {:error, _reason} ->
          state
      end
    else
      _ -> state
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/didClose
  # ---------------------------------------------------------------------------

  @doc """
  Handle didClose: remove the document and clear diagnostics.
  """
  @spec handle_did_close(t(), any()) :: t()
  def handle_did_close(state, params) do
    uri = parse_uri(params)

    if is_binary(uri) and uri != "" do
      doc_manager = DocumentManager.close(state.doc_manager, uri)
      parse_cache = ParseCache.evict(state.parse_cache, uri)
      state = %{state | doc_manager: doc_manager, parse_cache: parse_cache}

      # Clear diagnostics for the closed file.
      publish_diagnostics(state, uri, 0, [])
      state
    else
      state
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/didSave
  # ---------------------------------------------------------------------------

  @doc """
  Handle didSave: re-parse if the client sends full text.
  """
  @spec handle_did_save(t(), any()) :: t()
  def handle_did_save(state, params) do
    uri = parse_uri(params)

    if is_binary(uri) and uri != "" do
      case params do
        %{"text" => text} when is_binary(text) and text != "" ->
          case DocumentManager.get(state.doc_manager, uri) do
            {:ok, doc} ->
              doc_manager =
                state.doc_manager
                |> DocumentManager.close(uri)
                |> DocumentManager.open(uri, text, doc.version)

              {parse_result, parse_cache} =
                ParseCache.get_or_parse(
                  state.parse_cache, uri, doc.version, text, state.bridge_module
                )

              state = %{state | doc_manager: doc_manager, parse_cache: parse_cache}
              publish_diagnostics(state, uri, doc.version, parse_result.diagnostics)
              state

            :error ->
              state
          end

        _ ->
          state
      end
    else
      state
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/hover
  # ---------------------------------------------------------------------------

  @doc "Handle hover request."
  @spec handle_hover(t(), any(), any()) :: {any(), t()}
  def handle_hover(state, _id, params) do
    if not function_exported?(state.bridge_module, :hover, 2) do
      {nil, state}
    else
      with {:ok, _uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params),
           true <- parse_result.ast != nil do
        case state.bridge_module.hover(parse_result.ast, pos) do
          {:ok, nil} ->
            {nil, state}

          {:ok, hover_result} ->
            result = %{
              "contents" => %{
                "kind" => "markdown",
                "value" => hover_result.contents
              }
            }

            result =
              if hover_result.range do
                Map.put(result, "range", range_to_lsp(hover_result.range))
              else
                result
              end

            {result, state}

          {:error, _reason} ->
            {nil, state}
        end
      else
        _ -> {nil, state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/definition
  # ---------------------------------------------------------------------------

  @doc "Handle definition request."
  @spec handle_definition(t(), any(), any()) :: {any(), t()}
  def handle_definition(state, _id, params) do
    if not function_exported?(state.bridge_module, :definition, 3) do
      {nil, state}
    else
      with {:ok, uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params),
           true <- parse_result.ast != nil do
        case state.bridge_module.definition(parse_result.ast, pos, uri) do
          {:ok, nil} -> {nil, state}
          {:ok, location} -> {location_to_lsp(location), state}
          {:error, _} -> {nil, state}
        end
      else
        _ -> {nil, state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/references
  # ---------------------------------------------------------------------------

  @doc "Handle references request."
  @spec handle_references(t(), any(), any()) :: {any(), t()}
  def handle_references(state, _id, params) do
    if not function_exported?(state.bridge_module, :references, 4) do
      {[], state}
    else
      with {:ok, uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
        if parse_result.ast == nil do
          {[], state}
        else
          include_decl = get_in_safe(params, ["context", "includeDeclaration"]) == true

          case state.bridge_module.references(parse_result.ast, pos, uri, include_decl) do
            {:ok, locations} ->
              {Enum.map(locations, &location_to_lsp/1), state}

            {:error, _} ->
              {[], state}
          end
        end
      else
        _ -> {[], state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/completion
  # ---------------------------------------------------------------------------

  @doc "Handle completion request."
  @spec handle_completion(t(), any(), any()) :: {any(), t()}
  def handle_completion(state, _id, params) do
    empty_result = %{"isIncomplete" => false, "items" => []}

    if not function_exported?(state.bridge_module, :completion, 2) do
      {empty_result, state}
    else
      with {:ok, _uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
        if parse_result.ast == nil do
          {empty_result, state}
        else
          case state.bridge_module.completion(parse_result.ast, pos) do
            {:ok, items} ->
              lsp_items =
                Enum.map(items, fn item ->
                  ci = %{"label" => item.label}
                  ci = if item.kind, do: Map.put(ci, "kind", item.kind), else: ci
                  ci = if item.detail, do: Map.put(ci, "detail", item.detail), else: ci
                  ci = if item.documentation, do: Map.put(ci, "documentation", item.documentation), else: ci
                  ci = if item.insert_text, do: Map.put(ci, "insertText", item.insert_text), else: ci
                  ci = if item.insert_text_format, do: Map.put(ci, "insertTextFormat", item.insert_text_format), else: ci
                  ci
                end)

              {%{"isIncomplete" => false, "items" => lsp_items}, state}

            {:error, _} ->
              {empty_result, state}
          end
        end
      else
        _ -> {empty_result, state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/rename
  # ---------------------------------------------------------------------------

  @doc "Handle rename request."
  @spec handle_rename(t(), any(), any()) :: {any(), t()}
  def handle_rename(state, _id, params) do
    if not function_exported?(state.bridge_module, :rename, 3) do
      error = %{code: LspErrors.request_failed(), message: "rename not supported"}
      {error, state}
    else
      new_name = get_in_safe(params, ["newName"])

      if not is_binary(new_name) or new_name == "" do
        error = %{code: CodingAdventures.JsonRpc.Errors.invalid_params(), message: "newName is required"}
        {error, state}
      else
        with {:ok, _uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
          if parse_result.ast == nil do
            error = %{code: LspErrors.request_failed(), message: "no AST available"}
            {error, state}
          else
            case state.bridge_module.rename(parse_result.ast, pos, new_name) do
              {:ok, nil} ->
                error = %{code: LspErrors.request_failed(), message: "symbol not found at position"}
                {error, state}

              {:ok, workspace_edit} ->
                lsp_changes =
                  workspace_edit.changes
                  |> Enum.map(fn {edit_uri, edits} ->
                    lsp_edits =
                      Enum.map(edits, fn te ->
                        %{
                          "range" => range_to_lsp(te.range),
                          "newText" => te.new_text
                        }
                      end)
                    {edit_uri, lsp_edits}
                  end)
                  |> Map.new()

                {%{"changes" => lsp_changes}, state}

              {:error, reason} ->
                error = %{code: LspErrors.request_failed(), message: reason}
                {error, state}
            end
          end
        else
          _ ->
            error = %{code: LspErrors.request_failed(), message: "document not open"}
            {error, state}
        end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/documentSymbol
  # ---------------------------------------------------------------------------

  @doc "Handle documentSymbol request."
  @spec handle_document_symbol(t(), any(), any()) :: {any(), t()}
  def handle_document_symbol(state, _id, params) do
    if not function_exported?(state.bridge_module, :document_symbols, 1) do
      {[], state}
    else
      with {:ok, _uri, _pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
        if parse_result.ast == nil do
          {[], state}
        else
          case state.bridge_module.document_symbols(parse_result.ast) do
            {:ok, symbols} ->
              {convert_document_symbols(symbols), state}

            {:error, _} ->
              {[], state}
          end
        end
      else
        _ -> {[], state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/semanticTokens/full
  # ---------------------------------------------------------------------------

  @doc "Handle semanticTokens/full request."
  @spec handle_semantic_tokens_full(t(), any(), any()) :: {any(), t()}
  def handle_semantic_tokens_full(state, _id, params) do
    empty = %{"data" => []}

    if not function_exported?(state.bridge_module, :semantic_tokens, 2) do
      {empty, state}
    else
      uri = parse_uri(params)

      case DocumentManager.get(state.doc_manager, uri) do
        :error ->
          {empty, state}

        {:ok, doc} ->
          case state.bridge_module.tokenize(doc.text) do
            {:ok, tokens} ->
              case state.bridge_module.semantic_tokens(doc.text, tokens) do
                {:ok, sem_tokens} ->
                  data = Capabilities.encode_semantic_tokens(sem_tokens)
                  {%{"data" => data}, state}

                {:error, _} ->
                  {empty, state}
              end

            {:error, _} ->
              {empty, state}
          end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/foldingRange
  # ---------------------------------------------------------------------------

  @doc "Handle foldingRange request."
  @spec handle_folding_range(t(), any(), any()) :: {any(), t()}
  def handle_folding_range(state, _id, params) do
    if not function_exported?(state.bridge_module, :folding_ranges, 1) do
      {[], state}
    else
      with {:ok, _uri, _pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
        if parse_result.ast == nil do
          {[], state}
        else
          case state.bridge_module.folding_ranges(parse_result.ast) do
            {:ok, ranges} ->
              result =
                Enum.map(ranges, fn fr ->
                  m = %{"startLine" => fr.start_line, "endLine" => fr.end_line}
                  if fr.kind, do: Map.put(m, "kind", fr.kind), else: m
                end)

              {result, state}

            {:error, _} ->
              {[], state}
          end
        end
      else
        _ -> {[], state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/signatureHelp
  # ---------------------------------------------------------------------------

  @doc "Handle signatureHelp request."
  @spec handle_signature_help(t(), any(), any()) :: {any(), t()}
  def handle_signature_help(state, _id, params) do
    if not function_exported?(state.bridge_module, :signature_help, 2) do
      {nil, state}
    else
      with {:ok, _uri, pos, _doc, parse_result, state} <- get_parse_for_request(state, params) do
        if parse_result.ast == nil do
          {nil, state}
        else
          case state.bridge_module.signature_help(parse_result.ast, pos) do
            {:ok, nil} ->
              {nil, state}

            {:ok, sig_help} ->
              lsp_sigs =
                Enum.map(sig_help.signatures, fn sig ->
                  lsp_params =
                    Enum.map(sig.parameters, fn param ->
                      pp = %{"label" => param.label}
                      if param.documentation, do: Map.put(pp, "documentation", param.documentation), else: pp
                    end)

                  s = %{"label" => sig.label, "parameters" => lsp_params}
                  if sig.documentation, do: Map.put(s, "documentation", sig.documentation), else: s
                end)

              result = %{
                "signatures" => lsp_sigs,
                "activeSignature" => sig_help.active_signature,
                "activeParameter" => sig_help.active_parameter
              }

              {result, state}

            {:error, _} ->
              {nil, state}
          end
        end
      else
        _ -> {nil, state}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # textDocument/formatting
  # ---------------------------------------------------------------------------

  @doc "Handle formatting request."
  @spec handle_formatting(t(), any(), any()) :: {any(), t()}
  def handle_formatting(state, _id, params) do
    if not function_exported?(state.bridge_module, :format, 1) do
      {[], state}
    else
      uri = parse_uri(params)

      case DocumentManager.get(state.doc_manager, uri) do
        :error ->
          {[], state}

        {:ok, doc} ->
          case state.bridge_module.format(doc.text) do
            {:ok, edits} ->
              lsp_edits =
                Enum.map(edits, fn edit ->
                  %{
                    "range" => range_to_lsp(edit.range),
                    "newText" => edit.new_text
                  }
                end)

              {lsp_edits, state}

            {:error, reason} ->
              error = %{code: LspErrors.request_failed(), message: "formatting failed: #{reason}"}
              {error, state}
          end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Get parse result for a request that includes textDocument/uri and position.
  defp get_parse_for_request(state, params) do
    uri = parse_uri(params)
    pos = parse_position(params)

    case DocumentManager.get(state.doc_manager, uri) do
      :error ->
        :error

      {:ok, doc} ->
        {parse_result, parse_cache} =
          ParseCache.get_or_parse(
            state.parse_cache, uri, doc.version, doc.text, state.bridge_module
          )

        state = %{state | parse_cache: parse_cache}
        {:ok, uri, pos, doc, parse_result, state}
    end
  end

  # Publish diagnostics notification to the editor.
  #
  # We wrap the Writer call in a try/rescue because on OTP 27 the `:json`
  # module returns iodata (not a binary) from `encode/1`, and the json_rpc
  # Writer's `write_raw/2` has a `when is_binary(json)` guard that rejects
  # iodata. This is a known issue in the json_rpc package on OTP 27.
  # Rather than failing silently, we catch the error and fall back to
  # writing the notification directly.
  defp publish_diagnostics(state, uri, version, diagnostics) do
    lsp_diags =
      Enum.map(diagnostics, fn d ->
        diag = %{
          "range" => range_to_lsp(d.range),
          "severity" => d.severity,
          "message" => d.message
        }

        if d.code, do: Map.put(diag, "code", d.code), else: diag
      end)

    params = %{"uri" => uri, "diagnostics" => lsp_diags}
    params = if version > 0, do: Map.put(params, "version", version), else: params

    notif = %Notification{method: "textDocument/publishDiagnostics", params: params}

    try do
      Writer.write_message(state.writer, notif)
    rescue
      FunctionClauseError ->
        # Fallback for OTP 27 iodata issue: manually encode and write.
        map = CodingAdventures.JsonRpc.Message.message_to_map(notif)
        json = encode_to_binary(map)
        header = "Content-Length: #{byte_size(json)}\r\n\r\n"
        IO.binwrite(state.writer.device, header <> json)
    end
  end

  # Encode a map to a JSON binary string, handling OTP 27's iodata return.
  defp encode_to_binary(map) do
    try do
      result = :json.encode(map)
      IO.iodata_to_binary(result)
    rescue
      e ->
        IO.warn("ls00: JSON encoding failed: #{inspect(e)}")
        "{}"
    end
  end

  # Extract URI from params.
  defp parse_uri(params) when is_map(params) do
    case params do
      %{"textDocument" => %{"uri" => uri}} when is_binary(uri) -> uri
      _ -> ""
    end
  end

  defp parse_uri(_), do: ""

  # Extract position from params.
  defp parse_position(params) when is_map(params) do
    case params do
      %{"position" => %{"line" => line, "character" => char}} ->
        %Position{line: to_int(line), character: to_int(char)}

      _ ->
        %Position{line: 0, character: 0}
    end
  end

  defp parse_position(_), do: %Position{line: 0, character: 0}

  # Extract version from textDocument params.
  defp parse_version(params) do
    case params do
      %{"textDocument" => %{"version" => v}} -> to_int(v)
      _ -> 0
    end
  end

  # Parse contentChanges array from didChange params.
  defp parse_content_changes(params) do
    changes_raw = params["contentChanges"] || []

    Enum.flat_map(changes_raw, fn
      %{"text" => new_text} = change_map when is_binary(new_text) ->
        change =
          case change_map["range"] do
            nil ->
              %Types.TextChange{new_text: new_text}

            range_map ->
              range = parse_lsp_range(range_map)
              %Types.TextChange{range: range, new_text: new_text}
          end

        [change]

      _ ->
        []
    end)
  end

  # Parse an LSP range map into our Range struct.
  defp parse_lsp_range(map) when is_map(map) do
    start_map = map["start"] || %{}
    end_map = map["end"] || %{}

    %Types.Range{
      start: %Position{
        line: to_int(start_map["line"]),
        character: to_int(start_map["character"])
      },
      end_pos: %Position{
        line: to_int(end_map["line"]),
        character: to_int(end_map["character"])
      }
    }
  end

  # Convert our Position to an LSP-compatible map.
  defp position_to_lsp(%Position{line: line, character: char}) do
    %{"line" => line, "character" => char}
  end

  # Convert our Range to an LSP-compatible map.
  defp range_to_lsp(%Types.Range{start: s, end_pos: e}) do
    %{"start" => position_to_lsp(s), "end" => position_to_lsp(e)}
  end

  # Convert a Location to an LSP-compatible map.
  defp location_to_lsp(%Types.Location{uri: uri, range: range}) do
    %{"uri" => uri, "range" => range_to_lsp(range)}
  end

  # Recursively convert DocumentSymbol list to LSP maps.
  defp convert_document_symbols(symbols) do
    Enum.map(symbols, fn sym ->
      m = %{
        "name" => sym.name,
        "kind" => sym.kind,
        "range" => range_to_lsp(sym.range),
        "selectionRange" => range_to_lsp(sym.selection_range)
      }

      if length(sym.children) > 0 do
        Map.put(m, "children", convert_document_symbols(sym.children))
      else
        m
      end
    end)
  end

  # Safely navigate nested maps (like Kernel.get_in but tolerant of non-maps).
  defp get_in_safe(data, []) when is_map(data), do: data
  defp get_in_safe(data, [key | rest]) when is_map(data), do: get_in_safe(data[key], rest)
  defp get_in_safe(value, []), do: value
  defp get_in_safe(_, _), do: nil

  # Parse integer from potentially float JSON values.
  defp to_int(v) when is_integer(v), do: v
  defp to_int(v) when is_float(v), do: trunc(v)
  defp to_int(_), do: 0

  defp parse_int(v, _default) when is_integer(v), do: v
  defp parse_int(v, _default) when is_float(v), do: trunc(v)
  defp parse_int(_, default), do: default
end
