# event-loop (Ruby)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch (the
GPUI framework that powers Zed uses `winit`'s event loop, which has the same
shape as this library).

## Usage

```ruby
require "coding_adventures_event_loop"

class TickSource
  def initialize
    @count = 0
  end

  def poll
    @count += 1
    @count <= 3 ? [:tick] : [:quit]
  end
end

loop_ = CodingAdventures::EventLoop::Loop.new
loop_.add_source(TickSource.new)
loop_.on_event do |e|
  if e == :quit
    CodingAdventures::EventLoop::ControlFlow::EXIT
  else
    puts "tick!"
    CodingAdventures::EventLoop::ControlFlow::CONTINUE
  end
end
loop_.run
```

## API

| Item | Description |
|---|---|
| `ControlFlow::CONTINUE` | Symbol `:continue` — keep the loop running |
| `ControlFlow::EXIT` | Symbol `:exit` — stop the loop |
| `Loop.new` | Create a new empty loop |
| `loop.add_source(s)` | Register an event source (duck-typed: must respond to `poll`) |
| `loop.on_event { \|e\| }` | Register a handler block returning a `ControlFlow` constant |
| `loop.run` | Start the loop (blocks until exit) |
| `loop.stop` | Signal exit from within a handler or from outside |

`add_source` and `on_event` both return `self` for method chaining.

## Design notes

- **Duck-typed sources**: any object that responds to `poll` works — no inheritance required.
- **Block-based handlers**: Ruby's natural handler idiom; Procs/lambdas also accepted.
- **Pull-based**: `poll` is called by the loop; sources must never block.
- **CPU-friendly idle**: `Thread.pass` yields the scheduler when the queue is empty.
- **Chainable API**: `add_source` and `on_event` return `self` for fluent configuration.

## Development

```bash
cd code/packages/ruby/event_loop
bundle install
bundle exec ruby -Ilib -Itest test/test_event_loop.rb
```
