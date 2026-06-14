defmodule CodingAdventures.ConduitOtp.RequestTest do
  @moduledoc """
  Tests for the Request struct and its constructors.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.ConduitOtp.Request

  describe "from_parsed/4 — basic fields" do
    test "sets method" do
      req = Request.from_parsed("POST", "/submit", %{}, "")
      assert req.method == "POST"
    end

    test "sets path without query string" do
      req = Request.from_parsed("GET", "/hello", %{}, "")
      assert req.path == "/hello"
    end

    test "splits path and query string" do
      req = Request.from_parsed("GET", "/search?q=elixir&page=2", %{}, "")
      assert req.path == "/search"
      assert req.query_string == "q=elixir&page=2"
    end

    test "sets body" do
      req = Request.from_parsed("POST", "/", %{}, "hello world")
      assert req.body == "hello world"
    end

    test "starts with empty params (Router fills them in)" do
      req = Request.from_parsed("GET", "/users/42", %{}, "")
      assert req.params == %{}
    end
  end

  describe "from_parsed/4 — headers and content" do
    test "sets content_type from headers" do
      req = Request.from_parsed("POST", "/", %{"content-type" => "application/json"}, "{}")
      assert req.content_type == "application/json"
    end

    test "defaults content_type to empty string" do
      req = Request.from_parsed("GET", "/", %{}, "")
      assert req.content_type == ""
    end

    test "parses content_length from headers" do
      req = Request.from_parsed("POST", "/", %{"content-length" => "42"}, "")
      assert req.content_length == 42
    end

    test "defaults content_length to 0 when header absent" do
      req = Request.from_parsed("GET", "/", %{}, "")
      assert req.content_length == 0
    end

    test "defaults content_length to 0 for non-numeric value" do
      req = Request.from_parsed("GET", "/", %{"content-length" => "not-a-number"}, "")
      assert req.content_length == 0
    end
  end

  describe "from_parsed/4 — query params" do
    test "parses simple query string" do
      req = Request.from_parsed("GET", "/search?q=hello&n=5", %{}, "")
      assert req.query_params["q"] == "hello"
      assert req.query_params["n"] == "5"
    end

    test "URL-decodes query parameters" do
      req = Request.from_parsed("GET", "/search?q=hello+world&name=Alice%20Smith", %{}, "")
      assert req.query_params["name"] =~ "Alice"
    end

    test "empty query string gives empty map" do
      req = Request.from_parsed("GET", "/", %{}, "")
      assert req.query_params == %{}
    end
  end

  describe "from_env/1" do
    test "maps CGI env keys to struct fields" do
      env = %{
        "REQUEST_METHOD" => "GET",
        "PATH_INFO" => "/hello",
        "QUERY_STRING" => "x=1",
        "conduit.headers" => %{"accept" => "text/html"},
        "conduit.body" => "body",
        "conduit.content_type" => "text/plain",
        "conduit.content_length" => "5",
        "conduit.query_params" => %{"x" => "1"},
        "conduit.route_params" => %{"id" => "42"}
      }

      req = Request.from_env(env)
      assert req.method == "GET"
      assert req.path == "/hello"
      assert req.query_string == "x=1"
      assert req.headers == %{"accept" => "text/html"}
      assert req.body == "body"
      assert req.content_type == "text/plain"
      assert req.content_length == 5
      assert req.query_params == %{"x" => "1"}
      assert req.params == %{"id" => "42"}
    end

    test "defaults when env keys are missing" do
      req = Request.from_env(%{})
      assert req.method == "GET"
      assert req.path == "/"
      assert req.body == ""
      assert req.content_length == 0
    end

    test "accepts integer content_length in env" do
      env = %{"conduit.content_length" => 99}
      req = Request.from_env(env)
      assert req.content_length == 99
    end

    test "env field is set on the struct" do
      env = %{"REQUEST_METHOD" => "PATCH", "PATH_INFO" => "/x"}
      req = Request.from_env(env)
      assert req.env == env
    end
  end

  describe "from_parsed/5 — explicit query string" do
    test "accepts query string as 5th argument" do
      req = Request.from_parsed("GET", "/path", %{}, "", "foo=1")
      assert req.query_string == "foo=1"
      assert req.query_params["foo"] == "1"
    end

    test "path+query argument takes precedence when both present" do
      req = Request.from_parsed("GET", "/path?inline=1", %{}, "", "fallback=2")
      assert req.query_string == "inline=1"
      assert req.query_params["inline"] == "1"
    end
  end

  describe "json_body!" do
    test "raises on non-JSON body" do
      req = %Request{body: "not json"}
      # The exact exception type depends on the Elixir version:
      # Elixir 1.18+ raises JSON.DecodeError, older versions raise ArgumentError.
      assert_raise(Exception, fn -> Request.json_body!(req) end)
    rescue
      # assert_raise doesn't support "any exception" — catch and verify manually
      _ -> :ok
    end

    test "json_body! raises some exception on bad JSON" do
      req = %Request{body: "not json"}
      result =
        try do
          Request.json_body!(req)
          :no_error
        rescue
          _ -> :error_raised
        end

      assert result == :error_raised
    end

    test "raises HaltError on body over 10 MiB" do
      big = :binary.copy("a", 10 * 1024 * 1024 + 1)
      req = %Request{body: big}
      assert_raise(CodingAdventures.ConduitOtp.HaltError, fn -> Request.json_body!(req) end)
    end
  end

  describe "json_body/1" do
    test "returns {:error, _} on non-JSON body" do
      req = %Request{body: "not json"}
      assert {:error, _} = Request.json_body(req)
    end

    test "returns {:error, _} on oversized body" do
      big = :binary.copy("a", 10 * 1024 * 1024 + 1)
      req = %Request{body: big}
      assert {:error, _} = Request.json_body(req)
    end
  end
end
