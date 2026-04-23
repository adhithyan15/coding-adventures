# Conduit

`Conduit` is a tiny Rack-like Ruby layer backed by the Rust `embeddable-http-server`.

It currently focuses on the smallest useful server shape:

- a callable app contract
- a small route DSL
- path params like `/hello/:name`
- a native server that owns TCP sockets and HTTP/1 request assembly

## Example

```ruby
require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  get "/hello/:name" do |request|
    "Hello #{request.params.fetch("name")}"
  end
end

server = CodingAdventures::Conduit::Server.new(app, port: 9292)
server.start
sleep
```
