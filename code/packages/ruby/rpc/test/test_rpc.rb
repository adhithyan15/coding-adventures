# frozen_string_literal: true

require_relative "test_helper"
require "stringio"

# ================================================================
# Tests for CodingAdventures::Rpc
# ================================================================
#
# Test strategy:
#   All tests use two in-process test doubles:
#
#   MockCodec  — serialises RpcMessage objects to/from Ruby's Marshal
#                format so tests can inspect frames without depending on a
#                real codec (JSON, etc.). Marshal keeps nils, colons, and
#                nested values intact across a round trip.
#
#   MockFramer — stores frames in an Array and pops them on read.
#                write_frame appends to an output Array.
#                read_frame returns nil when the input array is empty,
#                simulating clean EOF.
#
# This approach lets us test the Server and Client classes at the
# RPC layer without any I/O or codec complexity, exactly as the
# spec's "mock codec + framer" test strategy prescribes.
#
# Test plan:
#   1. ErrorCodes   — correct integer constants
#   2. RpcError     — carries code + message
#   3. Message types — Struct fields and keyword_init
#   4. Server       — request dispatch, notification dispatch, unknown
#                     method, handler returning RpcErrorResponse, handler
#                     raising, multiple messages, chaining, codec error
#   5. Client       — request returns result, server error, connection
#                     closed, notify sends frame, server-push notification
#                     during request, auto-incrementing ids
#   6. RpcCodec     — module raises NotImplementedError
#   7. RpcFramer    — module raises NotImplementedError
#
# ================================================================

R = CodingAdventures::Rpc

# ---------------------------------------------------------------------------
# MockCodec
# ---------------------------------------------------------------------------
#
# Encodes RpcMessage objects to a Marshal byte stream and decodes them
# back.  This keeps nils, colons, and nested values intact without needing
# any custom escaping logic.
#
# Format examples:
#   Marshal.dump(RpcRequest.new(id: 1, method: "ping", params: nil))
#   Marshal.dump(RpcResponse.new(id: 1, result: { "a" => 1 }))
#   Marshal.dump(RpcErrorResponse.new(id: nil, code: -32700,
#                                     message: "Parse error", data: nil))
#   Marshal.dump(RpcNotification.new(method: "log", params: { "msg" => "hi" }))
#
class MockCodec
  def encode(msg)
    Marshal.dump(msg).b
  end

  def decode(bytes)
    raise R::RpcError.new(R::ErrorCodes::PARSE_ERROR, "Undecodable bytes") if bytes == "BAD_BYTES".b
    raise R::RpcError.new(R::ErrorCodes::INVALID_REQUEST, "Not an RPC message") if bytes == "BAD_SHAPE".b

    msg = begin
      Marshal.load(bytes)
    rescue StandardError
      raise R::RpcError.new(R::ErrorCodes::PARSE_ERROR, "Undecodable bytes")
    end

    case msg
    when R::RpcRequest, R::RpcResponse, R::RpcErrorResponse, R::RpcNotification
      msg
    else
      raise R::RpcError.new(R::ErrorCodes::INVALID_REQUEST, "Unknown message type: #{msg.class}")
    end
  end
end

# ---------------------------------------------------------------------------
# MockFramer
# ---------------------------------------------------------------------------
#
# In-memory framer backed by two arrays: +@input_frames+ (read from) and
# +@output_frames+ (written to).  read_frame pops from @input_frames and
# returns nil when empty, simulating clean EOF.  write_frame appends to
# @output_frames.
#
class MockFramer
  attr_reader :output_frames

  def initialize(input_frames = [])
    @input_frames  = input_frames.dup
    @output_frames = []
  end

  def read_frame
    @input_frames.shift # returns nil when empty → EOF
  end

  def write_frame(bytes)
    @output_frames << bytes
    nil
  end
end

# ===========================================================================
# 1. ErrorCodes
# ===========================================================================

class TestErrorCodes < Minitest::Test
  def test_parse_error_is_minus_32700
    assert_equal(-32_700, R::ErrorCodes::PARSE_ERROR)
  end

  def test_invalid_request_is_minus_32600
    assert_equal(-32_600, R::ErrorCodes::INVALID_REQUEST)
  end

  def test_method_not_found_is_minus_32601
    assert_equal(-32_601, R::ErrorCodes::METHOD_NOT_FOUND)
  end

  def test_invalid_params_is_minus_32602
    assert_equal(-32_602, R::ErrorCodes::INVALID_PARAMS)
  end

  def test_internal_error_is_minus_32603
    assert_equal(-32_603, R::ErrorCodes::INTERNAL_ERROR)
  end
end

# ===========================================================================
# 2. RpcError
# ===========================================================================

class TestRpcError < Minitest::Test
  def test_carries_code_and_message
    err = R::RpcError.new(-32_601, "Method not found")
    assert_equal(-32_601, err.code)
    assert_equal "Method not found", err.message
  end

  def test_is_a_standard_error
    assert_kind_of StandardError, R::RpcError.new(-32_603, "oops")
  end
end

# ===========================================================================
# 3. Message types
# ===========================================================================

class TestMessageTypes < Minitest::Test
  def test_rpc_request_fields
    req = R::RpcRequest.new(id: 1, method: "ping", params: { "x" => 1 })
    assert_equal 1, req.id
    assert_equal "ping", req.method
    assert_equal({ "x" => 1 }, req.params)
  end

  def test_rpc_request_params_defaults_to_nil
    req = R::RpcRequest.new(id: 2, method: "ping")
    assert_nil req.params
  end

  def test_rpc_response_fields
    resp = R::RpcResponse.new(id: 1, result: "pong")
    assert_equal 1, resp.id
    assert_equal "pong", resp.result
  end

  def test_rpc_error_response_fields
    err = R::RpcErrorResponse.new(id: 1, code: -32_601, message: "not found", data: nil)
    assert_equal 1, err.id
    assert_equal(-32_601, err.code)
    assert_equal "not found", err.message
    assert_nil err.data
  end

  def test_rpc_notification_fields
    notif = R::RpcNotification.new(method: "log", params: { "msg" => "hi" })
    assert_equal "log", notif.method
    assert_equal({ "msg" => "hi" }, notif.params)
  end

  def test_rpc_notification_params_defaults_to_nil
    notif = R::RpcNotification.new(method: "ping")
    assert_nil notif.params
  end
end

# ===========================================================================
# 4. Server
# ===========================================================================

class TestServer < Minitest::Test
  CODEC = MockCodec.new

  # Build a MockFramer pre-loaded with encoded request frames.
  def make_server(*messages)
    frames = messages.map { |m| CODEC.encode(m) }
    framer = MockFramer.new(frames)
    server = R::Server.new(CODEC, framer)
    [server, framer]
  end

  # Decode all output frames back into RpcMessage objects for easy assertion.
  def decode_output(framer)
    framer.output_frames.map { |f| CODEC.decode(f) }
  end

  # ── 4.1 Dispatches request to handler and writes RpcResponse ──────────────

  def test_dispatches_request_and_writes_response
    req = R::RpcRequest.new(id: 1, method: "ping")
    server, framer = make_server(req)

    server.on_request("ping") { |_id, _params| "pong" }.serve

    responses = decode_output(framer)
    assert_equal 1, responses.length
    assert_instance_of R::RpcResponse, responses[0]
    assert_equal 1, responses[0].id
    assert_equal "pong", responses[0].result
  end

  # ── 4.2 Returns -32601 for unregistered method ────────────────────────────

  def test_sends_method_not_found_for_unknown_request
    req = R::RpcRequest.new(id: 2, method: "unknown_method")
    server, framer = make_server(req)

    server.serve

    responses = decode_output(framer)
    assert_equal 1, responses.length
    assert_instance_of R::RpcErrorResponse, responses[0]
    assert_equal 2, responses[0].id
    assert_equal R::ErrorCodes::METHOD_NOT_FOUND, responses[0].code
  end

  # ── 4.3 Handler returns RpcErrorResponse ─────────────────────────────────

  def test_handler_can_return_error_response
    req = R::RpcRequest.new(id: 3, method: "bad_params")
    server, framer = make_server(req)

    server.on_request("bad_params") { |_id, _params|
      R::RpcErrorResponse.new(id: 3, code: R::ErrorCodes::INVALID_PARAMS,
                              message: "Missing field", data: nil)
    }.serve

    responses = decode_output(framer)
    assert_instance_of R::RpcErrorResponse, responses[0]
    assert_equal R::ErrorCodes::INVALID_PARAMS, responses[0].code
    assert_equal "Missing field", responses[0].message
  end

  # ── 4.4 Dispatches notification without writing response ──────────────────

  def test_dispatches_notification_without_writing_response
    notif = R::RpcNotification.new(method: "log", params: { "msg" => "hello" })
    server, framer = make_server(notif)

    called_params = nil
    server.on_notification("log") { |params| called_params = params }.serve

    assert_equal({ "msg" => "hello" }, called_params)
    assert_empty framer.output_frames
  end

  # ── 4.5 Silently drops unknown notification ───────────────────────────────

  def test_silently_drops_unknown_notification
    notif = R::RpcNotification.new(method: "unknown/notif")
    server, framer = make_server(notif)

    server.serve

    assert_empty framer.output_frames
  end

  # ── 4.6 Recovers from panicking handler, sends -32603 ────────────────────

  def test_recovers_from_panicking_handler
    req = R::RpcRequest.new(id: 4, method: "explode")
    server, framer = make_server(req)

    server.on_request("explode") { raise RuntimeError, "Something went wrong" }.serve

    responses = decode_output(framer)
    assert_equal 1, responses.length
    assert_instance_of R::RpcErrorResponse, responses[0]
    assert_equal R::ErrorCodes::INTERNAL_ERROR, responses[0].code
    assert_equal 4, responses[0].id
  end

  # ── 4.7 Sends -32603 when notification handler raises (no response) ───────

  def test_notification_handler_raise_is_swallowed
    notif = R::RpcNotification.new(method: "boom")
    server, framer = make_server(notif)

    server.on_notification("boom") { raise "should be swallowed" }.serve

    # No response must be written — notifications are fire-and-forget.
    assert_empty framer.output_frames
  end

  # ── 4.8 Sends error response with nil id when codec fails to decode ───────

  def test_sends_error_with_nil_id_on_codec_failure
    # Inject a raw "BAD_BYTES" string — MockCodec will raise PARSE_ERROR
    framer = MockFramer.new(["BAD_BYTES".b])
    server = R::Server.new(CODEC, framer)

    server.serve

    responses = decode_output(framer)
    assert_equal 1, responses.length
    assert_instance_of R::RpcErrorResponse, responses[0]
    assert_nil responses[0].id
    assert_equal R::ErrorCodes::PARSE_ERROR, responses[0].code
  end

  # ── 4.9 Discards incoming RpcResponse messages (server-only mode) ─────────

  def test_discards_incoming_responses
    resp = R::RpcResponse.new(id: 99, result: "ignored")
    server, framer = make_server(resp)

    server.serve

    assert_empty framer.output_frames
  end

  # ── 4.10 Handles multiple requests in sequence ────────────────────────────

  def test_handles_multiple_requests_in_sequence
    req1 = R::RpcRequest.new(id: 10, method: "add", params: { "a" => 1, "b" => 2 })
    req2 = R::RpcRequest.new(id: 11, method: "add", params: { "a" => 3, "b" => 4 })
    server, framer = make_server(req1, req2)

    server.on_request("add") { |_id, params| params["a"] + params["b"] }.serve

    responses = decode_output(framer)
    assert_equal 2, responses.length
    assert_equal 3, responses[0].result
    assert_equal 7, responses[1].result
  end

  # ── 4.11 on_request and on_notification are chainable ────────────────────

  def test_chaining_returns_self
    framer = MockFramer.new([])
    server = R::Server.new(CODEC, framer)
    result = server
      .on_request("a") { nil }
      .on_notification("b") { nil }
    assert_equal server, result
  end

  # ── 4.12 Second registration replaces the earlier handler ────────────────

  def test_second_on_request_replaces_handler
    req = R::RpcRequest.new(id: 1, method: "greet")
    server, framer = make_server(req)

    server
      .on_request("greet") { |_id, _p| "hello" }
      .on_request("greet") { |_id, _p| "hi" }
      .serve

    responses = decode_output(framer)
    assert_equal "hi", responses[0].result
  end

  # ── 4.13 Discards incoming RpcErrorResponse messages ─────────────────────

  def test_discards_incoming_error_responses
    err_resp = R::RpcErrorResponse.new(id: 5, code: -32_601, message: "nope", data: nil)
    server, framer = make_server(err_resp)

    server.serve

    assert_empty framer.output_frames
  end
end

# ===========================================================================
# 5. Client
# ===========================================================================

class TestClient < Minitest::Test
  CODEC = MockCodec.new

  # Build a client whose framer will return the given response messages.
  def make_client(*response_messages)
    frames = response_messages.map { |m| CODEC.encode(m) }
    framer = MockFramer.new(frames)
    client = R::Client.new(CODEC, framer)
    [client, framer]
  end

  # Decode the single outgoing frame the client wrote.
  def decode_sent(framer, index = 0)
    CODEC.decode(framer.output_frames[index])
  end

  # ── 5.1 request() encodes and sends request, returns decoded result ───────

  def test_request_sends_request_and_returns_result
    resp = R::RpcResponse.new(id: 1, result: "pong")
    client, framer = make_client(resp)

    result = client.request("ping")

    assert_equal "pong", result
    assert_equal 1, framer.output_frames.length
    sent = decode_sent(framer)
    assert_instance_of R::RpcRequest, sent
    assert_equal "ping", sent.method
    assert_equal 1, sent.id
  end

  # ── 5.2 request() raises RpcError when server replies with error ──────────

  def test_request_raises_on_error_response
    err_resp = R::RpcErrorResponse.new(id: 1, code: R::ErrorCodes::METHOD_NOT_FOUND,
                                       message: "Not found", data: nil)
    client, _framer = make_client(err_resp)

    ex = assert_raises(R::RpcError) { client.request("missing") }
    assert_equal R::ErrorCodes::METHOD_NOT_FOUND, ex.code
    assert_equal "Not found", ex.message
  end

  # ── 5.3 request() raises RpcError when connection closes before response ──

  def test_request_raises_on_connection_closed
    client, _framer = make_client # no response frames → EOF immediately

    ex = assert_raises(R::RpcError) { client.request("ping") }
    assert_equal R::ErrorCodes::INTERNAL_ERROR, ex.code
    assert_match(/[Cc]onnection closed/, ex.message)
  end

  # ── 5.4 notify() sends notification without waiting ───────────────────────

  def test_notify_sends_frame_without_waiting
    client, framer = make_client # no responses needed

    result = client.notify("log", { "msg" => "hello" })

    assert_nil result
    assert_equal 1, framer.output_frames.length
    sent = decode_sent(framer)
    assert_instance_of R::RpcNotification, sent
    assert_equal "log", sent.method
    assert_equal({ "msg" => "hello" }, sent.params)
  end

  # ── 5.5 notify() with nil params sends nil params ─────────────────────────

  def test_notify_with_nil_params
    client, framer = make_client

    client.notify("heartbeat")

    sent = decode_sent(framer)
    assert_instance_of R::RpcNotification, sent
    assert_nil sent.params
  end

  # ── 5.6 on_notification() handler is called for server-push notifications ─

  def test_on_notification_handler_called_during_request
    notif = R::RpcNotification.new(method: "ping_push", params: { "count" => 1 })
    resp  = R::RpcResponse.new(id: 1, result: "done")
    client, _framer = make_client(notif, resp)

    received = nil
    client.on_notification("ping_push") { |params| received = params }

    result = client.request("go")

    assert_equal "done", result
    assert_equal({ "count" => 1 }, received)
  end

  # ── 5.7 Unregistered server-push notification is silently ignored ─────────

  def test_unknown_server_push_notification_is_ignored
    notif = R::RpcNotification.new(method: "unknown_push")
    resp  = R::RpcResponse.new(id: 1, result: 42)
    client, _framer = make_client(notif, resp)

    # No handler registered — should not raise, just wait for the response.
    result = client.request("query")
    assert_equal 42, result
  end

  # ── 5.8 Request ids are auto-generated and monotonically increasing ───────

  def test_request_ids_are_auto_incremented
    resp1 = R::RpcResponse.new(id: 1, result: "a")
    resp2 = R::RpcResponse.new(id: 2, result: "b")
    client, framer = make_client(resp1, resp2)

    client.request("first")
    client.request("second")

    sent1 = decode_sent(framer, 0)
    sent2 = decode_sent(framer, 1)
    assert_equal 1, sent1.id
    assert_equal 2, sent2.id
  end

  # ── 5.9 on_notification is chainable ─────────────────────────────────────

  def test_on_notification_is_chainable
    client, _framer = make_client
    result = client.on_notification("x") { nil }
    assert_equal client, result
  end

  # ── 5.10 Response for a different id is skipped; client waits for match ───

  def test_skips_response_with_wrong_id
    wrong_resp  = R::RpcResponse.new(id: 99, result: "wrong")
    right_resp  = R::RpcResponse.new(id: 1,  result: "right")
    client, _framer = make_client(wrong_resp, right_resp)

    result = client.request("something")
    assert_equal "right", result
  end

  # ── 5.11 Error response for a different id is skipped ────────────────────

  def test_skips_error_response_with_wrong_id
    wrong_err   = R::RpcErrorResponse.new(id: 99, code: -32_601, message: "nope", data: nil)
    right_resp  = R::RpcResponse.new(id: 1, result: "ok")
    client, _framer = make_client(wrong_err, right_resp)

    result = client.request("something")
    assert_equal "ok", result
  end

  # ── 5.12 handler exception during server-push is swallowed ───────────────

  def test_server_push_handler_exception_is_swallowed
    notif = R::RpcNotification.new(method: "boom_push")
    resp  = R::RpcResponse.new(id: 1, result: "safe")
    client, _framer = make_client(notif, resp)

    client.on_notification("boom_push") { raise "should be swallowed" }

    result = client.request("ping")
    assert_equal "safe", result
  end
end

# ===========================================================================
# 6. RpcCodec module
# ===========================================================================

class TestRpcCodecModule < Minitest::Test
  # A concrete class that includes RpcCodec does NOT override the methods.
  class BrokenCodec
    include R::RpcCodec
  end

  def test_encode_raises_not_implemented
    assert_raises(NotImplementedError) { BrokenCodec.new.encode(nil) }
  end

  def test_decode_raises_not_implemented
    assert_raises(NotImplementedError) { BrokenCodec.new.decode("") }
  end
end

# ===========================================================================
# 7. RpcFramer module
# ===========================================================================

class TestRpcFramerModule < Minitest::Test
  class BrokenFramer
    include R::RpcFramer
  end

  def test_read_frame_raises_not_implemented
    assert_raises(NotImplementedError) { BrokenFramer.new.read_frame }
  end

  def test_write_frame_raises_not_implemented
    assert_raises(NotImplementedError) { BrokenFramer.new.write_frame("") }
  end
end

# ===========================================================================
# 8. MockCodec round-trip (validates test infrastructure)
# ===========================================================================

class TestMockCodecRoundTrip < Minitest::Test
  CODEC = MockCodec.new

  def round_trip(msg)
    CODEC.decode(CODEC.encode(msg))
  end

  def test_round_trips_rpc_request
    original = R::RpcRequest.new(id: 7, method: "hover", params: { "line" => 10 })
    result   = round_trip(original)
    assert_equal original.id,     result.id
    assert_equal original.method, result.method
    assert_equal original.params, result.params
  end

  def test_round_trips_rpc_request_with_string_id
    original = R::RpcRequest.new(id: "abc", method: "ping")
    result   = round_trip(original)
    assert_equal "abc", result.id
  end

  def test_round_trips_rpc_response
    original = R::RpcResponse.new(id: 1, result: [1, 2, 3])
    result   = round_trip(original)
    assert_equal original.id,     result.id
    assert_equal original.result, result.result
  end

  def test_round_trips_rpc_error_response
    original = R::RpcErrorResponse.new(id: nil, code: -32_700,
                                       message: "Parse error", data: nil)
    result   = round_trip(original)
    assert_nil result.id
    assert_equal(-32_700, result.code)
    assert_equal "Parse error", result.message
  end

  def test_round_trips_rpc_notification
    original = R::RpcNotification.new(method: "$/ping", params: nil)
    result   = round_trip(original)
    assert_equal "$/ping", result.method
    assert_nil result.params
  end

  def test_decode_raises_parse_error_for_bad_bytes
    ex = assert_raises(R::RpcError) { CODEC.decode("BAD_BYTES".b) }
    assert_equal R::ErrorCodes::PARSE_ERROR, ex.code
  end

  def test_decode_raises_invalid_request_for_bad_shape
    ex = assert_raises(R::RpcError) { CODEC.decode("BAD_SHAPE".b) }
    assert_equal R::ErrorCodes::INVALID_REQUEST, ex.code
  end
end
