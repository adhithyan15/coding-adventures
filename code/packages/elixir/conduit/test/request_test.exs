defmodule CodingAdventures.Conduit.RequestTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Conduit.Request

  describe "from_env/1" do
    test "populates all fields from a complete env map" do
      env = %{
        "REQUEST_METHOD" => "POST",
        "PATH_INFO" => "/echo",
        "QUERY_STRING" => "x=1",
        "REMOTE_ADDR" => "127.0.0.1",
        "REMOTE_PORT" => "12345",
        "conduit.route_params" => %{"id" => "42"},
        "conduit.query_params" => %{"x" => "1"},
        "conduit.headers" => %{"content-type" => "application/json"},
        "conduit.body" => "{\"a\":1}",
        "conduit.content_type" => "application/json",
        "conduit.content_length" => "8"
      }

      r = Request.from_env(env)
      assert r.method == "POST"
      assert r.path == "/echo"
      assert r.query_string == "x=1"
      assert r.params == %{"id" => "42"}
      assert r.query_params == %{"x" => "1"}
      assert r.headers == %{"content-type" => "application/json"}
      assert r.body == "{\"a\":1}"
      assert r.content_type == "application/json"
      assert r.content_length == 8
    end

    test "defaults missing fields" do
      r = Request.from_env(%{})
      assert r.method == "GET"
      assert r.path == "/"
      assert r.query_string == ""
      assert r.params == %{}
      assert r.query_params == %{}
      assert r.headers == %{}
      assert r.body == ""
      assert r.content_type == ""
      assert r.content_length == 0
    end

    test "non-numeric content_length defaults to 0" do
      r = Request.from_env(%{"conduit.content_length" => "not-a-number"})
      assert r.content_length == 0
    end

    test "preserves the raw env map" do
      env = %{"REQUEST_METHOD" => "GET", "x.custom" => "y"}
      r = Request.from_env(env)
      assert r.env["x.custom"] == "y"
    end
  end

  describe "json_body!/1" do
    @tag :json
    test "decodes a valid JSON body" do
      if Code.ensure_loaded?(JSON) do
        r = Request.from_env(%{"conduit.body" => "{\"a\":1,\"b\":\"x\"}"})
        assert Request.json_body!(r) == %{"a" => 1, "b" => "x"}
      end
    end

    test "rejects oversize bodies with HaltError(413)" do
      big = String.duplicate("a", 11 * 1024 * 1024)
      r = Request.from_env(%{"conduit.body" => big})

      assert_raise CodingAdventures.Conduit.HaltError, fn ->
        Request.json_body!(r)
      end
    end
  end

  describe "json_body/1 (tagged tuple variant)" do
    @tag :json
    test "ok tuple on valid JSON" do
      if Code.ensure_loaded?(JSON) do
        r = Request.from_env(%{"conduit.body" => "[1,2,3]"})
        assert Request.json_body(r) == {:ok, [1, 2, 3]}
      end
    end

    test "error tuple on invalid JSON" do
      if Code.ensure_loaded?(JSON) do
        r = Request.from_env(%{"conduit.body" => "not json"})
        {result, _reason} = Request.json_body(r)
        assert result == :error
      end
    end
  end
end
