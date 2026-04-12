# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# LSP Server Integration Tests
# ================================================================
#
# These tests feed JSON-RPC messages through the full pipeline using
# IO.pipe. The server runs in a thread; the test feeds messages and
# reads responses.
#
# ================================================================

class TestServer < Minitest::Test
  # Verify the constructor returns a usable server.
  def test_new_server_creates_server
    bridge = MockBridge.new
    in_io = StringIO.new("")
    out_io = StringIO.new
    server = CodingAdventures::Ls00::LspServer.new(bridge, in_io, out_io)
    refute_nil server
  end

  # Test initialize handler returns capabilities.
  def test_handler_initialize
    bridge = MockBridge.new
    bridge.hover_result = CodingAdventures::Ls00::HoverResult.new(contents: "test")
    writer, reader, thread, in_w = pipe_server(bridge)

    resp = send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1234,
      "capabilities" => {}
    })

    refute_nil resp.result
    caps = resp.result["capabilities"]
    refute_nil caps
    refute_nil caps["textDocumentSync"]
    refute_nil caps["hoverProvider"]

    server_info = resp.result["serverInfo"]
    refute_nil server_info
    assert_equal "ls00-generic-lsp-server", server_info["name"]

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test that opening a file with errors causes publishDiagnostics.
  def test_handler_did_open_publishes_diagnostics
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    # Initialize first
    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})

    # Open a file with an error
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt",
        "languageId" => "test",
        "version" => 1,
        "text" => "hello ERROR world"
      }
    })

    # Expect publishDiagnostics notification
    notif = read_rpc_notification(reader)
    assert_equal "textDocument/publishDiagnostics", notif.method

    params = notif.params
    assert_equal "file:///test.txt", params["uri"]
    diags = params["diagnostics"]
    refute_empty diags

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test that a clean file produces empty diagnostics.
  def test_handler_did_open_clean_file
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})

    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///clean.txt",
        "languageId" => "test",
        "version" => 1,
        "text" => "hello world"
      }
    })

    notif = read_rpc_notification(reader)
    assert_equal "textDocument/publishDiagnostics", notif.method
    diags = notif.params["diagnostics"]
    assert_empty diags

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test hover handler end-to-end.
  def test_handler_hover
    bridge = MockBridge.new
    bridge.hover_result = CodingAdventures::Ls00::HoverResult.new(
      contents: "**main** function",
      range: CodingAdventures::Ls00::LspRange.new(
        start: CodingAdventures::Ls00::Position.new(line: 0, character: 0),
        end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 4)
      )
    )
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.go", "languageId" => "go",
        "version" => 1, "text" => "func main() {}"
      }
    })
    read_rpc_notification(reader) # consume publishDiagnostics

    resp = send_rpc_request(writer, reader, 2, "textDocument/hover", {
      "textDocument" => { "uri" => "file:///test.go" },
      "position" => { "line" => 0, "character" => 5 }
    })

    refute_nil resp.result
    contents = resp.result["contents"]
    assert_equal "markdown", contents["kind"]
    assert_equal "**main** function", contents["value"]

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test that a minimal bridge returns null hover.
  def test_handler_hover_no_bridge
    bridge = MinimalBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/hover", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 0 }
    })

    assert_nil resp.result

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test documentSymbol handler.
  def test_handler_document_symbol
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.go", "languageId" => "go",
        "version" => 1, "text" => "func main() { var x = 1 }"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/documentSymbol", {
      "textDocument" => { "uri" => "file:///test.go" }
    })

    result = resp.result
    refute_nil result
    assert_kind_of Array, result
    refute_empty result
    assert_equal "main", result[0]["name"]

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test semanticTokens/full handler.
  def test_handler_semantic_tokens_full
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello world"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/semanticTokens/full", {
      "textDocument" => { "uri" => "file:///test.txt" }
    })

    refute_nil resp.result
    assert resp.result.key?("data")

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test definition handler.
  def test_handler_definition
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello world"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/definition", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 0 }
    })

    refute_nil resp.result
    assert_equal "file:///test.txt", resp.result["uri"]

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test references handler.
  def test_handler_references
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/references", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 0 },
      "context" => { "includeDeclaration" => true }
    })

    result = resp.result
    assert_kind_of Array, result
    refute_empty result

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test completion handler.
  def test_handler_completion
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "foo"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/completion", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 3 }
    })

    refute_nil resp.result
    items = resp.result["items"]
    assert_kind_of Array, items
    refute_empty items

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test rename handler.
  def test_handler_rename
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "let x = 1"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/rename", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 4 },
      "newName" => "y"
    })

    refute_nil resp.result
    refute_nil resp.result["changes"]

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test foldingRange handler.
  def test_handler_folding_range
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "func main() {\n  hello\n}"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/foldingRange", {
      "textDocument" => { "uri" => "file:///test.txt" }
    })

    result = resp.result
    assert_kind_of Array, result
    refute_empty result

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test signatureHelp handler.
  def test_handler_signature_help
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "foo("
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/signatureHelp", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "position" => { "line" => 0, "character" => 4 }
    })

    refute_nil resp.result
    sigs = resp.result["signatures"]
    assert_kind_of Array, sigs
    refute_empty sigs

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test formatting handler.
  def test_handler_formatting
    bridge = FullMockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello  world"
      }
    })
    read_rpc_notification(reader)

    resp = send_rpc_request(writer, reader, 2, "textDocument/formatting", {
      "textDocument" => { "uri" => "file:///test.txt" },
      "options" => { "tabSize" => 2, "insertSpaces" => true }
    })

    result = resp.result
    assert_kind_of Array, result
    refute_empty result

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test didChange updates the document and republishes diagnostics.
  def test_handler_did_change
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello world"
      }
    })
    read_rpc_notification(reader) # publishDiagnostics for open

    # Change the document to add "ERROR"
    send_rpc_notification(writer, "textDocument/didChange", {
      "textDocument" => { "uri" => "file:///test.txt", "version" => 2 },
      "contentChanges" => [
        { "text" => "hello ERROR world" }
      ]
    })
    notif = read_rpc_notification(reader)
    assert_equal "textDocument/publishDiagnostics", notif.method
    diags = notif.params["diagnostics"]
    refute_empty diags

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test didClose clears diagnostics.
  def test_handler_did_close
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })
    send_rpc_notification(writer, "initialized", {})
    send_rpc_notification(writer, "textDocument/didOpen", {
      "textDocument" => {
        "uri" => "file:///test.txt", "languageId" => "test",
        "version" => 1, "text" => "hello"
      }
    })
    read_rpc_notification(reader)

    send_rpc_notification(writer, "textDocument/didClose", {
      "textDocument" => { "uri" => "file:///test.txt" }
    })
    notif = read_rpc_notification(reader)
    assert_equal "textDocument/publishDiagnostics", notif.method
    diags = notif.params["diagnostics"]
    assert_empty diags

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test shutdown handler returns null.
  def test_handler_shutdown
    bridge = MockBridge.new
    writer, reader, thread, in_w = pipe_server(bridge)

    send_rpc_request(writer, reader, 1, "initialize", {
      "processId" => 1, "capabilities" => {}
    })

    resp = send_rpc_request(writer, reader, 2, "shutdown", nil)
    assert_nil resp.result

    in_w.close rescue nil # rubocop:disable Style/RescueModifier
    thread.join(2)
  end

  # Test LSP error codes are correct.
  def test_lsp_error_codes
    assert_equal(-32_002, CodingAdventures::Ls00::LspErrors::SERVER_NOT_INITIALIZED)
    assert_equal(-32_001, CodingAdventures::Ls00::LspErrors::UNKNOWN_ERROR_CODE)
    assert_equal(-32_803, CodingAdventures::Ls00::LspErrors::REQUEST_FAILED)
    assert_equal(-32_802, CodingAdventures::Ls00::LspErrors::SERVER_CANCELLED)
    assert_equal(-32_801, CodingAdventures::Ls00::LspErrors::CONTENT_MODIFIED)
    assert_equal(-32_800, CodingAdventures::Ls00::LspErrors::REQUEST_CANCELLED)
  end

  # Test document symbol conversion with nested children.
  def test_document_symbol_conversion
    bridge = MockBridge.new
    cache = CodingAdventures::Ls00::ParseCache.new

    result = cache.get_or_parse("file:///a.go", 1, "func main() {}", bridge)
    refute_nil result

    syms = bridge.document_symbols(result.ast)
    assert_equal 1, syms.length
    assert_equal "main", syms[0].name
    assert_equal CodingAdventures::Ls00::SymbolKind::FUNCTION, syms[0].kind
    assert_equal 1, syms[0].children.length
    assert_equal "x", syms[0].children[0].name
  end
end
