# ircd (Ruby)

Ruby IRC server program. Wires together the 4 IRC packages into a runnable
server.

## Usage

```
bundle install
bundle exec ruby -Ilib bin/ircd --port 6667 --server-name irc.example.com
```

## Options

```
--host HOST           Bind address (default: 0.0.0.0)
--port PORT           TCP port (default: 6667)
--server-name NAME    Server hostname (default: irc.local)
--motd LINE           MOTD line (default: Welcome.)
--oper-password PASS  OPER password
```

## Running tests

```
bundle install && bundle exec rake test
```
