# frozen_string_literal: true

require_relative "test_helper"

# =============================================================================
# HaltError
# =============================================================================

class TestHaltError < Minitest::Test
  def test_stores_status_body_and_headers
    err = CodingAdventures::Conduit::HaltError.new(403, "Forbidden", { "x-reason" => "auth" })
    assert_equal 403, err.status
    assert_equal "Forbidden", err.body
    assert_equal [["x-reason", "auth"]], err.halt_headers
  end

  def test_defaults_to_empty_body_and_headers
    err = CodingAdventures::Conduit::HaltError.new(204)
    assert_equal 204, err.status
    assert_equal "", err.body
    assert_equal [], err.halt_headers
  end

  def test_normalizes_hash_headers_to_pairs
    err = CodingAdventures::Conduit::HaltError.new(200, "ok", { "content-type" => "text/plain" })
    assert_equal [["content-type", "text/plain"]], err.halt_headers
  end

  def test_accepts_array_of_pairs_headers
    err = CodingAdventures::Conduit::HaltError.new(200, "ok", [["x-a", "1"], ["x-b", "2"]])
    assert_equal [["x-a", "1"], ["x-b", "2"]], err.halt_headers
  end
end

# =============================================================================
# HandlerContext
# =============================================================================

class TestHandlerContext < Minitest::Test
  def ctx(env = {})
    request = CodingAdventures::Conduit::Request.new(
      { "REQUEST_METHOD" => "GET", "PATH_INFO" => "/", "QUERY_STRING" => "" }.merge(env),
      params: {}, query_params: {}, headers: {}
    )
    CodingAdventures::Conduit::HandlerContext.new(request)
  end

  def test_json_raises_halt_error_with_json_content_type
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.json({ name: "Adhithya" }) }
    assert_equal 200, err.status
    assert_equal '{"name":"Adhithya"}', err.body
    assert_includes err.halt_headers.map { |k, _v| k }, "content-type"
    assert_includes err.halt_headers.assoc("content-type").last, "application/json"
  end

  def test_json_accepts_custom_status
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.json({ ok: false }, 422) }
    assert_equal 422, err.status
  end

  def test_html_raises_halt_error_with_html_content_type
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.html("<h1>Hi</h1>") }
    assert_equal 200, err.status
    assert_equal "<h1>Hi</h1>", err.body
    assert_includes err.halt_headers.assoc("content-type").last, "text/html"
  end

  def test_text_raises_halt_error_with_plain_content_type
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.text("Hello") }
    assert_equal 200, err.status
    assert_includes err.halt_headers.assoc("content-type").last, "text/plain"
  end

  def test_halt_raises_with_exact_status_and_body
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.halt(503, "Maintenance") }
    assert_equal 503, err.status
    assert_equal "Maintenance", err.body
  end

  def test_redirect_raises_halt_with_location_header
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.redirect("/home") }
    assert_equal 302, err.status
    assert_equal "/home", err.halt_headers.assoc("location").last
  end

  def test_redirect_accepts_custom_status
    c = ctx
    err = assert_raises(CodingAdventures::Conduit::HaltError) { c.redirect("/new", 301) }
    assert_equal 301, err.status
  end

  def test_params_delegates_to_request_params
    request = CodingAdventures::Conduit::Request.new(
      { "REQUEST_METHOD" => "GET", "PATH_INFO" => "/hello/Adhithya", "QUERY_STRING" => "" },
      params: { "name" => "Adhithya" },
      query_params: {},
      headers: {}
    )
    c = CodingAdventures::Conduit::HandlerContext.new(request)
    assert_equal({ "name" => "Adhithya" }, c.params)
  end
end

# =============================================================================
# Application DSL (pure Ruby, no server)
# =============================================================================

class TestApplicationDsl < Minitest::Test
  def test_before_appends_filter
    app = CodingAdventures::Conduit::Application.new do
      before { "noop" }
      before { "noop2" }
    end
    assert_equal 2, app.before_filters.length
  end

  def test_after_appends_filter
    app = CodingAdventures::Conduit::Application.new do
      after { "noop" }
    end
    assert_equal 1, app.after_filters.length
  end

  def test_not_found_sets_handler
    app = CodingAdventures::Conduit::Application.new do
      not_found { "gone" }
    end
    refute_nil app.not_found_handler
  end

  def test_error_sets_handler
    app = CodingAdventures::Conduit::Application.new do
      error { "oops" }
    end
    refute_nil app.error_handler
  end

  def test_settings_round_trip
    app = CodingAdventures::Conduit::Application.new do
      set :app_name, "Test"
      set :port, 3000
    end
    assert_equal "Test", app.settings[:app_name]
    assert_equal 3000, app.settings[:port]
  end

  def test_defaults_are_empty
    app = CodingAdventures::Conduit::Application.new
    assert_equal [], app.before_filters
    assert_equal [], app.after_filters
    assert_nil app.not_found_handler
    assert_nil app.error_handler
    assert_equal({}, app.settings)
  end
end

# =============================================================================
# Request body parsing (pure Ruby)
# =============================================================================

class TestRequestBodyParsing < Minitest::Test
  def test_json_parses_json_body
    env = {
      "REQUEST_METHOD" => "POST",
      "PATH_INFO" => "/",
      "QUERY_STRING" => "",
      "rack.input" => '{"name":"Adhithya"}'
    }
    request = CodingAdventures::Conduit::Request.new(env)
    assert_equal({ "name" => "Adhithya" }, request.json)
  end

  def test_json_is_memoized
    env = {
      "REQUEST_METHOD" => "POST",
      "PATH_INFO" => "/",
      "QUERY_STRING" => "",
      "rack.input" => '{"x":1}'
    }
    request = CodingAdventures::Conduit::Request.new(env)
    assert_same request.json, request.json
  end

  def test_form_parses_url_encoded_body
    env = {
      "REQUEST_METHOD" => "POST",
      "PATH_INFO" => "/",
      "QUERY_STRING" => "",
      "rack.input" => "name=Adhithya&lang=ruby"
    }
    request = CodingAdventures::Conduit::Request.new(env)
    assert_equal({ "name" => "Adhithya", "lang" => "ruby" }, request.form)
  end

  def test_form_returns_empty_hash_for_empty_body
    env = {
      "REQUEST_METHOD" => "POST",
      "PATH_INFO" => "/",
      "QUERY_STRING" => "",
      "rack.input" => ""
    }
    request = CodingAdventures::Conduit::Request.new(env)
    assert_equal({}, request.form)
  end
end

# =============================================================================
# Router (pure Ruby, unchanged baseline)
# =============================================================================

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

  def test_native_matcher_works_from_any_thread
    route = CodingAdventures::Conduit::Route.new("GET", "/hello/:name") { "ok" }
    params = Thread.new { route.match?("GET", "/hello/Adhithya") }.value
    assert_equal({ "name" => "Adhithya" }, params)
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

# =============================================================================
# End-to-end tests via real TCP server
# =============================================================================

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

  def http_get(server, path)
    Net::HTTP.get_response(URI("http://#{server.host}:#{server.port}#{path}"))
  end

  def http_post(server, path, body, headers = {})
    uri = URI("http://#{server.host}:#{server.port}#{path}")
    Net::HTTP.start(uri.host, uri.port) do |http|
      req = Net::HTTP::Post.new(uri.path)
      headers.each { |k, v| req[k] = v }
      req.body = body
      http.request(req)
    end
  end

  # --- Baseline route tests (unchanged) ---

  def test_native_server_handles_hello_route_end_to_end
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/hello/Adhithya")
      assert_equal "200", response.code
      assert_equal "Hello Adhithya", response.body
    end
  end

  def test_native_server_returns_not_found_for_missing_route
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/missing")
      assert_equal "404", response.code
    end
  end

  # --- HandlerContext helpers (end-to-end) ---

  def test_json_helper_sets_content_type_and_body
    app = CodingAdventures::Conduit.app do
      get "/data" do
        json({ message: "hello" })
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/data")
      assert_equal "200", response.code
      assert_includes response["content-type"], "application/json"
      assert_equal '{"message":"hello"}', response.body
    end
  end

  def test_html_helper_sets_content_type_and_body
    app = CodingAdventures::Conduit.app do
      get "/" do
        html "<h1>Hello</h1>"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/")
      assert_equal "200", response.code
      assert_includes response["content-type"], "text/html"
      assert_equal "<h1>Hello</h1>", response.body
    end
  end

  def test_halt_sends_custom_status_and_body
    app = CodingAdventures::Conduit.app do
      get "/secret" do
        halt(403, "Forbidden")
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/secret")
      assert_equal "403", response.code
      assert_equal "Forbidden", response.body
    end
  end

  def test_redirect_sends_location_header
    app = CodingAdventures::Conduit.app do
      get "/old" do
        redirect "/new", 301
      end
      get "/new" do
        "New page"
      end
    end

    with_server(app) do |server|
      uri = URI("http://#{server.host}:#{server.port}/old")
      response = Net::HTTP.start(uri.host, uri.port) do |http|
        req = Net::HTTP::Get.new(uri.path)
        http.request(req)
      end
      assert_equal "301", response.code
      assert_equal "/new", response["location"]
    end
  end

  def test_params_shorthand_available_in_zero_arity_block
    app = CodingAdventures::Conduit.app do
      get "/hello/:name" do
        "Hi #{params["name"]}"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/hello/Adhithya")
      assert_equal "200", response.code
      assert_equal "Hi Adhithya", response.body
    end
  end

  # --- Before filters ---

  def test_before_filter_fires_before_handler
    order = []
    app = CodingAdventures::Conduit.app do
      before { order << :before }
      get "/" do
        order << :handler
        "ok"
      end
    end

    with_server(app) do |server|
      http_get(server, "/")
      assert_equal [:before, :handler], order
    end
  end

  def test_before_filter_halt_short_circuits_handler
    handler_called = false
    app = CodingAdventures::Conduit.app do
      before { halt(503, "Maintenance") }
      get "/" do
        handler_called = true
        "ok"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/")
      assert_equal "503", response.code
      assert_equal "Maintenance", response.body
      refute handler_called
    end
  end

  def test_before_filter_can_access_request_path
    seen_path = nil
    app = CodingAdventures::Conduit.app do
      before { |request| seen_path = request.path }
      get "/hello/:name" do |request|
        "Hello #{request.params["name"]}"
      end
    end

    with_server(app) do |server|
      http_get(server, "/hello/Adhithya")
      assert_equal "/hello/Adhithya", seen_path
    end
  end

  def test_before_filter_fires_for_unmatched_routes_too
    filter_called = false
    app = CodingAdventures::Conduit.app do
      before { filter_called = true }
      get "/exists" do
        "ok"
      end
    end

    with_server(app) do |server|
      http_get(server, "/missing")
      assert filter_called, "before filter should fire for all requests, including 404s"
    end
  end

  def test_before_filter_halt_blocks_unregistered_path
    app = CodingAdventures::Conduit.app do
      before do |request|
        halt(503, "Maintenance") if request.path == "/down"
      end
      get "/" do
        "ok"
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/down")
      assert_equal "503", response.code
      assert_equal "Maintenance", response.body
    end
  end

  # --- After filters ---

  def test_after_filter_fires_after_handler
    order = []
    app = CodingAdventures::Conduit.app do
      after { order << :after }
      get "/" do
        order << :handler
        "ok"
      end
    end

    with_server(app) do |server|
      http_get(server, "/")
      assert_equal [:handler, :after], order
    end
  end

  # --- Custom not-found handler ---

  def test_custom_not_found_returns_custom_body
    app = CodingAdventures::Conduit.app do
      get "/exists" do
        "ok"
      end
      not_found do |request|
        html "<h1>Missing: #{request.path}</h1>", 404
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/gone")
      assert_equal "404", response.code
      assert_includes response.body, "Missing: /gone"
      assert_includes response["content-type"], "text/html"
    end
  end

  # --- Custom error handler ---

  def test_custom_error_handler_fires_on_handler_raise
    app = CodingAdventures::Conduit.app do
      get "/boom" do
        raise "kaboom"
      end
      error do |_request, _err|
        json({ error: "Internal Server Error" }, 500)
      end
    end

    with_server(app) do |server|
      response = http_get(server, "/boom")
      assert_equal "500", response.code
      assert_includes response["content-type"], "application/json"
      parsed = JSON.parse(response.body)
      assert_equal "Internal Server Error", parsed["error"]
    end
  end

  # --- POST with JSON body ---

  def test_request_json_parses_post_body
    app = CodingAdventures::Conduit.app do
      post "/echo" do |request|
        data = request.json
        json(data)
      end
    end

    with_server(app) do |server|
      response = http_post(server, "/echo", '{"greeting":"hello"}',
        "Content-Type" => "application/json")
      assert_equal "200", response.code
      assert_equal({ "greeting" => "hello" }, JSON.parse(response.body))
    end
  end

  # --- Settings ---

  def test_settings_accessible_from_handler
    app = CodingAdventures::Conduit.app do
      set :greeting, "Bonjour"
      get "/greet" do
        # Settings are on the Application instance; blocks are not instance_exec'd
        # on Application, so we reference settings through a closure.
        "ok"
      end
    end

    assert_equal "Bonjour", app.settings[:greeting]
  end
end
