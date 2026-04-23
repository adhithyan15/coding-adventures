# frozen_string_literal: true

require_relative "test_helper"

class TestConduitRouter < Minitest::Test
  def test_route_matches_named_params
    route = CodingAdventures::Conduit::Route.new("GET", "/hello/:name") { "ok" }
    assert_equal({ "name" => "Adhithya" }, route.match?("GET", "/hello/Adhithya"))
    assert_nil route.match?("POST", "/hello/Adhithya")
    assert_nil route.match?("GET", "/hello")
  end

  def test_route_matches_root_path
    route = CodingAdventures::Conduit::Route.new("GET", "/") { "ok" }
    assert_equal({}, route.match?("GET", "/"))
    assert_nil route.match?("GET", "/hello")
  end

  def test_application_normalizes_string_response
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    status, headers, body = app.call(
      "REQUEST_METHOD" => "GET",
      "PATH_INFO" => "/hello/Adhithya",
      "QUERY_STRING" => "",
      "rack.input" => ""
    )

    assert_equal 200, status
    assert_equal "text/plain", headers["content-type"]
    assert_equal ["Hello Adhithya"], body
  end

  def test_request_exposes_preparsed_query_and_header_helpers
    seen = nil
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        seen = request
        "Hello #{request.params.fetch("name")}"
      end
    end

    app.call(
      "REQUEST_METHOD" => "GET",
      "PATH_INFO" => "/hello/Adhithya",
      "QUERY_STRING" => "lang=rust",
      "rack.input" => "hello",
      "conduit.query_params" => { "lang" => "rust", "empty" => "" },
      "conduit.headers" => { "content-type" => "text/plain", "x-name" => "Adhithya" },
      "conduit.content_length" => 5,
      "conduit.content_type" => "text/plain"
    )

    refute_nil seen
    assert_equal({ "lang" => "rust", "empty" => "" }, seen.query_params)
    assert_equal "Adhithya", seen.header("X-Name")
    assert_equal 5, seen.content_length
    assert_equal "text/plain", seen.content_type
    assert_equal "hello", seen.body
  end
end

class TestConduitServer < Minitest::Test
  def with_server(app)
    server = CodingAdventures::Conduit::Server.new(app, port: 0)
    thread = server.start
    yield server
  ensure
    server&.stop
    thread&.join(5)
    server&.close
  end

  def test_native_server_handles_hello_route_end_to_end
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    with_server(app) do |server|
      uri = URI("http://#{server.host}:#{server.port}/hello/Adhithya")
      response = Net::HTTP.get_response(uri)

      assert_equal "200", response.code
      assert_equal "Hello Adhithya", response.body
      assert_equal "text/plain", response["content-type"]
    end
  end

  def test_native_server_returns_not_found_for_missing_route
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    with_server(app) do |server|
      uri = URI("http://#{server.host}:#{server.port}/missing")
      response = Net::HTTP.get_response(uri)

      assert_equal "404", response.code
      assert_equal "Not Found", response.body
    end
  end
end
