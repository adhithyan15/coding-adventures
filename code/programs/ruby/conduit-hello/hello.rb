# conduit-hello — full Sinatra-style demo built on Conduit.
#
# Exercises every feature of the Phase 3 DSL:
#
#   GET  /                  → HTML greeting
#   GET  /hello/:name       → JSON with name
#   POST /echo              → echoes JSON body
#   GET  /redirect          → 301 to /
#   GET  /halt              → 403 via halt()
#   GET  /down              → 503 via before filter
#   GET  /error             → triggers custom error handler
#   GET  /missing           → custom 404 handler
#
# Run:  ruby hello.rb
# Then: curl http://localhost:3000/hello/Adhithya

$LOAD_PATH.unshift File.expand_path("../../../packages/ruby/conduit/lib", __dir__)
require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  set :app_name, "Conduit Hello"

  # --- Before filter: block /down for maintenance ---
  before do |request|
    halt(503, "Under maintenance") if request.path == "/down"
  end

  # --- After filter: log every request to stdout ---
  after do |request|
    $stdout.puts "[after] #{request.method} #{request.path}"
    $stdout.flush
  end

  # --- Routes ---

  get "/" do
    html "<h1>Hello from Conduit!</h1><p>Try /hello/Adhithya</p>"
  end

  get "/hello/:name" do |request|
    json({ message: "Hello #{request.params.fetch("name")}", app: "Conduit" })
  end

  post "/echo" do |request|
    data = request.json
    json(data)
  end

  get "/redirect" do
    redirect "/", 301
  end

  get "/halt" do
    halt(403, "Forbidden — this route always halts")
  end

  get "/down" do
    # Unreachable: the before filter halts 503 on /down
    "ok"
  end

  get "/error" do
    raise "Intentional error for demo"
  end

  # --- Custom not-found handler ---
  not_found do |request|
    html "<h1>404 Not Found</h1><p>No route for #{request.path}</p>", 404
  end

  # --- Custom error handler ---
  error do |_request, err|
    json({ error: "Internal Server Error", detail: err }, 500)
  end
end

server = CodingAdventures::Conduit::Server.new(app, host: "127.0.0.1", port: 3000)

puts "#{app.settings[:app_name]} listening on http://#{server.host}:#{server.port}"
puts "Routes:"
puts "  GET  /                 → HTML greeting"
puts "  GET  /hello/:name      → JSON response"
puts "  POST /echo             → echo JSON body"
puts "  GET  /redirect         → 301 to /"
puts "  GET  /halt             → 403 Forbidden"
puts "  GET  /down             → 503 (before filter)"
puts "  GET  /error            → 500 via custom error handler"
puts "  GET  /missing          → 404 via custom not_found handler"
puts "Press Ctrl-C to stop."

Signal.trap("INT") { server.stop }
Signal.trap("TERM") { server.stop }

server.serve
