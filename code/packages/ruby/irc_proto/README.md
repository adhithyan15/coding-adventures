# coding_adventures_irc_proto

Pure IRC message parsing and serialisation. Level 0 of the coding-adventures
IRC stack — no I/O, no state, just a bidirectional codec.

## Usage

```ruby
require "coding_adventures/irc_proto"

msg = CodingAdventures::IrcProto.parse("NICK alice")
# => #<Message prefix=nil command="NICK" params=["alice"]>

CodingAdventures::IrcProto.serialize(msg)
# => "NICK alice"
```

## Running tests

```
bundle install && bundle exec rake test
```
