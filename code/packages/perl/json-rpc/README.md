# CodingAdventures::JsonRpc (Perl)

JSON-RPC 2.0 transport library — Content-Length-framed messages over stdin/stdout.

## Overview

JSON-RPC 2.0 is the wire protocol underlying the Language Server Protocol (LSP).
This package implements the transport layer: reading and writing Content-Length-framed
messages, and dispatching them to registered handlers.

Uses only `JSON::PP` from the Perl core library (available since Perl 5.14).
No CPAN runtime dependencies.

## Framing

```
Content-Length: <n>\r\n
\r\n
<UTF-8 JSON payload, exactly n bytes>
```

`Content-Length` is the **byte** length of the UTF-8-encoded JSON body.

## Quick Start

```perl
use CodingAdventures::JsonRpc;

my $server = CodingAdventures::JsonRpc::Server->new(\*STDIN, \*STDOUT);

$server->on_request('initialize', sub {
    my ($id, $params) = @_;
    return { capabilities => { hoverProvider => JSON::PP::true } };
});

$server->on_notification('textDocument/didOpen', sub {
    my ($params) = @_;
    # parse $params->{textDocument}{text} ...
});

$server->serve;    # blocks until stdin closes
```

## API Reference

### Message constructors

```perl
use CodingAdventures::JsonRpc::Message qw(request response error_response notification);

my $req  = request($id, $method, $params);      # params optional
my $resp = response($id, $result);
my $err  = error_response($id, { code => -32601, message => 'Method not found' });
my $notif = notification($method, $params);     # params optional
```

### MessageReader

```perl
my $reader = CodingAdventures::JsonRpc::Reader->new($fh);
my ($msg, $err) = $reader->read_message;
# Returns: ($hashref, undef)  on success
#          (undef, $error)    on error
#          (undef, undef)     on EOF
```

### MessageWriter

```perl
my $writer = CodingAdventures::JsonRpc::Writer->new($fh);
$writer->write_message($hashref);
$writer->write_raw($json_string);
```

### Server

```perl
my $server = CodingAdventures::JsonRpc::Server->new($in_fh, $out_fh);
$server->on_request('method/name', sub { my ($id, $params) = @_; return $result });
$server->on_notification('method/name', sub { my ($params) = @_ });
$server->serve;
```

### Error constants

```perl
use CodingAdventures::JsonRpc::Errors qw(:all);

PARSE_ERROR       # -32700
INVALID_REQUEST   # -32600
METHOD_NOT_FOUND  # -32601
INVALID_PARAMS    # -32602
INTERNAL_ERROR    # -32603
```

## Running Tests

```bash
prove -l -v t/
```

## Module Structure

```
lib/CodingAdventures/
  JsonRpc.pm            — umbrella re-export module
  JsonRpc/
    Errors.pm           — error-code constants
    Message.pm          — message constructors + classify_message
    Reader.pm           — MessageReader class
    Writer.pm           — MessageWriter class
    Server.pm           — Server class
```
