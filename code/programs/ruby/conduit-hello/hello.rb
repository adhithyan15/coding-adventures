# conduit-hello — minimal Sinatra-style demo built on Conduit.
#
# Starts an HTTP server on port 3000 and handles two routes:
#
#   GET /              → "Hello from Conduit!"
#   GET /hello/:name   → "Hello <name>"
#
# Run:  ruby hello.rb
# Then: open http://localhost:3000/hello/Adhithya

$LOAD_PATH.unshift File.expand_path("../../../packages/ruby/conduit/lib", __dir__)
require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  get "/" do
    "Hello from Conduit!"
  end

  get "/hello/:name" do |request|
    "Hello #{request.params.fetch("name")}"
  end
end

server = CodingAdventures::Conduit::Server.new(app, host: "127.0.0.1", port: 3000)

puts "Conduit listening on http://#{server.host}:#{server.port}"
puts "Try: http://#{server.host}:#{server.port}/hello/Adhithya"
puts "Press Ctrl-C to stop."

Signal.trap("INT") { server.stop }
Signal.trap("TERM") { server.stop }

server.serve
