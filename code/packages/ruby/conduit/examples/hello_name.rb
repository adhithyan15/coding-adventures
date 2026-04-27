# frozen_string_literal: true

$LOAD_PATH.unshift(File.expand_path("../lib", __dir__))

require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  get "/hello/:name" do |request|
    "Hello #{request.params.fetch("name")}"
  end
end

server = CodingAdventures::Conduit::Server.new(app, host: "127.0.0.1", port: 9292)
server.start

puts "Conduit listening at http://127.0.0.1:9292/hello/Adhithya"
puts "Press Ctrl+C to stop."

trap("INT") do
  server.close
  exit(0)
end

sleep
