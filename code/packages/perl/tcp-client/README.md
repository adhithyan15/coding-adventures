# CodingAdventures::TcpClient

A literate, educational TCP client library for Perl. Part of the [coding-adventures](https://github.com/coding-adventures/coding-adventures) project.

## What It Does

Provides an OOP API for TCP client connections with:

- **ConnectOptions** -- configurable timeouts and buffer size with builder pattern
- **TcpConnection** -- buffered reading (`read_line`, `read_exact`, `read_until`), writing (`write_all`, `flush`), half-close (`shutdown_write`), address inspection (`peer_addr`, `local_addr`)
- **TcpError** -- structured errors with `type()` field for programmatic matching
- **Timeout support** on connect, read, and write operations
- **Literate source** -- extensive inline documentation explaining TCP concepts

## Where It Fits

```
url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                          |
                     raw byte stream
```

This is a port of the Rust `tcp-client` crate into idiomatic Perl. It uses `IO::Socket::INET` for socket operations, `IO::Select` for read timeouts, and `sysread()` for buffer-safe reading.

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::TcpClient qw(connect);

# Connect with default options (30s timeouts, 8K buffer)
my $conn = connect('example.com', 80);

# Or with custom options
my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
    connect_timeout => 5,
    read_timeout    => 60,
);
my $conn = connect('example.com', 80, $opts);

# Builder pattern also works
my $opts = CodingAdventures::TcpClient::ConnectOptions->new()
    ->connect_timeout(5)
    ->read_timeout(60);

# Send an HTTP request
$conn->write_all("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
$conn->flush();

# Read the status line
my $status = $conn->read_line();
print "Status: $status";

# Read exactly 100 bytes
my $data = $conn->read_exact(100);

# Read until null byte
my $msg = $conn->read_until(0);

# Inspect addresses
print "Connected to: " . $conn->peer_addr() . "\n";
print "Local address: " . $conn->local_addr() . "\n";

# Half-close (signal "I'm done sending")
$conn->shutdown_write();

# Clean up
$conn->close();
```

## Error Handling

```perl
use CodingAdventures::TcpClient qw(connect);

eval {
    my $conn = connect('example.com', 80);
    $conn->write_all("GET / HTTP/1.0\r\n\r\n");
    my $line = $conn->read_line();
};
if (my $err = $@) {
    if (ref $err && $err->isa('CodingAdventures::TcpClient::TcpError')) {
        if ($err->type eq 'timeout') {
            warn "Timed out: " . $err->message;
        } elsif ($err->type eq 'connection_refused') {
            warn "Server not listening";
        } elsif ($err->type eq 'dns_resolution_failed') {
            warn "Bad hostname";
        }
    } else {
        die $err;    # re-throw non-TCP errors
    }
}
```

## API Reference

### Free Functions

| Function | Description |
|---|---|
| `connect($host, $port, $opts)` | Open a TCP connection, returns TcpConnection |

### ConnectOptions

| Method | Default | Description |
|---|---|---|
| `connect_timeout($val)` | 30 | Seconds to wait for TCP handshake |
| `read_timeout($val)` | 30 | Seconds to wait for data on reads |
| `write_timeout($val)` | 30 | Seconds to wait for writes |
| `buffer_size($val)` | 8192 | Bytes per sysread() call |

All setters return `$self` for chaining. Call without arguments to get current value.

### TcpConnection

| Method | Description |
|---|---|
| `read_line()` | Read until `\n`, returns line including `\n` |
| `read_exact($n)` | Read exactly N bytes |
| `read_until($byte)` | Read until delimiter byte (integer 0-255) |
| `write_all($data)` | Write all bytes via syswrite loop |
| `flush()` | Flush output buffer |
| `shutdown_write()` | Half-close (SHUT_WR) |
| `peer_addr()` | Remote address as "host:port" |
| `local_addr()` | Local address as "host:port" |
| `close()` | Close connection (safe to call twice) |

### TcpError Types

| Type | Meaning |
|---|---|
| `dns_resolution_failed` | Hostname could not be resolved |
| `connection_refused` | Server not listening on that port |
| `timeout` | Connect, read, or write timed out |
| `connection_reset` | Remote side crashed (TCP RST) |
| `broken_pipe` | Wrote to a closed connection |
| `unexpected_eof` | Connection closed before expected data arrived |
| `io_error` | Catch-all for other I/O errors |

## Running Tests

```bash
prove -l -v t/
# or
perl -Ilib t/01-basic.t
```

## Implementation Notes

- Uses `sysread()` instead of Perl's buffered `read()` to avoid conflicts between Perl's internal IO buffer and `IO::Select` timeout checking
- Maintains its own `_read_buf` for byte-level control over buffering
- Error classification parses `IO::Socket::INET` error strings since it doesn't provide structured errors
- DESTROY method ensures sockets are closed when objects go out of scope
