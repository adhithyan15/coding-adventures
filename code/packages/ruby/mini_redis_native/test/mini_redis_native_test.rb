# frozen_string_literal: true

require_relative "test_helper"
require "stringio"

module MiniRedisTestHelpers
  def command(parts)
    output = +"".b
    output << "*#{parts.length}\r\n"
    parts.each do |part|
      bytes = part.b
      output << "$#{bytes.bytesize}\r\n"
      output << bytes
      output << "\r\n"
    end
    output
  end

  def receive(worker, stream_id, data)
    worker.receive_tcp_bytes(stream_id, data).writes.join
  end

  def job_request(job_id, stream_id, data, metadata = {})
    {
      "version" => CodingAdventures::MiniRedisNative::JOB_PROTOCOL_VERSION,
      "kind" => "request",
      "body" => {
        "id" => job_id,
        "payload" => {
          "stream_id" => stream_id,
          "bytes_hex" => data.unpack1("H*")
        },
        "metadata" => metadata
      }
    }
  end

  def with_server
    server = CodingAdventures::MiniRedisNative::Server.new(port: 0)
    thread = server.start
    socket = TCPSocket.new(server.host, server.port)
    socket.binmode
    yield server, socket
  ensure
    socket&.close
    server&.stop
    thread&.join(5)
    server&.close
  end
end

class TestRubyMiniRedisWorker < Minitest::Test
  include MiniRedisTestHelpers

  Worker = CodingAdventures::MiniRedisNative::MiniRedisWorker
  RespReply = CodingAdventures::MiniRedisNative::RespReply
  TcpOutputFrame = CodingAdventures::MiniRedisNative::TcpOutputFrame

  def test_version_exists
    assert_equal "0.1.0", CodingAdventures::MiniRedisNative::VERSION
  end

  def test_resp_reply_encodes_core_types
    assert_equal "+OK\r\n".b, RespReply.new(kind: "simple", value: "OK").encode
    assert_equal ":7\r\n".b, RespReply.new(kind: "integer", value: 7).encode
    assert_equal "$3\r\nabc\r\n".b, RespReply.new(kind: "bulk", value: "abc").encode
    assert_equal "$-1\r\n".b, RespReply.new(kind: "bulk").encode
    assert_equal "-ERR nope\r\n".b, RespReply.new(kind: "error", value: "ERR nope").encode
  end

  def test_tcp_output_frame_serializes_raw_write_frames
    frame = TcpOutputFrame.new(writes: ["+OK\r\n".b, ":1\r\n".b], close: true)
    assert_equal(
      { "writes_hex" => %w[2b4f4b0d0a 3a310d0a], "close" => true },
      frame.to_wire_payload
    )
  end

  def test_worker_buffers_fragmented_resp_frames
    worker = Worker.new
    data = command(["PING"])
    assert_equal "".b, receive(worker, "stream-1", data.byteslice(0, 3))
    assert_equal "+PONG\r\n".b, receive(worker, "stream-1", data.byteslice(3..))
  end

  def test_worker_processes_pipelined_resp_frames
    worker = Worker.new
    data = command(["PING"]) + command(["PING", "hello"])
    assert_equal "+PONG\r\n$5\r\nhello\r\n".b, receive(worker, "stream-1", data)
  end

  def test_worker_executes_string_and_hash_commands
    worker = Worker.new
    assert_equal "+PONG\r\n".b, receive(worker, "1", command(["PING"]))
    assert_equal "+OK\r\n".b, receive(worker, "1", command(["SET", "counter", "2"]))
    assert_equal "$1\r\n2\r\n".b, receive(worker, "1", command(["GET", "counter"]))
    assert_equal ":7\r\n".b, receive(worker, "1", command(["INCRBY", "counter", "5"]))
    assert_equal ":1\r\n".b, receive(worker, "1", command(["HSET", "user", "name", "ada"]))
    assert_equal "$3\r\nada\r\n".b, receive(worker, "1", command(["HGET", "user", "name"]))
    assert_equal ":1\r\n".b, receive(worker, "1", command(["HEXISTS", "user", "name"]))
  end

  def test_select_is_stream_local
    worker = Worker.new
    assert_equal "+OK\r\n".b, receive(worker, "1", command(["SET", "k", "db0"]))
    assert_equal "+OK\r\n".b, receive(worker, "2", command(["SELECT", "1"]))
    assert_equal "$-1\r\n".b, receive(worker, "2", command(["GET", "k"]))
    assert_equal "+OK\r\n".b, receive(worker, "2", command(["SET", "k", "db1"]))
    assert_equal "$3\r\ndb0\r\n".b, receive(worker, "1", command(["GET", "k"]))
    assert_equal "$3\r\ndb1\r\n".b, receive(worker, "2", command(["GET", "k"]))
  end

  def test_wire_request_round_trips_opaque_tcp_payload
    worker = Worker.new
    request = job_request("job-1", "stream-7", command(["PING"]), { "trace_id" => "t1" })
    response = JSON.parse(worker.handle_wire_request(JSON.generate(request)))
    assert_equal(
      {
        "writes_hex" => ["+PONG\r\n".unpack1("H*")],
        "close" => false
      },
      response.fetch("body").fetch("result").fetch("payload")
    )
  end

  def test_stdio_worker_processes_multiple_lines
    input = StringIO.new(
      [
        JSON.generate(job_request("a", "stream-1", command(["SET", "k", "v"]))),
        JSON.generate(job_request("b", "stream-1", command(["GET", "k"])))
      ].join("\n") + "\n"
    )
    output = StringIO.new

    CodingAdventures::MiniRedisNative.run_stdio_worker(input, output)
    lines = output.string.lines.map { |line| JSON.parse(line) }
    assert_equal ["+OK\r\n".unpack1("H*")], lines[0]["body"]["result"]["payload"]["writes_hex"]
    assert_equal ["$1\r\nv\r\n".unpack1("H*")], lines[1]["body"]["result"]["payload"]["writes_hex"]
  end
end

class TestNativeMiniRedisServer < Minitest::Test
  include MiniRedisTestHelpers

  def test_server_exposes_local_address
    server = CodingAdventures::MiniRedisNative::Server.new(port: 0)
    assert_equal "127.0.0.1", server.host
    assert_operator server.port, :>, 0
  ensure
    server&.close
  end

  def test_native_server_handles_mini_redis_round_trip
    with_server do |_server, socket|
      socket.write(command(["PING"]))
      assert_equal "+PONG\r\n".b, socket.read(7)

      socket.write(command(["SET", "name", "ruby"]))
      assert_equal "+OK\r\n".b, socket.read(5)

      socket.write(command(["GET", "name"]))
      assert_equal "$4\r\nruby\r\n".b, socket.read(10)
    end
  end

  def test_native_server_handles_fragmented_and_pipelined_commands
    with_server do |_server, socket|
      ping = command(["PING"])
      socket.write(ping.byteslice(0, 3))
      sleep 0.05
      socket.write(ping.byteslice(3..))
      assert_equal "+PONG\r\n".b, socket.read(7)

      socket.write(command(["PING"]) + command(["PING", "hello"]))
      assert_equal "+PONG\r\n$5\r\nhello\r\n".b, socket.read(18)
    end
  end
end
