# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_conduit"
require "net/http"
require "uri"
require "json"

class ConduitHelloTest < Minitest::Test
  def setup
    @app = CodingAdventures::Conduit.app do
      set :app_name, "Conduit Hello"

      before do |request|
        halt(503, "Under maintenance") if request.path == "/down"
      end

      get "/" do
        html "<h1>Hello from Conduit!</h1>"
      end

      get "/hello/:name" do |request|
        json({ message: "Hello #{request.params.fetch("name")}" })
      end

      post "/echo" do |request|
        json(request.json)
      end

      get "/redirect" do
        redirect "/", 301
      end

      get "/halt" do
        halt(403, "Forbidden")
      end

      get "/error" do
        raise "Intentional error"
      end

      not_found do |request|
        html "<h1>Not Found: #{request.path}</h1>", 404
      end

      error do |_request, _err|
        json({ error: "Internal Server Error" }, 500)
      end
    end

    @server = CodingAdventures::Conduit::Server.new(@app, port: 0)
    @server.start
  end

  def teardown
    @server&.close
  end

  def get(path)
    Net::HTTP.get_response(URI("http://127.0.0.1:#{@server.port}#{path}"))
  end

  def post_json(path, body)
    uri = URI("http://127.0.0.1:#{@server.port}#{path}")
    Net::HTTP.start(uri.host, uri.port) do |http|
      req = Net::HTTP::Post.new(uri.path)
      req["Content-Type"] = "application/json"
      req.body = JSON.generate(body)
      http.request(req)
    end
  end

  def test_root_returns_html
    response = get("/")
    assert_equal "200", response.code
    assert_includes response["content-type"], "text/html"
    assert_includes response.body, "Hello from Conduit"
  end

  def test_named_param_returns_json_greeting
    response = get("/hello/Adhithya")
    assert_equal "200", response.code
    assert_includes response["content-type"], "application/json"
    assert_equal "Hello Adhithya", JSON.parse(response.body)["message"]
  end

  def test_echo_returns_posted_json_body
    response = post_json("/echo", { name: "Conduit" })
    assert_equal "200", response.code
    assert_equal "Conduit", JSON.parse(response.body)["name"]
  end

  def test_redirect_returns_301_with_location
    uri = URI("http://127.0.0.1:#{@server.port}/redirect")
    response = Net::HTTP.start(uri.host, uri.port) do |http|
      http.request(Net::HTTP::Get.new(uri.path))
    end
    assert_equal "301", response.code
    assert_equal "/", response["location"]
  end

  def test_halt_returns_403
    response = get("/halt")
    assert_equal "403", response.code
    assert_equal "Forbidden", response.body
  end

  def test_before_filter_blocks_down_path_with_503
    response = get("/down")
    assert_equal "503", response.code
    assert_equal "Under maintenance", response.body
  end

  def test_custom_not_found_returns_html_404
    response = get("/missing")
    assert_equal "404", response.code
    assert_includes response["content-type"], "text/html"
    assert_includes response.body, "Not Found"
  end

  def test_custom_error_handler_returns_json_500
    response = get("/error")
    assert_equal "500", response.code
    assert_includes response["content-type"], "application/json"
    assert_equal "Internal Server Error", JSON.parse(response.body)["error"]
  end

  def test_settings_stores_app_name
    assert_equal "Conduit Hello", @app.settings[:app_name]
  end
end
