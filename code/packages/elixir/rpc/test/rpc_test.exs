defmodule RpcTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for the Rpc package.

  ## Test Organization

  1. Errors — code constants and error map constructors
  2. Message — struct creation
  3. Server — dispatch, error responses, exception recovery, notifications
  4. Client — request/response correlation, error handling, notify
  5. Rpc top-level delegates
  6. MockCodec encode/decode round-trips
  7. MockFramer read_frame/write_frame

  ## Testing Strategy

  Since `Rpc` is codec-agnostic, we use in-memory mock implementations of
  `Rpc.Codec` and `Rpc.Framer` rather than any real wire format. This keeps
  the tests fast, deterministic, and free of I/O dependencies.

  ### MockCodec

  `MockCodec` uses Erlang's `:erlang.term_to_binary` / `:erlang.binary_to_term`
  for serialization. This is not a production codec — it is only here to prove
  that the server and client work correctly with *any* implementation of
  `Rpc.Codec`.

  ### SpyFramer

  `SpyFramer` uses an Erlang queue for incoming frames (pre-populated by the
  test) and an `Agent` to capture outgoing frames. After `serve/4` returns,
  the Agent's list is inspected to verify what responses were written.

  ### In-Memory Pipes

  For server tests: pre-populate `SpyFramer`'s queue, run `Server.serve/4` in
  a `Task`, then inspect the Agent's written frames.

  For client tests: pre-populate a simple `MockFramer`'s queue with encoded
  responses, call `Client.request/3`, verify the result.
  """

  alias Rpc.{Client, Errors, Server}
  alias Rpc.Message.{ErrorResponse, Notification, Request, Response}

  # ===========================================================================
  # MockCodec — Erlang term_to_binary / binary_to_term codec
  # ===========================================================================
  #
  # Serializes Rpc.Message structs using Erlang's built-in term serialization.
  # This is intentionally a "cheat" codec for tests — it proves that Server
  # and Client are codec-agnostic, not that any particular encoding is correct.

  defmodule MockCodec do
    @behaviour Rpc.Codec

    @impl Rpc.Codec
    def encode(msg) do
      {:ok, :erlang.term_to_binary(msg)}
    end

    @impl Rpc.Codec
    def decode(data) do
      msg = :erlang.binary_to_term(data)

      case msg do
        %Request{} -> {:ok, msg}
        %Response{} -> {:ok, msg}
        %ErrorResponse{} -> {:ok, msg}
        %Notification{} -> {:ok, msg}
        _ -> {:error, Rpc.Errors.make_invalid_request("not a recognized message type")}
      end
    rescue
      _ ->
        {:error,
         %ErrorResponse{
           id: nil,
           code: Errors.parse_error(),
           message: "Parse error",
           data: "binary_to_term failed"
         }}
    end
  end

  # ===========================================================================
  # MockFramer — queue-based in-memory framer (no Agent)
  # ===========================================================================
  #
  # State: %{in_queue: :queue.t(), out_list: [binary()]}
  # Used for Client tests where we need both read and write access.

  defmodule MockFramer do
    @behaviour Rpc.Framer

    # Build an initial state with pre-loaded incoming frames.
    def new(frames \\ []) do
      q =
        Enum.reduce(frames, :queue.new(), fn frame, q ->
          :queue.in(frame, q)
        end)

      %{in_queue: q, out_list: []}
    end

    # Retrieve all written frames in order (FIFO — first written is first).
    def written_frames(%{out_list: list}), do: Enum.reverse(list)

    @impl Rpc.Framer
    def read_frame(%{in_queue: q} = state) do
      case :queue.out(q) do
        {:empty, _} -> :eof
        {{:value, frame}, new_q} -> {:ok, frame, %{state | in_queue: new_q}}
      end
    end

    @impl Rpc.Framer
    def write_frame(data, %{out_list: list} = state) do
      {:ok, %{state | out_list: [data | list]}}
    end
  end

  # ===========================================================================
  # SpyFramer — Agent-backed framer for server tests
  # ===========================================================================
  #
  # Server tests need to inspect frames written by the server *after* serve/4
  # has already returned. Since serve/4 threads framer_state through its stack
  # frames and returns only :ok, we use an Agent to collect written frames out-
  # of-band.
  #
  # State: %{in_queue: :queue.t(), agent: pid()}

  defmodule SpyFramer do
    @behaviour Rpc.Framer

    def new(frames, agent_pid) do
      q =
        Enum.reduce(frames, :queue.new(), fn frame, q ->
          :queue.in(frame, q)
        end)

      %{in_queue: q, agent: agent_pid}
    end

    @impl Rpc.Framer
    def read_frame(%{in_queue: q} = state) do
      case :queue.out(q) do
        {:empty, _} -> :eof
        {{:value, frame}, new_q} -> {:ok, frame, %{state | in_queue: new_q}}
      end
    end

    @impl Rpc.Framer
    def write_frame(data, %{agent: agent} = state) do
      Agent.update(agent, fn list -> [data | list] end)
      {:ok, state}
    end
  end

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------

  # Encode a message with MockCodec into raw bytes for the framer.
  defp encode_msg(msg) do
    {:ok, bytes} = MockCodec.encode(msg)
    bytes
  end

  # Run the server against pre-built input messages. Returns decoded responses.
  # Uses SpyFramer so we can observe what the server wrote.
  defp serve_and_capture(input_messages, register_fn) do
    frames = Enum.map(input_messages, &encode_msg/1)
    {:ok, agent} = Agent.start_link(fn -> [] end)
    framer_state = SpyFramer.new(frames, agent)
    handlers = register_fn.(%{})

    task = Task.async(fn -> Server.serve(MockCodec, SpyFramer, framer_state, handlers) end)
    :ok = Task.await(task, 5_000)

    raw_frames = Agent.get(agent, fn list -> Enum.reverse(list) end)
    Agent.stop(agent)

    Enum.map(raw_frames, fn bytes ->
      {:ok, msg} = MockCodec.decode(bytes)
      msg
    end)
  end

  # ===========================================================================
  # 1. Errors — constants and constructors
  # ===========================================================================

  describe "Errors — code constants" do
    test "1. parse_error is -32700" do
      assert Errors.parse_error() == -32_700
    end

    test "2. invalid_request is -32600" do
      assert Errors.invalid_request() == -32_600
    end

    test "3. method_not_found is -32601" do
      assert Errors.method_not_found() == -32_601
    end

    test "4. invalid_params is -32602" do
      assert Errors.invalid_params() == -32_602
    end

    test "5. internal_error is -32603" do
      assert Errors.internal_error() == -32_603
    end
  end

  describe "Errors — constructors" do
    test "6. make_parse_error without data omits :data key" do
      err = Errors.make_parse_error()
      assert err.code == -32_700
      assert err.message == "Parse error"
      refute Map.has_key?(err, :data)
    end

    test "7. make_parse_error with data includes :data key" do
      err = Errors.make_parse_error("bad bytes")
      assert err.code == -32_700
      assert err.data == "bad bytes"
    end

    test "8. make_invalid_request" do
      err = Errors.make_invalid_request("missing method")
      assert err.code == -32_600
      assert err.data == "missing method"
    end

    test "9. make_method_not_found" do
      err = Errors.make_method_not_found("tools/call")
      assert err.code == -32_601
      assert err.data == "tools/call"
    end

    test "10. make_invalid_params" do
      err = Errors.make_invalid_params("expected map")
      assert err.code == -32_602
      assert err.data == "expected map"
    end

    test "11. make_internal_error with data" do
      err = Errors.make_internal_error("handler crashed")
      assert err.code == -32_603
      assert err.data == "handler crashed"
    end

    test "12. make_internal_error without data omits :data key" do
      err = Errors.make_internal_error()
      assert err.code == -32_603
      refute Map.has_key?(err, :data)
    end
  end

  # ===========================================================================
  # 2. Message — struct creation
  # ===========================================================================

  describe "Rpc.Message structs" do
    test "13. Request struct has id, method, params" do
      req = %Request{id: 1, method: "ping", params: nil}
      assert req.id == 1
      assert req.method == "ping"
      assert req.params == nil
    end

    test "14. Response struct has id and result" do
      resp = %Response{id: 2, result: "pong"}
      assert resp.id == 2
      assert resp.result == "pong"
    end

    test "15. ErrorResponse struct has id, code, message, data" do
      err = %ErrorResponse{id: 3, code: -32_601, message: "Method not found", data: "foo"}
      assert err.id == 3
      assert err.code == -32_601
      assert err.data == "foo"
    end

    test "16. Notification struct has method and params" do
      notif = %Notification{method: "log", params: %{"msg" => "hello"}}
      assert notif.method == "log"
      assert notif.params == %{"msg" => "hello"}
    end

    test "17. Request params defaults to nil" do
      req = %Request{id: 1, method: "ping"}
      assert req.params == nil
    end

    test "18. Notification params defaults to nil" do
      notif = %Notification{method: "notify"}
      assert notif.params == nil
    end
  end

  # ===========================================================================
  # 3. Server — dispatch loop
  # ===========================================================================

  describe "Server — request dispatch" do
    test "19. dispatches request to handler and writes success response" do
      input = [%Request{id: 1, method: "ping"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "ping", fn _id, _params -> "pong" end)
        end)

      assert [%Response{id: 1, result: "pong"}] = responses
    end

    test "20. sends -32601 method not found for unregistered method" do
      input = [%Request{id: 2, method: "unknown"}]

      responses = serve_and_capture(input, fn handlers -> handlers end)

      assert [%ErrorResponse{id: 2, code: -32_601}] = responses
    end

    test "21. handler returning {:error, ErrorResponse} sends error response" do
      input = [%Request{id: 3, method: "fail"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "fail", fn id, _params ->
            {:error,
             %ErrorResponse{id: id, code: Errors.invalid_params(), message: "bad params"}}
          end)
        end)

      assert [%ErrorResponse{id: 3, code: -32_602, message: "bad params"}] = responses
    end

    test "22. handler exception results in internal error response (-32603)" do
      input = [%Request{id: 4, method: "boom"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "boom", fn _id, _params ->
            raise RuntimeError, "kaboom"
          end)
        end)

      assert [%ErrorResponse{id: 4, code: -32_603}] = responses
    end

    test "23. handler exception data contains the exception message" do
      input = [%Request{id: 5, method: "crash"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "crash", fn _id, _params ->
            raise ArgumentError, "bad argument: nil"
          end)
        end)

      assert [%ErrorResponse{id: 5, code: -32_603, data: data}] = responses
      assert data =~ "bad argument: nil"
    end

    test "24. server survives after handler exception and processes next request" do
      input = [
        %Request{id: 1, method: "boom"},
        %Request{id: 2, method: "ping"}
      ]

      responses =
        serve_and_capture(input, fn handlers ->
          handlers
          |> Server.register_request("boom", fn _id, _params -> raise "crash" end)
          |> Server.register_request("ping", fn _id, _params -> "pong" end)
        end)

      assert length(responses) == 2
      assert %ErrorResponse{id: 1, code: -32_603} = Enum.at(responses, 0)
      assert %Response{id: 2, result: "pong"} = Enum.at(responses, 1)
    end

    test "25. multiple requests processed in order" do
      input = [
        %Request{id: 1, method: "add", params: [1, 2]},
        %Request{id: 2, method: "add", params: [3, 4]}
      ]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "add", fn _id, [a, b] -> a + b end)
        end)

      assert [%Response{id: 1, result: 3}, %Response{id: 2, result: 7}] = responses
    end

    test "26. request with params passes params to handler" do
      input = [%Request{id: 1, method: "echo", params: "hello world"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_request(handlers, "echo", fn _id, params -> params end)
        end)

      assert [%Response{id: 1, result: "hello world"}] = responses
    end
  end

  describe "Server — notification dispatch" do
    test "27. dispatches notification to handler without writing a response" do
      parent = self()
      input = [%Notification{method: "log", params: "hello"}]

      responses =
        serve_and_capture(input, fn handlers ->
          Server.register_notification(handlers, "log", fn params ->
            send(parent, {:notified, params})
          end)
        end)

      # No response written for notifications.
      assert responses == []
      assert_received {:notified, "hello"}
    end

    test "28. unknown notification is silently ignored (no response)" do
      input = [%Notification{method: "unknown"}]
      responses = serve_and_capture(input, fn handlers -> handlers end)
      assert responses == []
    end

    test "29. notification handler exception does not crash server" do
      parent = self()

      input = [
        %Notification{method: "crash"},
        %Notification{method: "ok"}
      ]

      responses =
        serve_and_capture(input, fn handlers ->
          handlers
          |> Server.register_notification("crash", fn _params ->
            raise "notification crash"
          end)
          |> Server.register_notification("ok", fn _params -> send(parent, :ok_notified) end)
        end)

      assert responses == []
      assert_received :ok_notified
    end

    test "30. request followed by notification writes only one response" do
      parent = self()

      input = [
        %Request{id: 1, method: "ping"},
        %Notification{method: "log"}
      ]

      responses =
        serve_and_capture(input, fn handlers ->
          handlers
          |> Server.register_request("ping", fn _id, _params -> "pong" end)
          |> Server.register_notification("log", fn _params -> send(parent, :logged) end)
        end)

      assert [%Response{id: 1, result: "pong"}] = responses
      assert_received :logged
    end
  end

  describe "Server — EOF and codec errors" do
    test "31. serve returns :ok on empty input (immediate EOF)" do
      framer_state = MockFramer.new([])
      :ok = Server.serve(MockCodec, MockFramer, framer_state, %{})
    end

    test "32. codec decode error sends error response with nil id" do
      # Inject raw bytes that MockCodec cannot decode (garbage binary).
      bad_bytes = <<"not valid erlang term binary">>

      {:ok, agent} = Agent.start_link(fn -> [] end)
      # Build a SpyFramer with the bad frame pre-loaded.
      bad_spy_state = SpyFramer.new([bad_bytes], agent)

      task = Task.async(fn -> Server.serve(MockCodec, SpyFramer, bad_spy_state, %{}) end)
      :ok = Task.await(task, 5_000)

      raw_frames = Agent.get(agent, fn list -> Enum.reverse(list) end)
      Agent.stop(agent)

      # Should have sent one error response with nil id.
      assert length(raw_frames) == 1
      {:ok, msg} = MockCodec.decode(hd(raw_frames))
      assert %ErrorResponse{id: nil} = msg
    end
  end

  describe "Server.register_request/3 and register_notification/3" do
    test "33. register_request adds handler to :request key" do
      handlers = Server.register_request(%{}, "ping", fn _id, _params -> "pong" end)
      assert Map.has_key?(handlers, :request)
      assert Map.has_key?(handlers[:request], "ping")
    end

    test "34. register_notification adds handler to :notification key" do
      handlers = Server.register_notification(%{}, "log", fn _params -> :ok end)
      assert Map.has_key?(handlers, :notification)
      assert Map.has_key?(handlers[:notification], "log")
    end

    test "35. registering same method twice replaces the handler" do
      handlers =
        %{}
        |> Server.register_request("ping", fn _id, _params -> "first" end)
        |> Server.register_request("ping", fn _id, _params -> "second" end)

      handler = handlers[:request]["ping"]
      assert handler.(1, nil) == "second"
    end
  end

  # ===========================================================================
  # 4. Client — request/response correlation
  # ===========================================================================

  describe "Client.new/3" do
    test "36. creates a client with next_id starting at 1" do
      client = Client.new(MockCodec, MockFramer, MockFramer.new())
      assert client.next_id == 1
    end

    test "37. creates a client with empty notification handlers" do
      client = Client.new(MockCodec, MockFramer, MockFramer.new())
      assert client.notif_handlers == %{}
    end
  end

  describe "Client.request/3" do
    test "38. sends request and returns decoded result" do
      response = %Response{id: 1, result: "pong"}
      framer_state = MockFramer.new([encode_msg(response)])
      client = Client.new(MockCodec, MockFramer, framer_state)

      {:ok, result, _client2} = Client.request(client, "ping", nil)
      assert result == "pong"
    end

    test "39. request id is 1 on first call" do
      # Use SpyFramer so we can inspect what the client wrote.
      response = %Response{id: 1, result: 42}
      {:ok, agent} = Agent.start_link(fn -> [] end)
      spy_state = SpyFramer.new([encode_msg(response)], agent)
      client = Client.new(MockCodec, SpyFramer, spy_state)

      {:ok, _result, _client2} = Client.request(client, "ping", nil)

      written = Agent.get(agent, fn list -> Enum.reverse(list) end)
      Agent.stop(agent)

      # The first written frame should be the request with id=1.
      [req_bytes | _] = written
      {:ok, req_msg} = MockCodec.decode(req_bytes)
      assert %Request{id: 1, method: "ping"} = req_msg
    end

    test "40. request id increments on successive calls" do
      r1 = %Response{id: 1, result: "first"}
      r2 = %Response{id: 2, result: "second"}
      framer_state = MockFramer.new([encode_msg(r1), encode_msg(r2)])
      client = Client.new(MockCodec, MockFramer, framer_state)

      {:ok, res1, client2} = Client.request(client, "a", nil)
      {:ok, res2, _client3} = Client.request(client2, "b", nil)

      assert res1 == "first"
      assert res2 == "second"
      assert client2.next_id == 2
    end

    test "41. returns error when server responds with ErrorResponse" do
      error_resp = %ErrorResponse{
        id: 1,
        code: -32_601,
        message: "Method not found",
        data: "no_such_method"
      }

      framer_state = MockFramer.new([encode_msg(error_resp)])
      client = Client.new(MockCodec, MockFramer, framer_state)

      {:error, err, _client2} = Client.request(client, "no_such_method", nil)
      assert %ErrorResponse{id: 1, code: -32_601} = err
    end

    test "42. returns error when connection closes before response" do
      # Empty framer — read_frame returns :eof immediately.
      framer_state = MockFramer.new([])
      client = Client.new(MockCodec, MockFramer, framer_state)

      {:error, %ErrorResponse{code: -32_603, data: data}, _} =
        Client.request(client, "ping", nil)

      assert data =~ "connection closed"
    end

    test "43. dispatches server-push notification while waiting for response" do
      parent = self()

      notif = %Notification{method: "server_push", params: "pushed"}
      resp = %Response{id: 1, result: "ok"}
      framer_state = MockFramer.new([encode_msg(notif), encode_msg(resp)])
      client = Client.new(MockCodec, MockFramer, framer_state)

      client2 =
        Client.on_notification(client, "server_push", fn params ->
          send(parent, {:push, params})
        end)

      {:ok, result, _client3} = Client.request(client2, "ping", nil)
      assert result == "ok"
      assert_received {:push, "pushed"}
    end

    test "44. server-push notification with no handler is silently ignored" do
      notif = %Notification{method: "unknown_push", params: "data"}
      resp = %Response{id: 1, result: "ok"}
      framer_state = MockFramer.new([encode_msg(notif), encode_msg(resp)])
      client = Client.new(MockCodec, MockFramer, framer_state)

      # No handler registered — should skip the notification and return result.
      {:ok, result, _client2} = Client.request(client, "ping", nil)
      assert result == "ok"
    end

    test "45. passes params to server (via spy framer)" do
      response = %Response{id: 1, result: "got params"}
      {:ok, agent} = Agent.start_link(fn -> [] end)
      spy_state = SpyFramer.new([encode_msg(response)], agent)
      client = Client.new(MockCodec, SpyFramer, spy_state)

      {:ok, _result, _client2} = Client.request(client, "echo", %{"key" => "val"})

      written = Agent.get(agent, fn list -> Enum.reverse(list) end)
      Agent.stop(agent)

      [req_bytes | _] = written
      {:ok, req_msg} = MockCodec.decode(req_bytes)
      assert %Request{method: "echo", params: %{"key" => "val"}} = req_msg
    end
  end

  describe "Client.notify/3" do
    test "46. notify sends a notification frame" do
      {:ok, agent} = Agent.start_link(fn -> [] end)
      spy_state = SpyFramer.new([], agent)
      client = Client.new(MockCodec, SpyFramer, spy_state)

      {:ok, _client2} = Client.notify(client, "log", %{"msg" => "hello"})

      written = Agent.get(agent, fn list -> Enum.reverse(list) end)
      Agent.stop(agent)

      assert length(written) == 1
      {:ok, msg} = MockCodec.decode(hd(written))
      assert %Notification{method: "log", params: %{"msg" => "hello"}} = msg
    end

    test "47. notify does not read any response frame" do
      # Empty framer — notify should not block or error trying to read.
      {:ok, agent} = Agent.start_link(fn -> [] end)
      spy_state = SpyFramer.new([], agent)
      client = Client.new(MockCodec, SpyFramer, spy_state)

      {:ok, _client2} = Client.notify(client, "fire_and_forget", nil)
      Agent.stop(agent)
    end

    test "48. notify with nil params sends notification with nil params" do
      {:ok, agent} = Agent.start_link(fn -> [] end)
      spy_state = SpyFramer.new([], agent)
      client = Client.new(MockCodec, SpyFramer, spy_state)

      {:ok, _client2} = Client.notify(client, "ping", nil)

      written = Agent.get(agent, fn list -> Enum.reverse(list) end)
      Agent.stop(agent)

      {:ok, msg} = MockCodec.decode(hd(written))
      assert %Notification{method: "ping", params: nil} = msg
    end
  end

  describe "Client.on_notification/3" do
    test "49. registers a notification handler" do
      client = Client.new(MockCodec, MockFramer, MockFramer.new())
      client2 = Client.on_notification(client, "push", fn _params -> :ok end)
      assert Map.has_key?(client2.notif_handlers, "push")
    end

    test "50. overrides an existing handler for the same method" do
      client = Client.new(MockCodec, MockFramer, MockFramer.new())

      client2 =
        client
        |> Client.on_notification("push", fn _params -> :first end)
        |> Client.on_notification("push", fn _params -> :second end)

      handler = client2.notif_handlers["push"]
      assert handler.(nil) == :second
    end
  end

  # ===========================================================================
  # 5. Rpc top-level delegates
  # ===========================================================================

  describe "Rpc top-level delegates" do
    test "51. Rpc.register_request delegates to Server.register_request" do
      handlers = Rpc.register_request(%{}, "ping", fn _id, _params -> "pong" end)
      assert Map.has_key?(handlers, :request)
    end

    test "52. Rpc.register_notification delegates to Server.register_notification" do
      handlers = Rpc.register_notification(%{}, "log", fn _params -> :ok end)
      assert Map.has_key?(handlers, :notification)
    end

    test "53. Rpc.serve delegates to Server.serve (returns :ok on empty input)" do
      :ok = Rpc.serve(MockCodec, MockFramer, MockFramer.new([]), %{})
    end
  end

  # ===========================================================================
  # 6. MockCodec — encode/decode round-trips
  # ===========================================================================

  describe "MockCodec — encode/decode round-trips" do
    test "54. round-trips a Request" do
      msg = %Request{id: 1, method: "ping", params: %{"key" => "value"}}
      {:ok, bytes} = MockCodec.encode(msg)
      {:ok, decoded} = MockCodec.decode(bytes)
      assert decoded == msg
    end

    test "55. round-trips a Response" do
      msg = %Response{id: 1, result: 42}
      {:ok, bytes} = MockCodec.encode(msg)
      {:ok, decoded} = MockCodec.decode(bytes)
      assert decoded == msg
    end

    test "56. round-trips an ErrorResponse" do
      msg = %ErrorResponse{id: 1, code: -32_601, message: "Method not found"}
      {:ok, bytes} = MockCodec.encode(msg)
      {:ok, decoded} = MockCodec.decode(bytes)
      assert decoded == msg
    end

    test "57. round-trips a Notification" do
      msg = %Notification{method: "log", params: "hello"}
      {:ok, bytes} = MockCodec.encode(msg)
      {:ok, decoded} = MockCodec.decode(bytes)
      assert decoded == msg
    end

    test "58. decode returns parse error for garbage bytes" do
      result = MockCodec.decode(<<"not valid erlang">>)
      assert {:error, %ErrorResponse{id: nil, code: -32_700}} = result
    end
  end

  # ===========================================================================
  # 7. MockFramer — read_frame/write_frame
  # ===========================================================================

  describe "MockFramer — read_frame/write_frame" do
    test "59. read_frame returns :eof on empty queue" do
      state = MockFramer.new([])
      assert :eof = MockFramer.read_frame(state)
    end

    test "60. read_frame returns frames in FIFO order" do
      state = MockFramer.new(["first", "second"])
      {:ok, f1, state2} = MockFramer.read_frame(state)
      {:ok, f2, state3} = MockFramer.read_frame(state2)
      assert :eof = MockFramer.read_frame(state3)
      assert f1 == "first"
      assert f2 == "second"
    end

    test "61. write_frame accumulates frames in written_frames order" do
      state = MockFramer.new([])
      {:ok, state2} = MockFramer.write_frame("frame1", state)
      {:ok, state3} = MockFramer.write_frame("frame2", state2)
      assert MockFramer.written_frames(state3) == ["frame1", "frame2"]
    end

    test "62. write then read round-trip via MockCodec" do
      msg = %Request{id: 1, method: "test"}
      {:ok, bytes} = MockCodec.encode(msg)
      state = MockFramer.new([bytes])
      {:ok, read_bytes, _state2} = MockFramer.read_frame(state)
      {:ok, decoded} = MockCodec.decode(read_bytes)
      assert decoded == msg
    end
  end
end
