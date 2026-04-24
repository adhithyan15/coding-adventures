# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_conduit"
require "net/http"
require "uri"

class ConduitHelloTest < Minitest::Test
  def setup
    app = CodingAdventures::Conduit.app do
      get "/" do
        "Hello from Conduit!"
      end

      get "/hello/:name" do |request|
        "Hello #{request.params.fetch("name")}"
      end
    end

    @server = CodingAdventures::Conduit::Server.new(app, port: 0)
    @server.start
  end

  def teardown
    @server&.close
  end

  def test_root_route_returns_greeting
    response = Net::HTTP.get_response(URI("http://127.0.0.1:#{@server.port}/"))
    assert_equal "200", response.code
    assert_equal "Hello from Conduit!", response.body
  end

  def test_named_param_route_returns_personalised_greeting
    response = Net::HTTP.get_response(URI("http://127.0.0.1:#{@server.port}/hello/Adhithya"))
    assert_equal "200", response.code
    assert_equal "Hello Adhithya", response.body
  end

  def test_unknown_route_returns_404
    response = Net::HTTP.get_response(URI("http://127.0.0.1:#{@server.port}/missing"))
    assert_equal "404", response.code
  end
end
