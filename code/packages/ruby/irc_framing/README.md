# coding_adventures_irc_framing

Stateful IRC TCP byte-stream to line-frame converter.

## Usage

```ruby
require "coding_adventures/irc_framing"
f = CodingAdventures::IrcFraming::Framer.new
f.feed("NICK alice\r\n")
f.frames  # => ["NICK alice"]
```

## Limits

Lines longer than 510 bytes of content (RFC 1459 / RFC 2812 section 2.3: 512
including CRLF) are silently discarded.  A pending, not yet LF-terminated, line
is capped at 511 bytes (510 plus a CR): past that the partial line is dropped
and input is skipped up to the next LF, so memory stays constant however much a
peer sends without an LF (SEC-011).  No API change.

## Running tests

```
bundle install && bundle exec rake test
```
