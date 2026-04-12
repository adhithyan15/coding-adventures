# coding_adventures_irc_framing

Stateful IRC TCP byte-stream to line-frame converter.

## Usage

```ruby
require "coding_adventures/irc_framing"
f = CodingAdventures::IrcFraming::Framer.new
f.feed("NICK alice\r\n")
f.frames  # => ["NICK alice"]
```

## Running tests

```
bundle install && bundle exec rake test
```
