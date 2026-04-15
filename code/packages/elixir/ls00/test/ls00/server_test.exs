defmodule Ls00.ServerTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Integration tests for the LSP server handlers.

  These tests exercise the handler functions directly (without going through
  the full JSON-RPC transport) to test the LSP logic independently of the
  wire protocol. This approach avoids coupling to the json_rpc package's
  internal encoding details.
  """

  alias Ls00.Handlers
  alias Ls00.Types
  alias CodingAdventures.JsonRpc.Writer

  # ---------------------------------------------------------------------------
  # MockBridge: implements hover and document_symbols
  # ---------------------------------------------------------------------------

  defmodule MockBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(source) do
      tokens =
        source
        |> String.split(~r/\s+/, trim: true)
        |> Enum.with_index()
        |> Enum.map(fn {word, idx} ->
          %Ls00.Types.Token{type: "WORD", value: word, line: 1, column: idx + 1}
        end)

      {:ok, tokens}
    end

    @impl true
    def parse(source) do
      diags =
        if String.contains?(source, "ERROR") do
          [
            %Ls00.Types.Diagnostic{
              range: %Ls00.Types.Range{
                start: %Ls00.Types.Position{line: 0, character: 0},
                end_pos: %Ls00.Types.Position{line: 0, character: 5}
              },
              severity: Ls00.Types.severity_error(),
              message: "syntax error: unexpected ERROR token"
            }
          ]
        else
          []
        end

      {:ok, source, diags}
    end

    @impl true
    def hover(_ast, _pos) do
      {:ok, %Ls00.Types.HoverResult{
        contents: "**main** function",
        range: %Ls00.Types.Range{
          start: %Ls00.Types.Position{line: 0, character: 0},
          end_pos: %Ls00.Types.Position{line: 0, character: 4}
        }
      }}
    end

    @impl true
    def document_symbols(_ast) do
      {:ok, [
        %Ls00.Types.DocumentSymbol{
          name: "main",
          kind: Ls00.Types.symbol_function(),
          range: %Ls00.Types.Range{
            start: %Ls00.Types.Position{line: 0, character: 0},
            end_pos: %Ls00.Types.Position{line: 10, character: 1}
          },
          selection_range: %Ls00.Types.Range{
            start: %Ls00.Types.Position{line: 0, character: 9},
            end_pos: %Ls00.Types.Position{line: 0, character: 13}
          },
          children: [
            %Ls00.Types.DocumentSymbol{
              name: "x",
              kind: Ls00.Types.symbol_variable(),
              range: %Ls00.Types.Range{
                start: %Ls00.Types.Position{line: 1, character: 4},
                end_pos: %Ls00.Types.Position{line: 1, character: 12}
              },
              selection_range: %Ls00.Types.Range{
                start: %Ls00.Types.Position{line: 1, character: 8},
                end_pos: %Ls00.Types.Position{line: 1, character: 9}
              }
            }
          ]
        }
      ]}
    end
  end

  # ---------------------------------------------------------------------------
  # MinimalBridge: only required callbacks
  # ---------------------------------------------------------------------------

  defmodule MinimalBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(_source), do: {:ok, []}

    @impl true
    def parse(source), do: {:ok, source, []}
  end

  # ---------------------------------------------------------------------------
  # FullMockBridge: all optional callbacks
  # ---------------------------------------------------------------------------

  defmodule FullMockBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(source) do
      tokens =
        source
        |> String.split(~r/\s+/, trim: true)
        |> Enum.with_index()
        |> Enum.map(fn {word, idx} ->
          %Ls00.Types.Token{type: "WORD", value: word, line: 1, column: idx + 1}
        end)

      {:ok, tokens}
    end

    @impl true
    def parse(source), do: {:ok, source, []}

    @impl true
    def hover(_ast, _pos) do
      {:ok, %Ls00.Types.HoverResult{contents: "test hover"}}
    end

    @impl true
    def definition(_ast, pos, uri) do
      {:ok, %Ls00.Types.Location{
        uri: uri,
        range: %Ls00.Types.Range{start: pos, end_pos: pos}
      }}
    end

    @impl true
    def references(_ast, pos, uri, _include_decl) do
      {:ok, [%Ls00.Types.Location{
        uri: uri,
        range: %Ls00.Types.Range{start: pos, end_pos: pos}
      }]}
    end

    @impl true
    def completion(_ast, _pos) do
      {:ok, [%Ls00.Types.CompletionItem{
        label: "foo",
        kind: Ls00.Types.completion_function(),
        detail: "() void"
      }]}
    end

    @impl true
    def rename(_ast, pos, new_name) do
      {:ok, %Ls00.Types.WorkspaceEdit{
        changes: %{
          "file:///test.txt" => [
            %Ls00.Types.TextEdit{
              range: %Ls00.Types.Range{start: pos, end_pos: pos},
              new_text: new_name
            }
          ]
        }
      }}
    end

    @impl true
    def semantic_tokens(_source, tokens) do
      result =
        Enum.map(tokens, fn tok ->
          %Ls00.Types.SemanticToken{
            line: tok.line - 1,
            character: tok.column - 1,
            length: String.length(tok.value),
            token_type: "variable",
            modifiers: []
          }
        end)

      {:ok, result}
    end

    @impl true
    def document_symbols(_ast) do
      {:ok, [%Ls00.Types.DocumentSymbol{
        name: "main",
        kind: Ls00.Types.symbol_function(),
        range: %Ls00.Types.Range{
          start: %Ls00.Types.Position{line: 0, character: 0},
          end_pos: %Ls00.Types.Position{line: 10, character: 1}
        },
        selection_range: %Ls00.Types.Range{
          start: %Ls00.Types.Position{line: 0, character: 9},
          end_pos: %Ls00.Types.Position{line: 0, character: 13}
        }
      }]}
    end

    @impl true
    def folding_ranges(_ast) do
      {:ok, [%Ls00.Types.FoldingRange{start_line: 0, end_line: 5, kind: "region"}]}
    end

    @impl true
    def signature_help(_ast, _pos) do
      {:ok, %Ls00.Types.SignatureHelpResult{
        signatures: [
          %Ls00.Types.SignatureInformation{
            label: "foo(a int, b string)",
            parameters: [
              %Ls00.Types.ParameterInformation{label: "a int"},
              %Ls00.Types.ParameterInformation{label: "b string"}
            ]
          }
        ],
        active_signature: 0,
        active_parameter: 0
      }}
    end

    @impl true
    def format(source) do
      {:ok, [
        %Ls00.Types.TextEdit{
          range: %Ls00.Types.Range{
            start: %Ls00.Types.Position{line: 0, character: 0},
            end_pos: %Ls00.Types.Position{line: 999, character: 0}
          },
          new_text: source
        }
      ]}
    end
  end

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------

  # Create a handler state for a given bridge module.
  defp new_state(bridge_module) do
    # Use a StringIO device for the writer (captures notifications output).
    {:ok, out_pid} = StringIO.open("")
    writer = Writer.new(out_pid)

    %Handlers{
      bridge_module: bridge_module,
      writer: writer
    }
  end

  # Initialize the server and open a document.
  defp init_and_open(bridge_module, uri, text) do
    state = new_state(bridge_module)

    # Initialize
    {_result, state} = Handlers.handle_initialize(state, 1, %{
      "processId" => 1234,
      "capabilities" => %{}
    })

    state = Handlers.handle_initialized(state, %{})

    # Open document
    state = Handlers.handle_did_open(state, %{
      "textDocument" => %{
        "uri" => uri,
        "languageId" => "test",
        "version" => 1,
        "text" => text
      }
    })

    state
  end

  # ---------------------------------------------------------------------------
  # Lifecycle tests
  # ---------------------------------------------------------------------------

  test "initialize returns capabilities" do
    state = new_state(MockBridge)

    {result, state} = Handlers.handle_initialize(state, 1, %{
      "processId" => 1234,
      "capabilities" => %{}
    })

    assert is_map(result)
    caps = result["capabilities"]
    assert caps["textDocumentSync"] == 2
    assert caps["hoverProvider"] == true
    assert caps["documentSymbolProvider"] == true

    server_info = result["serverInfo"]
    assert server_info["name"] == "ls00-generic-lsp-server"
    assert server_info["version"] == "0.1.0"
    assert state.initialized == true
  end

  test "shutdown sets shutdown flag" do
    state = new_state(MockBridge)
    {result, state} = Handlers.handle_shutdown(state, 1, %{})

    assert result == nil
    assert state.shutdown == true
  end

  # ---------------------------------------------------------------------------
  # didOpen / diagnostics tests
  # ---------------------------------------------------------------------------

  test "didOpen with error source produces diagnostics" do
    state = new_state(MockBridge)
    {_result, state} = Handlers.handle_initialize(state, 1, %{
      "processId" => 1, "capabilities" => %{}
    })

    state = Handlers.handle_did_open(state, %{
      "textDocument" => %{
        "uri" => "file:///test.txt",
        "languageId" => "test",
        "version" => 1,
        "text" => "hello ERROR world"
      }
    })

    # Verify the parse cache has diagnostics.
    {parse_result, _cache} =
      Ls00.ParseCache.get_or_parse(state.parse_cache, "file:///test.txt", 1, "hello ERROR world", MockBridge)

    assert length(parse_result.diagnostics) > 0
  end

  test "didOpen with clean source produces no diagnostics" do
    state = new_state(MockBridge)
    {_result, state} = Handlers.handle_initialize(state, 1, %{
      "processId" => 1, "capabilities" => %{}
    })

    state = Handlers.handle_did_open(state, %{
      "textDocument" => %{
        "uri" => "file:///clean.txt",
        "languageId" => "test",
        "version" => 1,
        "text" => "hello world"
      }
    })

    {parse_result, _cache} =
      Ls00.ParseCache.get_or_parse(state.parse_cache, "file:///clean.txt", 1, "hello world", MockBridge)

    assert parse_result.diagnostics == []
  end

  # ---------------------------------------------------------------------------
  # didChange test
  # ---------------------------------------------------------------------------

  test "didChange updates document text" do
    state = init_and_open(MockBridge, "file:///test.txt", "hello world")

    state = Handlers.handle_did_change(state, %{
      "textDocument" => %{"uri" => "file:///test.txt", "version" => 2},
      "contentChanges" => [%{"text" => "goodbye world"}]
    })

    {:ok, doc} = Ls00.DocumentManager.get(state.doc_manager, "file:///test.txt")
    assert doc.text == "goodbye world"
    assert doc.version == 2
  end

  # ---------------------------------------------------------------------------
  # didClose test
  # ---------------------------------------------------------------------------

  test "didClose removes document" do
    state = init_and_open(MockBridge, "file:///test.txt", "hello")

    state = Handlers.handle_did_close(state, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert :error = Ls00.DocumentManager.get(state.doc_manager, "file:///test.txt")
  end

  # ---------------------------------------------------------------------------
  # Hover tests
  # ---------------------------------------------------------------------------

  test "hover returns markdown content" do
    state = init_and_open(MockBridge, "file:///test.go", "func main() {}")

    {result, _state} = Handlers.handle_hover(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.go"},
      "position" => %{"line" => 0, "character" => 5}
    })

    assert result != nil
    assert result["contents"]["kind"] == "markdown"
    assert result["contents"]["value"] == "**main** function"
    assert result["range"] != nil
  end

  test "hover returns nil for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_hover(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert result == nil
  end

  # ---------------------------------------------------------------------------
  # Definition test
  # ---------------------------------------------------------------------------

  test "definition returns location" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_definition(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert result != nil
    assert result["uri"] == "file:///test.txt"
  end

  test "definition returns nil for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_definition(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert result == nil
  end

  # ---------------------------------------------------------------------------
  # References test
  # ---------------------------------------------------------------------------

  test "references returns location array" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_references(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0},
      "context" => %{"includeDeclaration" => true}
    })

    assert is_list(result)
    assert length(result) > 0
  end

  test "references returns empty for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_references(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert result == []
  end

  # ---------------------------------------------------------------------------
  # Completion test
  # ---------------------------------------------------------------------------

  test "completion returns items" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "foo")

    {result, _state} = Handlers.handle_completion(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 3}
    })

    assert result["isIncomplete"] == false
    assert is_list(result["items"])
    assert length(result["items"]) > 0
    first = hd(result["items"])
    assert first["label"] == "foo"
  end

  # ---------------------------------------------------------------------------
  # Rename test
  # ---------------------------------------------------------------------------

  test "rename returns workspace edit" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_rename(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0},
      "newName" => "world"
    })

    assert is_map(result["changes"])
    assert Map.has_key?(result["changes"], "file:///test.txt")
  end

  test "rename returns error for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_rename(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0},
      "newName" => "world"
    })

    assert result.code == Ls00.LspErrors.request_failed()
  end

  # ---------------------------------------------------------------------------
  # Document symbol test
  # ---------------------------------------------------------------------------

  test "documentSymbol returns nested symbols" do
    state = init_and_open(MockBridge, "file:///test.go", "func main() { var x = 1 }")

    {result, _state} = Handlers.handle_document_symbol(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.go"}
    })

    assert is_list(result)
    assert length(result) > 0
    first = hd(result)
    assert first["name"] == "main"
    assert first["kind"] == Types.symbol_function()
    assert is_list(first["children"])
    assert length(first["children"]) == 1
    child = hd(first["children"])
    assert child["name"] == "x"
  end

  # ---------------------------------------------------------------------------
  # Semantic tokens test
  # ---------------------------------------------------------------------------

  test "semanticTokens/full returns data" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_semantic_tokens_full(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert is_list(result["data"])
    # "hello world" has 2 tokens, each encoded as a 5-tuple
    assert length(result["data"]) == 10
  end

  test "semanticTokens/full returns empty for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_semantic_tokens_full(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert result["data"] == []
  end

  # ---------------------------------------------------------------------------
  # Folding range test
  # ---------------------------------------------------------------------------

  test "foldingRange returns ranges" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_folding_range(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert is_list(result)
    assert length(result) > 0
    first = hd(result)
    assert first["startLine"] == 0
    assert first["endLine"] == 5
    assert first["kind"] == "region"
  end

  # ---------------------------------------------------------------------------
  # Signature help test
  # ---------------------------------------------------------------------------

  test "signatureHelp returns signatures" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_signature_help(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert is_list(result["signatures"])
    assert length(result["signatures"]) > 0
    assert result["activeSignature"] == 0
    assert result["activeParameter"] == 0
  end

  test "signatureHelp returns nil for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello")

    {result, _state} = Handlers.handle_signature_help(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"},
      "position" => %{"line" => 0, "character" => 0}
    })

    assert result == nil
  end

  # ---------------------------------------------------------------------------
  # Formatting test
  # ---------------------------------------------------------------------------

  test "formatting returns text edits" do
    state = init_and_open(FullMockBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_formatting(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert is_list(result)
    assert length(result) > 0
  end

  test "formatting returns empty for minimal bridge" do
    state = init_and_open(MinimalBridge, "file:///test.txt", "hello world")

    {result, _state} = Handlers.handle_formatting(state, 2, %{
      "textDocument" => %{"uri" => "file:///test.txt"}
    })

    assert result == []
  end

  # ---------------------------------------------------------------------------
  # LSP error codes test
  # ---------------------------------------------------------------------------

  test "LSP error codes have correct values" do
    assert Ls00.LspErrors.server_not_initialized() == -32002
    assert Ls00.LspErrors.unknown_error_code() == -32001
    assert Ls00.LspErrors.request_failed() == -32803
    assert Ls00.LspErrors.server_cancelled() == -32802
    assert Ls00.LspErrors.content_modified() == -32801
    assert Ls00.LspErrors.request_cancelled() == -32800
  end

  # ---------------------------------------------------------------------------
  # Server.new test
  # ---------------------------------------------------------------------------

  test "Server.new creates a server struct" do
    {:ok, in_pid} = StringIO.open("")
    {:ok, out_pid} = StringIO.open("")

    server = Ls00.Server.new(MockBridge, in_pid, out_pid)
    assert server != nil
    # The returned value is a JsonRpc.Server struct.
    assert %CodingAdventures.JsonRpc.Server{} = server
  end
end
