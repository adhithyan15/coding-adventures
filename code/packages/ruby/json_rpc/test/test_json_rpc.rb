# frozen_string_literal: true

require_relative "test_helper"
require "stringio"
require "json"

# ================================================================
# Tests for CodingAdventures::JsonRpc
# ================================================================
#
# Test plan:
#   1. ErrorCodes — correct integer constants
#   2. parseMessage — all valid shapes, error cases
#   3. message_to_h — serialisation round-trips
#   4. MessageReader — single message, back-to-back, EOF, malformed
#                      JSON, valid JSON that is not a message
#   5. MessageWriter — correct Content-Length, UTF-8, \r\n separator
#   6. Server — request dispatch, notification dispatch, unknown
#               method, handler returning ResponseError, handler
#               raising, multiple messages, chaining
#
# ================================================================

JR = CodingAdventures::JsonRpc

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# Build a Content-Length-framed string from a JSON string.
def frame(json)
  payload = json.encode("UTF-8").b
  "Content-Length: #{payload.bytesize}\r\n\r\n#{payload}"
end

# Build a StringIO containing zero or more framed messages, then closed.
def make_input(*jsons)
  combined = jsons.map { |j| frame(j) }.join
  StringIO.new(combined.b, "rb")
end

# Parse all framed JSON payloads from a StringIO output buffer.
# We force ASCII-8BIT for the outer string so byte indexing is safe,
# then force_encoding to UTF-8 on each JSON payload before parsing.
def parse_frames(output_io)
  raw = output_io.string.b # work in binary mode for byte-safe indexing
  results = []
  while raw.bytesize > 0
    m = raw.match(/Content-Length: (\d+)\r\n\r\n/)
    break unless m
    len     = m[1].to_i
    start   = m.end(0)
    payload = raw[start, len].force_encoding("UTF-8")
    results << JSON.parse(payload)
    raw = raw[(start + len)..]
  end
  results
end

# ===========================================================================
# 1. ErrorCodes
# ===========================================================================

class TestErrorCodes < Minitest::Test
  def test_parse_error_is_minus_32700
    assert_equal(-32_700, JR::ErrorCodes::PARSE_ERROR)
  end

  def test_invalid_request_is_minus_32600
    assert_equal(-32_600, JR::ErrorCodes::INVALID_REQUEST)
  end

  def test_method_not_found_is_minus_32601
    assert_equal(-32_601, JR::ErrorCodes::METHOD_NOT_FOUND)
  end

  def test_invalid_params_is_minus_32602
    assert_equal(-32_602, JR::ErrorCodes::INVALID_PARAMS)
  end

  def test_internal_error_is_minus_32603
    assert_equal(-32_603, JR::ErrorCodes::INTERNAL_ERROR)
  end
end

# ===========================================================================
# 2. parse_message
# ===========================================================================

class TestParseMessage < Minitest::Test
  # Spec test: Request with integer id
  def test_parses_request_with_integer_id
    hash = { "jsonrpc" => "2.0", "id" => 1, "method" => "ping" }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Request, msg
    assert_equal 1, msg.id
    assert_equal "ping", msg.method
    assert_nil msg.params
  end

  # Spec test: Request with string id
  def test_parses_request_with_string_id
    hash = { "jsonrpc" => "2.0", "id" => "abc", "method" => "ping" }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Request, msg
    assert_equal "abc", msg.id
  end

  # Spec test: Request with params
  def test_parses_request_with_params
    hash = { "jsonrpc" => "2.0", "id" => 2, "method" => "hover",
             "params" => { "line" => 0 } }
    msg = JR.parse_message(hash)
    assert_equal({ "line" => 0 }, msg.params)
  end

  # Spec test: Notification (no id)
  def test_parses_notification
    hash = { "jsonrpc" => "2.0", "method" => "textDocument/didOpen" }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Notification, msg
    assert_equal "textDocument/didOpen", msg.method
    assert_nil msg.params
  end

  # Spec test: success Response
  def test_parses_success_response
    hash = { "jsonrpc" => "2.0", "id" => 3, "result" => { "ok" => true } }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Response, msg
    assert_equal 3, msg.id
    assert_equal({ "ok" => true }, msg.result)
    assert_nil msg.error
  end

  # Spec test: error Response
  def test_parses_error_response
    hash = {
      "jsonrpc" => "2.0", "id" => 4,
      "error" => { "code" => -32_601, "message" => "Method not found" }
    }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Response, msg
    assert_instance_of JR::ResponseError, msg.error
    assert_equal(-32_601, msg.error.code)
    assert_equal "Method not found", msg.error.message
  end

  # Spec test: error Response with nil id
  def test_parses_error_response_with_nil_id
    hash = {
      "jsonrpc" => "2.0", "id" => nil,
      "error" => { "code" => -32_700, "message" => "Parse error" }
    }
    msg = JR.parse_message(hash)
    assert_instance_of JR::Response, msg
    assert_nil msg.id
  end

  # Spec test: raise on non-Hash
  def test_raises_on_non_hash
    assert_raises(JR::Error) { JR.parse_message("hello") }
    assert_raises(JR::Error) { JR.parse_message(42) }
    assert_raises(JR::Error) { JR.parse_message(nil) }
    assert_raises(JR::Error) { JR.parse_message([1, 2]) }
  end

  # Spec test: raise on unrecognised shape
  def test_raises_on_unrecognised_shape
    err = assert_raises(JR::Error) { JR.parse_message({ "jsonrpc" => "2.0" }) }
    assert_equal JR::ErrorCodes::INVALID_REQUEST, err.code
  end

  # Spec test: raise when id is not String or Integer for Request
  def test_raises_when_request_id_is_invalid
    assert_raises(JR::Error) do
      JR.parse_message({ "id" => [], "method" => "ping" })
    end
  end

  # Spec test: raise when error object is malformed
  def test_raises_when_error_object_is_not_hash
    assert_raises(JR::Error) do
      JR.parse_message({ "jsonrpc" => "2.0", "id" => 1, "error" => "not a hash" })
    end
  end
end

# ===========================================================================
# 3. message_to_h
# ===========================================================================

class TestMessageToH < Minitest::Test
  def test_serialises_request
    req = JR::Request.new(id: 1, method: "ping")
    h = JR.message_to_h(req)
    assert_equal "2.0", h["jsonrpc"]
    assert_equal 1, h["id"]
    assert_equal "ping", h["method"]
    refute h.key?("params")
  end

  def test_serialises_request_with_params
    req = JR::Request.new(id: 1, method: "ping", params: { x: 1 })
    h = JR.message_to_h(req)
    assert_equal({ x: 1 }, h["params"])
  end

  def test_serialises_notification
    notif = JR::Notification.new(method: "$/ping")
    h = JR.message_to_h(notif)
    assert_equal "2.0", h["jsonrpc"]
    assert_equal "$/ping", h["method"]
    refute h.key?("id")
  end

  def test_serialises_success_response
    resp = JR::Response.new(id: 2, result: 42)
    h = JR.message_to_h(resp)
    assert_equal 42, h["result"]
    refute h.key?("error")
  end

  def test_serialises_error_response
    err  = JR::ResponseError.new(code: -32_601, message: "Method not found")
    resp = JR::Response.new(id: 3, error: err)
    h = JR.message_to_h(resp)
    assert_equal(-32_601, h["error"]["code"])
    assert_equal "Method not found", h["error"]["message"]
  end

  def test_round_trip_request
    original = JR::Request.new(id: 7, method: "textDocument/hover", params: { "line" => 0 })
    json   = JSON.generate(JR.message_to_h(original))
    parsed = JR.parse_message(JSON.parse(json))
    assert_instance_of JR::Request, parsed
    assert_equal original.id, parsed.id
    assert_equal original.method, parsed.method
  end
end

# ===========================================================================
# 4. MessageReader
# ===========================================================================

class TestMessageReader < Minitest::Test
  def test_reads_single_request
    json = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "initialize" })
    reader = JR::MessageReader.new(make_input(json))
    msg = reader.read_message
    assert_instance_of JR::Request, msg
    assert_equal "initialize", msg.method
  end

  def test_reads_back_to_back_messages
    j1 = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "ping" })
    j2 = JSON.generate({ "jsonrpc" => "2.0", "method" => "notify" })
    reader = JR::MessageReader.new(make_input(j1, j2))
    m1 = reader.read_message
    m2 = reader.read_message
    assert_instance_of JR::Request, m1
    assert_instance_of JR::Notification, m2
  end

  def test_returns_nil_on_eof_with_no_data
    reader = JR::MessageReader.new(make_input)
    assert_nil reader.read_message
  end

  def test_returns_nil_after_last_message
    json = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "ping" })
    reader = JR::MessageReader.new(make_input(json))
    reader.read_message
    assert_nil reader.read_message
  end

  def test_raises_parse_error_on_malformed_json
    bad = "Content-Length: 5\r\n\r\n{bad}".b
    io  = StringIO.new(bad, "rb")
    reader = JR::MessageReader.new(io)
    err = assert_raises(JR::Error) { reader.read_message }
    assert_equal JR::ErrorCodes::PARSE_ERROR, err.code
  end

  def test_raises_invalid_request_on_valid_json_not_a_message
    json = JSON.generate([1, 2, 3]) # array — not a message
    reader = JR::MessageReader.new(make_input(json))
    err = assert_raises(JR::Error) { reader.read_message }
    assert_equal JR::ErrorCodes::INVALID_REQUEST, err.code
  end

  def test_reads_notification
    json = JSON.generate({ "jsonrpc" => "2.0", "method" => "textDocument/didOpen", "params" => {} })
    reader = JR::MessageReader.new(make_input(json))
    msg = reader.read_message
    assert_instance_of JR::Notification, msg
  end

  def test_reads_response
    json = JSON.generate({ "jsonrpc" => "2.0", "id" => 5, "result" => { "ok" => true } })
    reader = JR::MessageReader.new(make_input(json))
    msg = reader.read_message
    assert_instance_of JR::Response, msg
  end

  def test_read_raw_returns_json_string
    json = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "raw" })
    reader = JR::MessageReader.new(make_input(json))
    raw = reader.read_raw
    assert_equal json, raw.force_encoding("UTF-8")
  end

  def test_read_raw_returns_nil_on_eof
    reader = JR::MessageReader.new(make_input)
    assert_nil reader.read_raw
  end

  def test_raises_on_missing_content_length
    bad = "Content-Type: application/json\r\n\r\n{}".b
    io  = StringIO.new(bad, "rb")
    reader = JR::MessageReader.new(io)
    err = assert_raises(JR::Error) { reader.read_message }
    assert_equal JR::ErrorCodes::PARSE_ERROR, err.code
  end
end

# ===========================================================================
# 5. MessageWriter
# ===========================================================================

class TestMessageWriter < Minitest::Test
  def test_writes_correct_content_length
    out = StringIO.new("".b, "wb")
    writer = JR::MessageWriter.new(out)
    req = JR::Request.new(id: 1, method: "ping")
    writer.write_message(req)

    json = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "ping" })
    expected_len = json.encode("UTF-8").b.bytesize
    assert_includes out.string, "Content-Length: #{expected_len}"
  end

  def test_uses_crlf_crlf_separator
    out = StringIO.new("".b, "wb")
    writer = JR::MessageWriter.new(out)
    writer.write_message(JR::Notification.new(method: "ping"))
    assert_includes out.string, "\r\n\r\n"
  end

  def test_payload_is_valid_json
    out = StringIO.new("".b, "wb")
    writer = JR::MessageWriter.new(out)
    writer.write_message(JR::Response.new(id: 1, result: { "x" => 42 }))
    raw = out.string
    json_start = raw.index("\r\n\r\n") + 4
    payload = raw[json_start..]
    parsed = JSON.parse(payload)
    assert_equal 42, parsed["result"]["x"]
  end

  def test_content_length_accounts_for_multibyte_unicode
    out = StringIO.new("".b, "wb")
    writer = JR::MessageWriter.new(out)
    # "€" is 3 bytes in UTF-8
    writer.write_message(JR::Notification.new(method: "ping", params: { "s" => "€" }))
    raw = out.string
    m = raw.match(/Content-Length: (\d+)/)
    claimed_len = m[1].to_i
    json_start  = raw.index("\r\n\r\n") + 4
    actual_len  = raw[json_start..].b.bytesize
    assert_equal actual_len, claimed_len
  end

  def test_write_raw_frames_a_json_string
    out = StringIO.new("".b, "wb")
    writer = JR::MessageWriter.new(out)
    json = '{"jsonrpc":"2.0","id":9,"result":null}'
    writer.write_raw(json)
    raw = out.string
    assert_includes raw, json
    assert_includes raw, "Content-Length: #{json.b.bytesize}"
  end
end

# ===========================================================================
# 6. Server
# ===========================================================================

class TestServer < Minitest::Test
  def test_dispatches_request_and_writes_response
    req_json = JSON.generate({ "jsonrpc" => "2.0", "id" => 1, "method" => "ping" })
    input  = make_input(req_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output)
      .on_request("ping") { |_id, _params| "pong" }
      .serve

    frames = parse_frames(output)
    assert_equal 1, frames.length
    assert_equal 1, frames[0]["id"]
    assert_equal "pong", frames[0]["result"]
  end

  def test_dispatches_notification_without_writing_response
    notif_json = JSON.generate({ "jsonrpc" => "2.0", "method" => "notify", "params" => { "x" => 1 } })
    input  = make_input(notif_json)
    output = StringIO.new("".b, "wb")

    called_with = nil
    JR::Server.new(input, output)
      .on_notification("notify") { |params| called_with = params }
      .serve

    assert_equal({ "x" => 1 }, called_with)
    assert_empty output.string # no response written
  end

  def test_sends_method_not_found_for_unknown_request
    req_json = JSON.generate({ "jsonrpc" => "2.0", "id" => 2, "method" => "unknown/method" })
    input  = make_input(req_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output).serve

    frames = parse_frames(output)
    assert_equal 1, frames.length
    assert_equal JR::ErrorCodes::METHOD_NOT_FOUND, frames[0]["error"]["code"]
  end

  def test_sends_error_response_when_handler_returns_response_error
    req_json = JSON.generate({ "jsonrpc" => "2.0", "id" => 3, "method" => "fail" })
    input  = make_input(req_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output)
      .on_request("fail") { |_id, _params|
        JR::ResponseError.new(code: JR::ErrorCodes::INVALID_PARAMS, message: "Invalid params")
      }
      .serve

    frames = parse_frames(output)
    assert_equal JR::ErrorCodes::INVALID_PARAMS, frames[0]["error"]["code"]
  end

  def test_sends_internal_error_when_handler_raises
    req_json = JSON.generate({ "jsonrpc" => "2.0", "id" => 4, "method" => "boom" })
    input  = make_input(req_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output)
      .on_request("boom") { raise "Unexpected failure" }
      .serve

    frames = parse_frames(output)
    assert_equal JR::ErrorCodes::INTERNAL_ERROR, frames[0]["error"]["code"]
  end

  def test_ignores_unknown_notification_silently
    notif_json = JSON.generate({ "jsonrpc" => "2.0", "method" => "unknown/notif" })
    input  = make_input(notif_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output).serve

    assert_empty output.string
  end

  def test_handles_multiple_requests_in_sequence
    req1 = JSON.generate({ "jsonrpc" => "2.0", "id" => 10, "method" => "add", "params" => { "a" => 1, "b" => 2 } })
    req2 = JSON.generate({ "jsonrpc" => "2.0", "id" => 11, "method" => "add", "params" => { "a" => 3, "b" => 4 } })
    input  = make_input(req1, req2)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output)
      .on_request("add") { |_id, params| params["a"] + params["b"] }
      .serve

    frames = parse_frames(output)
    assert_equal 2, frames.length
    assert_equal 3, frames[0]["result"]
    assert_equal 7, frames[1]["result"]
  end

  def test_on_request_and_on_notification_are_chainable
    input  = make_input
    output = StringIO.new("".b, "wb")
    server = JR::Server.new(input, output)
    result = server
      .on_request("a") { nil }
      .on_notification("b") { nil }
    assert_equal server, result
  end

  def test_sends_error_response_on_malformed_json_framing
    bad = "Content-Length: 5\r\n\r\n{bad}".b
    input  = StringIO.new(bad, "rb")
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output).serve

    frames = parse_frames(output)
    assert frames.length >= 1
    assert_equal JR::ErrorCodes::PARSE_ERROR, frames[0]["error"]["code"]
  end

  def test_round_trip_request_params_echoed_back
    req_json = JSON.generate({
      "jsonrpc" => "2.0", "id" => 99, "method" => "echo",
      "params" => { "msg" => "hello" }
    })
    input  = make_input(req_json)
    output = StringIO.new("".b, "wb")

    JR::Server.new(input, output)
      .on_request("echo") { |_id, params| params }
      .serve

    frames = parse_frames(output)
    assert_equal 99, frames[0]["id"]
    assert_equal({ "msg" => "hello" }, frames[0]["result"])
  end
end
