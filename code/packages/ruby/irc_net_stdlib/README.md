# coding_adventures_irc_net_stdlib

Ruby stdlib TCP event loop for IRC. Level 3 of the coding-adventures IRC stack.

Thread-per-connection model using `TCPServer`, `Thread`, and `Mutex`.
Provides `StdlibEventLoop` and a `Handler` mixin.

## Usage

```ruby
require "coding_adventures/irc_net_stdlib"

class MyHandler
  include CodingAdventures::IrcNetStdlib::Handler

  def on_connect(conn_id, host)
    puts "Connected: #{conn_id} from #{host}"
  end

  def on_data(conn_id, data)
    puts "Data from #{conn_id}: #{data}"
  end

  def on_disconnect(conn_id)
    puts "Disconnected: #{conn_id}"
  end
end

loop = CodingAdventures::IrcNetStdlib::StdlibEventLoop.new
loop.run("0.0.0.0", 6667, MyHandler.new)
```

## Running tests

```
bundle install && bundle exec rake test
```
