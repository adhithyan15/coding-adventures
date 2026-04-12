defmodule CodingAdventures.JsonRpcTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for the CodingAdventures.JsonRpc package.

  ## Test Organization

  1. Errors — constants and constructors
  2. Message — parse_message and message_to_map
  3. Writer — Content-Length framing
  4. Reader — header parsing, EOF, error cases
  5. Server — dispatch, error responses, round-trips
  6. Round-trip — full encode/decode pipeline

  ## Testing Strategy

  We use `StringIO` for in-memory I/O, which lets tests run fully in memory
  without touching stdin/stdout. A `StringIO` pid behaves like any Erlang I/O
  device — it accepts `:file.read/2` and `IO.binwrite/2` calls.

  ### Building a framed message for Reader tests

      defp frame(json) do
        n = byte_size(json)
        "Content-Length: \#{n}\\r\\n\\r\\n\#{json}"
      end
  """

  alias CodingAdventures.JsonRpc.{Errors, JsonCodec, Message, Reader, Writer, Server}
  alias CodingAdventures.JsonRpc.Message.{Notification, Request, Response}

  # ===========================================================================
  # Helpers
  # ===========================================================================

  # Build a Content-Length-framed message string for feeding to Reader.
  defp frame(json) when is_binary(json) do
    n = byte_size(json)
    "Content-Length: #{n}\r\n\r\n#{json}"
  end

  # Open a StringIO with content and return the pid.
  defp open_input(content) do
    {:ok, pid} = StringIO.open(content)
    pid
  end

  # Open a writable StringIO and return the pid.
  defp open_output() do
    {:ok, pid} = StringIO.open("")
    pid
  end

  # Read the output written to a StringIO pid.
  defp get_output(pid) do
    {_in, out} = StringIO.contents(pid)
    out
  end

  # ===========================================================================
  # 1. Errors — constants and constructors
  # ===========================================================================

  describe "Errors — code constants" do
    test "1. parse_error code is -32700" do
      assert Errors.parse_error() == -32_700
    end

    test "2. invalid_request code is -32600" do
      assert Errors.invalid_request() == -32_600
    end

    test "3. method_not_found code is -32601" do
      assert Errors.method_not_found() == -32_601
    end

    test "4. invalid_params code is -32602" do
      assert Errors.invalid_params() == -32_602
    end

    test "5. internal_error code is -32603" do
      assert Errors.internal_error() == -32_603
    end
  end

  describe "Errors — constructors" do
    test "6. make_parse_error without data" do
      err = Errors.make_parse_error()
      assert err.code == -32_700
      assert err.message == "Parse error"
      refute Map.has_key?(err, :data)
    end

    test "7. make_parse_error with data" do
      err = Errors.make_parse_error("unexpected token")
      assert err.code == -32_700
      assert err.data == "unexpected token"
    end

    test "8. make_method_not_found with method name" do
      err = Errors.make_method_not_found("textDocument/hover")
      assert err.code == -32_601
      assert err.data == "textDocument/hover"
    end

    test "9. make_internal_error" do
      err = Errors.make_internal_error("crash")
      assert err.code == -32_603
      assert err.data == "crash"
    end
  end

  # ===========================================================================
  # 2. Message — parse_message/1
  # ===========================================================================

  describe "Message.parse_message/1 — Request" do
    test "10. parses a minimal request (id + method)" do
      json = ~s({"jsonrpc":"2.0","id":1,"method":"initialize"})
      assert {:ok, %Request{id: 1, method: "initialize", params: nil}} = Message.parse_message(json)
    end

    test "11. parses request with params object" do
      json = ~s({"jsonrpc":"2.0","id":2,"method":"hover","params":{"line":0}})
      assert {:ok, %Request{id: 2, method: "hover", params: %{"line" => 0}}} =
               Message.parse_message(json)
    end

    test "12. parses request with string id" do
      json = ~s({"jsonrpc":"2.0","id":"abc","method":"ping"})
      assert {:ok, %Request{id: "abc", method: "ping"}} = Message.parse_message(json)
    end
  end

  describe "Message.parse_message/1 — Notification" do
    test "13. parses a notification (method, no id)" do
      json = ~s({"jsonrpc":"2.0","method":"textDocument/didOpen"})
      assert {:ok, %Notification{method: "textDocument/didOpen", params: nil}} =
               Message.parse_message(json)
    end

    test "14. parses notification with params" do
      json = ~s({"jsonrpc":"2.0","method":"initialized","params":{}})
      assert {:ok, %Notification{method: "initialized", params: %{}}} =
               Message.parse_message(json)
    end
  end

  describe "Message.parse_message/1 — Response" do
    test "15. parses a success response" do
      json = ~s({"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}})
      assert {:ok, %Response{id: 1, result: %{"capabilities" => %{}}}} =
               Message.parse_message(json)
    end

    test "16. parses an error response" do
      json = ~s({"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}})
      assert {:ok, %Response{id: 1, error: %{"code" => -32_601}}} =
               Message.parse_message(json)
    end

    test "17. parses response with null id" do
      json = ~s({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}})
      assert {:ok, %Response{id: nil}} = Message.parse_message(json)
    end
  end

  describe "Message.parse_message/1 — errors" do
    test "18. returns parse error (-32700) for invalid JSON" do
      assert {:error, %{code: -32_700}} = Message.parse_message("not json!!!")
    end

    test "19. returns parse error for empty string" do
      assert {:error, %{code: _}} = Message.parse_message("")
    end

    test "20. returns invalid request (-32600) for JSON array" do
      assert {:error, %{code: -32_600}} = Message.parse_message("[1,2,3]")
    end

    test "21. returns invalid request for JSON number" do
      assert {:error, %{code: -32_600}} = Message.parse_message("42")
    end

    test "22. returns invalid request for object with no method/result/error" do
      json = ~s({"jsonrpc":"2.0","foo":"bar"})
      assert {:error, %{code: -32_600}} = Message.parse_message(json)
    end
  end

  # ===========================================================================
  # 3. Message.message_to_map/1
  # ===========================================================================

  describe "Message.message_to_map/1" do
    test "23. Request → map includes jsonrpc, id, method" do
      req = %Request{id: 5, method: "ping"}
      map = Message.message_to_map(req)
      assert map["jsonrpc"] == "2.0"
      assert map["id"] == 5
      assert map["method"] == "ping"
      refute Map.has_key?(map, "params")
    end

    test "24. Request with params → includes params key" do
      req = %Request{id: 1, method: "hover", params: %{"line" => 3}}
      map = Message.message_to_map(req)
      assert map["params"] == %{"line" => 3}
    end

    test "25. Response (success) → map includes result, no error" do
      resp = %Response{id: 1, result: %{"ok" => true}}
      map = Message.message_to_map(resp)
      assert map["result"] == %{"ok" => true}
      refute Map.has_key?(map, "error")
    end

    test "26. Response (error) → map includes error, no result" do
      resp = %Response{id: 1, error: %{code: -32_601, message: "Method not found"}}
      map = Message.message_to_map(resp)
      assert map["error"] == %{code: -32_601, message: "Method not found"}
      refute Map.has_key?(map, "result")
    end

    test "27. Notification → map has jsonrpc and method, no id" do
      notif = %Notification{method: "initialized"}
      map = Message.message_to_map(notif)
      assert map["jsonrpc"] == "2.0"
      assert map["method"] == "initialized"
      refute Map.has_key?(map, "id")
    end
  end

  # ===========================================================================
  # 4. Writer — Content-Length framing
  # ===========================================================================

  describe "Writer.write_message/2" do
    test "28. writes correct Content-Length header" do
      out = open_output()
      writer = Writer.new(out)
      msg = %Response{id: 1, result: nil}
      :ok = Writer.write_message(writer, msg)
      output = get_output(out)

      # Extract the Content-Length value from the header.
      [header_line | _] = String.split(output, "\r\n")
      assert String.starts_with?(header_line, "Content-Length: ")
      len_str = String.replace_prefix(header_line, "Content-Length: ", "")
      {declared_len, ""} = Integer.parse(len_str)

      # The declared length must match the actual payload byte size.
      [_, payload] = String.split(output, "\r\n\r\n", parts: 2)
      assert byte_size(payload) == declared_len
    end

    test "29. payload is valid JSON" do
      out = open_output()
      writer = Writer.new(out)
      msg = %Response{id: 42, result: %{"hello" => "world"}}
      :ok = Writer.write_message(writer, msg)
      output = get_output(out)
      [_, payload] = String.split(output, "\r\n\r\n", parts: 2)
      assert {:ok, _decoded} = JsonCodec.decode(payload)
    end

    test "30. header and payload separated by \\r\\n\\r\\n" do
      out = open_output()
      writer = Writer.new(out)
      msg = %Notification{method: "ping"}
      :ok = Writer.write_message(writer, msg)
      output = get_output(out)
      assert String.contains?(output, "\r\n\r\n")
    end

    test "31. write_raw writes framed bytes directly" do
      out = open_output()
      json = ~s({"jsonrpc":"2.0","method":"test"})
      :ok = Writer.write_raw(out, json)
      output = get_output(out)
      assert String.starts_with?(output, "Content-Length: ")
      [_, payload] = String.split(output, "\r\n\r\n", parts: 2)
      assert payload == json
    end
  end

  # ===========================================================================
  # 5. Reader — reading framed messages
  # ===========================================================================

  describe "Reader.read_message/1 — success cases" do
    test "32. reads a single request" do
      json = ~s({"jsonrpc":"2.0","id":1,"method":"initialize"})
      pid = open_input(frame(json))
      reader = Reader.new(pid)
      assert {:ok, %Request{id: 1, method: "initialize"}} = Reader.read_message(reader)
    end

    test "33. reads back-to-back messages" do
      json1 = ~s({"jsonrpc":"2.0","id":1,"method":"ping"})
      json2 = ~s({"jsonrpc":"2.0","method":"notify"})
      pid = open_input(frame(json1) <> frame(json2))
      reader = Reader.new(pid)
      assert {:ok, %Request{id: 1}} = Reader.read_message(reader)
      assert {:ok, %Notification{method: "notify"}} = Reader.read_message(reader)
    end

    test "34. returns {:ok, nil} on EOF" do
      pid = open_input("")
      reader = Reader.new(pid)
      assert {:ok, nil} = Reader.read_message(reader)
    end

    test "35. reads notification" do
      json = ~s({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}})
      pid = open_input(frame(json))
      reader = Reader.new(pid)
      assert {:ok, %Notification{method: "textDocument/didOpen"}} = Reader.read_message(reader)
    end

    test "36. reads response with result" do
      json = ~s({"jsonrpc":"2.0","id":5,"result":{"capabilities":{}}})
      pid = open_input(frame(json))
      reader = Reader.new(pid)
      assert {:ok, %Response{id: 5}} = Reader.read_message(reader)
    end
  end

  describe "Reader.read_message/1 — error cases" do
    test "37. returns parse error for malformed JSON" do
      # Content-Length header is valid but the payload is not JSON.
      broken = "Content-Length: 4\r\n\r\nbrok"
      pid = open_input(broken)
      reader = Reader.new(pid)
      result = Reader.read_message(reader)
      assert {:error, %{code: -32_700}} = result
    end

    test "38. returns error for missing Content-Length header" do
      # A blank line but no Content-Length — this is malformed framing.
      pid = open_input("\r\n")
      reader = Reader.new(pid)
      result = Reader.read_message(reader)
      assert {:error, %{code: -32_700}} = result
    end

    test "39. returns invalid request for valid JSON that is not a message" do
      json = "[1, 2, 3]"
      pid = open_input(frame(json))
      reader = Reader.new(pid)
      assert {:error, %{code: -32_600}} = Reader.read_message(reader)
    end

    test "40. reads three messages, last read returns nil" do
      json = ~s({"jsonrpc":"2.0","id":1,"method":"a"})
      pid = open_input(frame(json))
      reader = Reader.new(pid)
      assert {:ok, %Request{}} = Reader.read_message(reader)
      assert {:ok, nil} = Reader.read_message(reader)
    end
  end

  # ===========================================================================
  # 6. Server — dispatch loop
  # ===========================================================================

  # Helper: run the server against a sequence of framed input messages and
  # capture output. We run the server in a Task because serve/1 is blocking.
  defp run_server(input_frames, register_fn) do
    all_input = Enum.join(input_frames)
    in_pid = open_input(all_input)
    out_pid = open_output()

    server = Server.new(in_pid, out_pid) |> register_fn.()

    # Run in a Task with a timeout to avoid hanging if serve() doesn't terminate.
    task = Task.async(fn -> Server.serve(server) end)
    Task.await(task, 5_000)

    get_output(out_pid)
  end

  # Parse all framed responses from the server output.
  defp parse_all_responses(output) do
    parse_frames(output, [])
  end

  defp parse_frames("", acc), do: Enum.reverse(acc)

  defp parse_frames(bin, acc) do
    case String.split(bin, "\r\n\r\n", parts: 2) do
      [_header, rest] ->
        # Find payload length from header.
        header_line = List.first(String.split(bin, "\r\n"))
        len_str = String.replace_prefix(header_line, "Content-Length: ", "")
        {n, ""} = Integer.parse(len_str)
        <<payload::binary-size(n), after_payload::binary>> = rest
        {:ok, msg} = Message.parse_message(payload)
        parse_frames(after_payload, [msg | acc])

      _ ->
        Enum.reverse(acc)
    end
  end

  describe "Server — request dispatch" do
    test "41. dispatches request to handler and writes success response" do
      input = [frame(~s({"jsonrpc":"2.0","id":1,"method":"ping"}))]

      output =
        run_server(input, fn server ->
          Server.on_request(server, "ping", fn _id, _params -> %{"pong" => true} end)
        end)

      responses = parse_all_responses(output)
      assert length(responses) == 1
      [resp] = responses
      assert %Response{id: 1, result: %{"pong" => true}} = resp
    end

    test "42. sends -32601 for unknown method" do
      input = [frame(~s({"jsonrpc":"2.0","id":2,"method":"unknown"}))]

      output = run_server(input, fn server -> server end)
      responses = parse_all_responses(output)
      assert length(responses) == 1
      [resp] = responses
      assert %Response{id: 2, error: error} = resp
      assert error["code"] == -32_601
    end

    test "43. handler error map is sent as error response" do
      input = [frame(~s({"jsonrpc":"2.0","id":3,"method":"fail"}))]

      output =
        run_server(input, fn server ->
          Server.on_request(server, "fail", fn _id, _params ->
            %{code: -32_602, message: "Invalid params"}
          end)
        end)

      responses = parse_all_responses(output)
      [resp] = responses
      assert %Response{id: 3, error: error} = resp
      assert error[:code] == -32_602 or error["code"] == -32_602
    end

    test "44. handler exception results in internal error response" do
      input = [frame(~s({"jsonrpc":"2.0","id":4,"method":"boom"}))]

      output =
        run_server(input, fn server ->
          Server.on_request(server, "boom", fn _id, _params ->
            raise RuntimeError, "kaboom"
          end)
        end)

      responses = parse_all_responses(output)
      [resp] = responses
      assert %Response{id: 4, error: error} = resp
      assert (error["code"] || error[:code]) == -32_603
    end
  end

  describe "Server — notification dispatch" do
    test "45. dispatches notification to handler, no response written" do
      # Collect side-effects via a process message.
      parent = self()
      input = [frame(~s({"jsonrpc":"2.0","method":"ping"}))]

      output =
        run_server(input, fn server ->
          Server.on_notification(server, "ping", fn _params ->
            send(parent, :notified)
          end)
        end)

      assert_received :notified
      # No response should be written for a notification.
      assert output == ""
    end

    test "46. unknown notification is silently ignored" do
      input = [frame(~s({"jsonrpc":"2.0","method":"unknown"}))]
      output = run_server(input, fn server -> server end)
      assert output == ""
    end
  end

  describe "Server — multiple messages" do
    test "47. handles request then notification in sequence" do
      parent = self()

      input = [
        frame(~s({"jsonrpc":"2.0","id":1,"method":"ping"})),
        frame(~s({"jsonrpc":"2.0","method":"notify"}))
      ]

      output =
        run_server(input, fn server ->
          server
          |> Server.on_request("ping", fn _id, _params -> "pong" end)
          |> Server.on_notification("notify", fn _params -> send(parent, :notified) end)
        end)

      assert_received :notified
      responses = parse_all_responses(output)
      assert length(responses) == 1
      [resp] = responses
      assert %Response{id: 1, result: "pong"} = resp
    end

    test "48. server terminates cleanly on EOF" do
      # Empty input — serve should return :ok immediately.
      in_pid = open_input("")
      out_pid = open_output()
      server = Server.new(in_pid, out_pid)
      assert :ok = Server.serve(server)
    end
  end

  # ===========================================================================
  # 7. Round-trip tests
  # ===========================================================================

  describe "Round-trip: write then read" do
    test "49. Request round-trip" do
      out = open_output()
      writer = Writer.new(out)
      req = %Request{id: 10, method: "hover", params: %{"line" => 5}}
      :ok = Writer.write_message(writer, req)

      written = get_output(out)
      in_pid = open_input(written)
      reader = Reader.new(in_pid)

      assert {:ok, %Request{id: 10, method: "hover", params: %{"line" => 5}}} =
               Reader.read_message(reader)
    end

    test "50. Notification round-trip" do
      out = open_output()
      writer = Writer.new(out)
      notif = %Notification{method: "textDocument/didSave", params: %{"uri" => "file:///a.bf"}}
      :ok = Writer.write_message(writer, notif)

      written = get_output(out)
      in_pid = open_input(written)
      reader = Reader.new(in_pid)

      assert {:ok, %Notification{method: "textDocument/didSave"}} = Reader.read_message(reader)
    end

    test "51. Response round-trip" do
      out = open_output()
      writer = Writer.new(out)
      resp = %Response{id: 99, result: %{"capabilities" => %{"hover" => true}}}
      :ok = Writer.write_message(writer, resp)

      written = get_output(out)
      in_pid = open_input(written)
      reader = Reader.new(in_pid)

      assert {:ok, %Response{id: 99, result: %{"capabilities" => %{"hover" => true}}}} =
               Reader.read_message(reader)
    end

    test "52. back-to-back round-trips" do
      out = open_output()
      writer = Writer.new(out)

      messages = [
        %Request{id: 1, method: "a"},
        %Notification{method: "b"},
        %Response{id: 1, result: 42}
      ]

      Enum.each(messages, &Writer.write_message(writer, &1))

      written = get_output(out)
      in_pid = open_input(written)
      reader = Reader.new(in_pid)

      assert {:ok, %Request{id: 1, method: "a"}} = Reader.read_message(reader)
      assert {:ok, %Notification{method: "b"}} = Reader.read_message(reader)
      assert {:ok, %Response{id: 1, result: 42}} = Reader.read_message(reader)
      assert {:ok, nil} = Reader.read_message(reader)
    end
  end

  # ===========================================================================
  # 8. JsonCodec — internal codec
  # ===========================================================================

  describe "JsonCodec — basic encode/decode" do
    test "53. encodes a map to JSON" do
      {:ok, json} = JsonCodec.encode(%{"a" => 1})
      assert is_binary(json)
      assert String.contains?(json, "\"a\"")
    end

    test "54. decodes JSON to a map with string keys" do
      {:ok, map} = JsonCodec.decode(~s({"key":"value"}))
      assert map["key"] == "value"
    end

    test "55. decode returns error for invalid JSON" do
      assert {:error, _} = JsonCodec.decode("not json")
    end

    test "56. round-trips nil (null)" do
      {:ok, json} = JsonCodec.encode(nil)
      {:ok, val} = JsonCodec.decode(json)
      assert val == nil
    end

    test "57. round-trips boolean" do
      {:ok, json} = JsonCodec.encode(true)
      {:ok, val} = JsonCodec.decode(json)
      assert val == true
    end
  end
end
