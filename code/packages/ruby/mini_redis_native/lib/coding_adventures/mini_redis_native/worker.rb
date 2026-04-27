# frozen_string_literal: true

require "json"

module CodingAdventures
  module MiniRedisNative
    JOB_PROTOCOL_VERSION = 1
    DEFAULT_DATABASES = 16
    MAX_BUFFERED_STREAM_BYTES = 1024 * 1024

    class RespProtocolError < StandardError; end

    RespReply = Struct.new(:kind, :value, keyword_init: true) do
      def encode
        case kind
        when "simple"
          "+#{MiniRedisNative.bytes_value(value)}\r\n".b
        when "error"
          "-#{MiniRedisNative.bytes_value(value)}\r\n".b
        when "integer"
          ":#{Integer(value || 0)}\r\n".b
        when "bulk"
          return "$-1\r\n".b if value.nil?

          data = MiniRedisNative.bytes_value(value)
          "$#{data.bytesize}\r\n".b + data + "\r\n".b
        else
          raise ArgumentError, "unknown RESP reply kind: #{kind}"
        end
      end
    end

    TcpOutputFrame = Struct.new(:writes, :close, keyword_init: true) do
      def initialize(writes: [], close: false)
        super
      end

      def to_wire_payload
        {
          "writes_hex" => writes.map { |chunk| chunk.unpack1("H*") },
          "close" => close
        }
      end
    end

    RedisStreamSession = Struct.new(:selected_db, :buffer, keyword_init: true) do
      def initialize(selected_db: 0, buffer: +"".b)
        super
      end
    end

    class MiniRedisWorker
      attr_reader :pending_jobs

      def initialize(database_count: DEFAULT_DATABASES)
        @database_count = database_count
        @databases = Array.new(database_count) { {} }
        @sessions = {}
        @pending_jobs = []
      end

      def enqueue_tcp_job(stream_id, data)
        @pending_jobs << [stream_id.to_s, data.b]
      end

      def process_next_job
        job = @pending_jobs.shift
        return TcpOutputFrame.new if job.nil?

        receive_tcp_bytes(job[0], job[1])
      end

      def receive_tcp_bytes(stream_id, data)
        session = (@sessions[stream_id] ||= RedisStreamSession.new)
        if session.buffer.bytesize + data.bytesize > MAX_BUFFERED_STREAM_BYTES
          session.buffer.clear
          return TcpOutputFrame.new(
            writes: [error("ERR protocol error: request exceeds maximum buffered size").encode],
            close: true
          )
        end

        session.buffer << data
        writes = []

        until session.buffer.empty?
          parsed = parse_resp_command(session.buffer)
          break if parsed.nil?

          argv, consumed = parsed
          session.buffer.slice!(0, consumed)
          writes << execute_argv(session, argv).encode
        end

        TcpOutputFrame.new(writes: writes)
      rescue RespProtocolError => e
        session&.buffer&.clear
        TcpOutputFrame.new(writes: [error("ERR #{e.message}").encode])
      end

      def handle_wire_request(line)
        job_id, metadata, payload = MiniRedisNative.decode_job_request(line)
        stream_id, data = MiniRedisNative.decode_tcp_payload(payload)
        enqueue_tcp_job(stream_id, data)
        frame = process_next_job
        MiniRedisNative.encode_job_response(job_id, metadata, frame.to_wire_payload)
      end

      private

      def execute_argv(session, argv)
        return error("ERR empty command") if argv.empty?

        command = argv[0].encode("UTF-8", invalid: :replace, undef: :replace).upcase
        args = argv[1..] || []
        db = @databases[session.selected_db]
        next_db, reply = execute_command(session.selected_db, db, command, args)
        session.selected_db = next_db
        reply
      rescue StandardError => e
        error("ERR worker error: #{e.message}")
      end

      def execute_command(selected_db, db, command, args)
        case command
        when "PING" then [selected_db, ping(args)]
        when "SET" then [selected_db, set(db, args)]
        when "GET" then [selected_db, get(db, args)]
        when "EXISTS" then [selected_db, exists(db, args)]
        when "DEL" then [selected_db, delete(db, args)]
        when "INCRBY" then [selected_db, incrby(db, args)]
        when "HSET" then [selected_db, hset(db, args)]
        when "HGET" then [selected_db, hget(db, args)]
        when "HEXISTS" then [selected_db, hexists(db, args)]
        when "SELECT" then select_db(selected_db, args)
        else [selected_db, error("ERR unknown command '#{command}'")]
        end
      end

      def ping(args)
        return RespReply.new(kind: "simple", value: "PONG") if args.empty?
        return RespReply.new(kind: "bulk", value: args[0]) if args.length == 1

        wrong_arity("PING")
      end

      def set(db, args)
        return wrong_arity("SET") unless args.length == 2

        db[args[0]] = args[1]
        RespReply.new(kind: "simple", value: "OK")
      end

      def get(db, args)
        return wrong_arity("GET") unless args.length == 1

        value = db[args[0]]
        return RespReply.new(kind: "bulk") if value.nil?
        return wrong_type if value.is_a?(Hash)

        RespReply.new(kind: "bulk", value: value)
      end

      def exists(db, args)
        return wrong_arity("EXISTS") if args.empty?

        RespReply.new(kind: "integer", value: args.count { |key| db.key?(key) })
      end

      def delete(db, args)
        return wrong_arity("DEL") if args.empty?

        removed = args.count { |key| !db.delete(key).nil? }
        RespReply.new(kind: "integer", value: removed)
      end

      def incrby(db, args)
        return wrong_arity("INCRBY") unless args.length == 2

        key, delta_raw = args
        delta = Integer(delta_raw)
        current = db.fetch(key, "0".b)
        return wrong_type if current.is_a?(Hash)

        next_value = Integer(current) + delta
        db[key] = next_value.to_s.b
        RespReply.new(kind: "integer", value: next_value)
      rescue ArgumentError
        error("ERR value is not an integer or out of range")
      end

      def hset(db, args)
        return wrong_arity("HSET") if args.length < 3 || args.length.even?

        key = args[0]
        mapping = db[key]
        return wrong_type unless mapping.nil? || mapping.is_a?(Hash)

        mapping ||= {}
        added = 0
        index = 1
        while index < args.length
          field = args[index]
          value = args[index + 1]
          added += 1 unless mapping.key?(field)
          mapping[field] = value
          index += 2
        end
        db[key] = mapping
        RespReply.new(kind: "integer", value: added)
      end

      def hget(db, args)
        return wrong_arity("HGET") unless args.length == 2

        value = db[args[0]]
        return RespReply.new(kind: "bulk") if value.nil?
        return wrong_type unless value.is_a?(Hash)

        RespReply.new(kind: "bulk", value: value[args[1]])
      end

      def hexists(db, args)
        return wrong_arity("HEXISTS") unless args.length == 2

        value = db[args[0]]
        return RespReply.new(kind: "integer", value: 0) if value.nil?
        return wrong_type unless value.is_a?(Hash)

        RespReply.new(kind: "integer", value: value.key?(args[1]) ? 1 : 0)
      end

      def select_db(selected_db, args)
        return [selected_db, wrong_arity("SELECT")] unless args.length == 1

        index = Integer(args[0])
        return [selected_db, error("ERR invalid DB index")] if index.negative? || index >= @database_count

        [index, RespReply.new(kind: "simple", value: "OK")]
      rescue ArgumentError
        [selected_db, error("ERR invalid DB index")]
      end

      def parse_resp_command(buffer)
        raise RespProtocolError, "protocol error: expected array command frame" unless buffer.getbyte(0) == "*".ord

        header, position = read_line(buffer, 0)
        return nil if header.nil?

        count = Integer(header.byteslice(1..))
        raise RespProtocolError, "protocol error: null command arrays are not supported" if count.negative?

        parts = []
        count.times do
          return nil if position >= buffer.bytesize

          case buffer.getbyte(position)
          when "$".ord
            parsed = parse_bulk_string(buffer, position)
            return nil if parsed.nil?

            part, position = parsed
            parts << part
          when "+".ord, ":".ord
            line, next_position = read_line(buffer, position)
            return nil if line.nil?

            parts << line.byteslice(1..)
            position = next_position
          else
            raise RespProtocolError, "protocol error: expected bulk string command part"
          end
        end

        [parts, position]
      rescue ArgumentError
        raise RespProtocolError, "protocol error: invalid array length"
      end

      def parse_bulk_string(buffer, position)
        line, position = read_line(buffer, position)
        return nil if line.nil?

        length = Integer(line.byteslice(1..))
        raise RespProtocolError, "protocol error: null bulk command parts are not supported" if length.negative?

        data_end = position + length
        return nil if buffer.bytesize < data_end + 2
        raise RespProtocolError, "protocol error: malformed bulk string terminator" unless buffer.byteslice(data_end, 2) == "\r\n"

        [buffer.byteslice(position, length), data_end + 2]
      rescue ArgumentError
        raise RespProtocolError, "protocol error: invalid bulk string length"
      end

      def read_line(buffer, position)
        line_end = buffer.index("\r\n", position)
        return [nil, position] if line_end.nil?

        [buffer.byteslice(position, line_end - position), line_end + 2]
      end

      def error(message)
        MiniRedisNative.error(message)
      end

      def wrong_arity(command)
        error("ERR wrong number of arguments for '#{command}'")
      end

      def wrong_type
        error("WRONGTYPE Operation against a key holding the wrong kind of value")
      end
    end

    def self.run_stdio_worker(input = $stdin, output = $stdout)
      output.sync = true if output.respond_to?(:sync=)
      worker = MiniRedisWorker.new

      input.each_line do |line|
        line = line.strip
        next if line.empty?

        response =
          begin
            worker.handle_wire_request(line)
          rescue StandardError => e
            encode_job_error_response("unknown", {}, "worker_protocol_error", "worker protocol error: #{e.message}")
          end
        output.write(response)
        output.write("\n")
      end
    end

    def self.decode_job_request(line)
      frame = JSON.parse(line)
      raise ArgumentError, "unsupported job protocol version: #{frame["version"]}" unless frame["version"] == JOB_PROTOCOL_VERSION
      raise ArgumentError, "expected request frame, got #{frame["kind"].inspect}" unless frame["kind"] == "request"

      body = frame.fetch("body")
      metadata = body.fetch("metadata", {})
      payload = body.fetch("payload")
      raise ArgumentError, "job metadata must be an object" unless metadata.is_a?(Hash)
      raise ArgumentError, "job payload must be an object" unless payload.is_a?(Hash)

      [body.fetch("id").to_s, metadata, payload]
    end

    def self.decode_tcp_payload(payload)
      stream_id = payload.fetch("stream_id")
      bytes_hex = payload.fetch("bytes_hex")
      raise ArgumentError, "stream_id must be a string" unless stream_id.is_a?(String)
      raise ArgumentError, "bytes_hex must be a string" unless bytes_hex.is_a?(String)

      [stream_id, [bytes_hex].pack("H*")]
    end

    def self.encode_job_response(job_id, metadata, payload)
      JSON.generate(
        {
          "version" => JOB_PROTOCOL_VERSION,
          "kind" => "response",
          "body" => {
            "id" => job_id,
            "result" => {
              "status" => "ok",
              "payload" => payload
            },
            "metadata" => metadata
          }
        }
      )
    end

    def self.encode_job_error_response(job_id, metadata, code, message)
      JSON.generate(
        {
          "version" => JOB_PROTOCOL_VERSION,
          "kind" => "response",
          "body" => {
            "id" => job_id,
            "result" => {
              "status" => "error",
              "error" => {
                "code" => code,
                "message" => message,
                "retryable" => false,
                "origin" => "worker",
                "detail" => nil
              }
            },
            "metadata" => metadata
          }
        }
      )
    end

    def self.bytes_value(value)
      case value
      when nil then "".b
      when String then value.b
      when Integer then value.to_s.b
      else value.to_s.b
      end
    end

    def self.error(message)
      RespReply.new(kind: "error", value: message)
    end
  end
end
