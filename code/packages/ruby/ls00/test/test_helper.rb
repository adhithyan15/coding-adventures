# frozen_string_literal: true

require "minitest/autorun"
require "stringio"
require "json"
require "coding_adventures_json_rpc"
require "coding_adventures_ls00"

# ================================================================
# Test Bridges — MockBridge and MinimalBridge
# ================================================================
#
# MockBridge implements the required LanguageBridge methods PLUS hover
# and document_symbols. It provides deterministic, simple behaviors
# for testing the framework without a real language implementation.
#
# MinimalBridge implements ONLY the required methods (tokenize, parse).
# Used to test that optional capabilities are NOT advertised.
#
# FullMockBridge extends MockBridge with ALL optional provider methods.
# Used to test full capability advertisement and all handler paths.
#
# ================================================================

# MockBridge implements required methods plus hover and document_symbols.
class MockBridge
  attr_accessor :hover_result

  def initialize
    @hover_result = nil
  end

  # tokenize splits source by whitespace and returns one token per word.
  def tokenize(source)
    col = 1
    source.split.map do |word|
      tok = CodingAdventures::Ls00::Token.new(
        type: "WORD", value: word, line: 1, column: col
      )
      col += word.length + 1
      tok
    end
  end

  # parse returns a minimal AST. If source contains "ERROR", it returns
  # a diagnostic.
  def parse(source)
    diags = []
    if source.include?("ERROR")
      diags << CodingAdventures::Ls00::Diagnostic.new(
        range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 0),
          end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 5)
        ),
        severity: CodingAdventures::Ls00::DiagnosticSeverity::ERROR,
        message: "syntax error: unexpected ERROR token",
        code: nil
      )
    end
    [source, diags]
  end

  # hover returns the configured hover_result.
  def hover(_ast, _pos)
    @hover_result
  end

  # document_symbols returns a fixed two-symbol tree.
  def document_symbols(_ast)
    [
      CodingAdventures::Ls00::DocumentSymbol.new(
        name: "main",
        kind: CodingAdventures::Ls00::SymbolKind::FUNCTION,
        range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 0),
          end_pos: CodingAdventures::Ls00::Position.new(line: 10, character: 1)
        ),
        selection_range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 9),
          end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 13)
        ),
        children: [
          CodingAdventures::Ls00::DocumentSymbol.new(
            name: "x",
            kind: CodingAdventures::Ls00::SymbolKind::VARIABLE,
            range: CodingAdventures::Ls00::LspRange.new(
              start: CodingAdventures::Ls00::Position.new(line: 1, character: 4),
              end_pos: CodingAdventures::Ls00::Position.new(line: 1, character: 12)
            ),
            selection_range: CodingAdventures::Ls00::LspRange.new(
              start: CodingAdventures::Ls00::Position.new(line: 1, character: 8),
              end_pos: CodingAdventures::Ls00::Position.new(line: 1, character: 9)
            ),
            children: []
          )
        ]
      )
    ]
  end
end

# MinimalBridge implements ONLY the required LanguageBridge interface.
class MinimalBridge
  def tokenize(_source)
    []
  end

  def parse(source)
    [source, []]
  end
end

# FullMockBridge extends MockBridge with all optional interfaces.
class FullMockBridge < MockBridge
  def semantic_tokens(_source, tokens)
    tokens.map do |tok|
      CodingAdventures::Ls00::SemanticToken.new(
        line: tok.line - 1,
        character: tok.column - 1,
        length: tok.value.length,
        token_type: "variable",
        modifiers: nil
      )
    end
  end

  def definition(_ast, pos, uri)
    CodingAdventures::Ls00::Location.new(
      uri: uri,
      range: CodingAdventures::Ls00::LspRange.new(start: pos, end_pos: pos)
    )
  end

  def references(_ast, pos, uri, _include_decl)
    [CodingAdventures::Ls00::Location.new(
      uri: uri,
      range: CodingAdventures::Ls00::LspRange.new(start: pos, end_pos: pos)
    )]
  end

  def completion(_ast, _pos)
    [CodingAdventures::Ls00::CompletionItem.new(
      label: "foo",
      kind: CodingAdventures::Ls00::CompletionItemKind::FUNCTION,
      detail: "() void"
    )]
  end

  def rename(_ast, pos, new_name)
    CodingAdventures::Ls00::WorkspaceEdit.new(
      changes: {
        "file:///test.txt" => [
          CodingAdventures::Ls00::TextEdit.new(
            range: CodingAdventures::Ls00::LspRange.new(start: pos, end_pos: pos),
            new_text: new_name
          )
        ]
      }
    )
  end

  def folding_ranges(_ast)
    [CodingAdventures::Ls00::FoldingRange.new(start_line: 0, end_line: 5, kind: "region")]
  end

  def signature_help(_ast, _pos)
    CodingAdventures::Ls00::SignatureHelpResult.new(
      signatures: [
        CodingAdventures::Ls00::SignatureInformation.new(
          label: "foo(a int, b string)",
          documentation: nil,
          parameters: [
            CodingAdventures::Ls00::ParameterInformation.new(label: "a int", documentation: nil),
            CodingAdventures::Ls00::ParameterInformation.new(label: "b string", documentation: nil)
          ]
        )
      ],
      active_signature: 0,
      active_parameter: 0
    )
  end

  def format(source)
    [CodingAdventures::Ls00::TextEdit.new(
      range: CodingAdventures::Ls00::LspRange.new(
        start: CodingAdventures::Ls00::Position.new(line: 0, character: 0),
        end_pos: CodingAdventures::Ls00::Position.new(line: 999, character: 0)
      ),
      new_text: source # no-op formatter: returns source unchanged
    )]
  end
end

# ================================================================
# JSON-RPC test helpers
# ================================================================

# make_message creates a Content-Length-framed JSON-RPC message string.
def make_message(obj)
  json = JSON.generate(obj)
  "Content-Length: #{json.bytesize}\r\n\r\n#{json}"
end

# read_message reads one Content-Length-framed message from a StringIO.
def read_message(io)
  reader = CodingAdventures::JsonRpc::MessageReader.new(io)
  reader.read_message
end

# pipe_server creates an LspServer with pipe-based IO for testing.
# Returns [writer_to_server, reader_from_server, server_thread].
def pipe_server(bridge)
  in_r, in_w = IO.pipe
  out_r, out_w = IO.pipe
  in_r.binmode
  in_w.binmode
  out_r.binmode
  out_w.binmode

  server = CodingAdventures::Ls00::LspServer.new(bridge, in_r, out_w)
  thread = Thread.new do
    server.serve
  rescue StandardError
    # swallow errors on shutdown
  ensure
    out_w.close rescue nil # rubocop:disable Style/RescueModifier
  end

  client_writer = CodingAdventures::JsonRpc::MessageWriter.new(in_w)
  client_reader = CodingAdventures::JsonRpc::MessageReader.new(out_r)

  [client_writer, client_reader, thread, in_w]
end

# send_request sends a JSON-RPC request and returns the response result.
def send_rpc_request(writer, reader, id, method, params)
  req = CodingAdventures::JsonRpc::Request.new(id: id, method: method, params: params)
  writer.write_message(req)

  msg = reader.read_message
  raise "expected Response, got #{msg.class}" unless msg.is_a?(CodingAdventures::JsonRpc::Response)
  raise "expected id #{id}, got #{msg.id}" unless msg.id == id

  msg
end

# send_notification sends a JSON-RPC notification (no response expected).
def send_rpc_notification(writer, method, params)
  notif = CodingAdventures::JsonRpc::Notification.new(method: method, params: params)
  writer.write_message(notif)
end

# read_notification reads the next message, expecting a notification.
def read_rpc_notification(reader)
  msg = reader.read_message
  raise "expected Notification, got #{msg.class}" unless msg.is_a?(CodingAdventures::JsonRpc::Notification)

  msg
end
