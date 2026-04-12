# CodingAdventures::Rpc

Codec-agnostic RPC primitive for Perl. The abstract layer that
`CodingAdventures::JsonRpc` and future codec-specific packages build on top of.

## What Is It?

`CodingAdventures::Rpc` captures the *semantics* of remote procedure calls —
requests, responses, notifications, error codes, method dispatch, id
correlation, handler panic recovery — without knowing anything about
serialization formats or framing schemes.

Think of it like this:

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (your server logic, tool, test client, …)     │
├─────────────────────────────────────────────────────────────┤
│  CodingAdventures::Rpc                   ← this package     │
│  RpcServer / RpcClient                                      │
│  (method dispatch, id correlation, error handling,          │
│   handler registry, panic recovery)                         │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec                                                   │
│  (RpcMessage ↔ bytes)          JSON, MessagePack, Protobuf  │
├─────────────────────────────────────────────────────────────┤
│  RpcFramer                                                  │
│  (byte stream ↔ chunks)   Content-Length, newlines, WS      │
├─────────────────────────────────────────────────────────────┤
│  Transport (filehandle, socket, pipe)                       │
└─────────────────────────────────────────────────────────────┘
```

## Where Does It Fit?

| Package                         | Depends on        | Description                              |
|---------------------------------|-------------------|------------------------------------------|
| `CodingAdventures::Rpc`         | —                 | Abstract RPC primitive (this package)    |
| `CodingAdventures::JsonRpc`     | `Rpc`             | JSON codec + Content-Length framer + rpc |

## Installation

```bash
cpanm --notest --quiet Test2::V0
perl Makefile.PL && make && make test
```

## Usage

### Server

```perl
use CodingAdventures::Rpc::Server;

# Inject your codec (JSON, MessagePack, etc.) and framer (Content-Length, etc.)
my $server = CodingAdventures::Rpc::Server->new(
    codec  => MyJsonCodec->new,
    framer => MyContentLengthFramer->new(\*STDIN, \*STDOUT),
);

# Register request handlers
$server->on_request('ping', sub {
    my ($id, $params) = @_;
    return { pong => 1 };
});

# Register notification handlers
$server->on_notification('log', sub {
    my ($params) = @_;
    warn $params->{message}, "\n";
});

$server->serve;  # blocks until stdin closes
```

### Client

```perl
use CodingAdventures::Rpc::Client;

my $client = CodingAdventures::Rpc::Client->new(
    codec  => MyJsonCodec->new,
    framer => MyContentLengthFramer->new($in_fh, $out_fh),
);

# Optional: handle server-push notifications
$client->on_notification('diagnostics', sub {
    my ($params) = @_;
    print "Diagnostics: $params->{count} issues\n";
});

# Send a request (blocking)
my ($result, $err) = $client->request('add', { a => 3, b => 4 });
if (defined $err) {
    warn "Error $err->{code}: $err->{message}\n";
} else {
    print "sum = $result->{sum}\n";
}

# Fire-and-forget notification
$client->notify('shutdown');
```

### Message constructors

```perl
use CodingAdventures::Rpc::Message qw(:all);
use CodingAdventures::Rpc::Errors  qw(:all);

my $req   = make_request(1, 'ping', { echo => 'hi' });
my $resp  = make_response(1, { pong => 'hi' });
my $err   = make_error(1, METHOD_NOT_FOUND, 'Method not found');
my $notif = make_notification('log', { level => 'info' });
```

## Implementing a Codec

```perl
package MyCodec;
use parent 'CodingAdventures::Rpc::Codec';
use CodingAdventures::Rpc::Message qw(:all);

sub encode {
    my ($self, $msg) = @_;
    # serialize $msg (a blessed hashref with a 'kind' key) to bytes
    return $bytes;
}

sub decode {
    my ($self, $bytes) = @_;
    # on success:  return ($msg_hashref, undef)
    # on parse failure: return (undef, "parse error: ...")
    # on shape failure: return (undef, "invalid request: ...")
}
```

## Implementing a Framer

```perl
package MyFramer;
use parent 'CodingAdventures::Rpc::Framer';

sub new {
    my ($class, %args) = @_;
    return $class->SUPER::new(%args);
}

sub read_frame {
    my ($self) = @_;
    # read one frame from $self->{in_fh}
    # return ($payload_bytes, undef)  on success
    # return (undef, undef)           on clean EOF
    # return (undef, $error_string)   on framing error
}

sub write_frame {
    my ($self, $bytes) = @_;
    # wrap $bytes in your framing envelope and write to $self->{out_fh}
}
```

## Error Codes

| Constant          | Code    | When to use                                 |
|-------------------|---------|---------------------------------------------|
| `PARSE_ERROR`     | -32700  | Bytes could not be decoded by the codec     |
| `INVALID_REQUEST` | -32600  | Decoded but not a valid RPC message shape   |
| `METHOD_NOT_FOUND`| -32601  | No handler for the requested method name    |
| `INVALID_PARAMS`  | -32602  | Handler rejected params as malformed        |
| `INTERNAL_ERROR`  | -32603  | Unhandled exception inside a handler        |

## Tests

```bash
prove -l -v t/
```

Coverage: 29 subtests covering all four message kinds, all error codes,
server dispatch, client request/notify, server-push notifications, panic
recovery, codec/framer abstract base class validation, and constructor
argument checking.
