package CodingAdventures::TcpClient;

# ============================================================================
# CodingAdventures::TcpClient -- TCP Client Connection Library
# ============================================================================
#
# A literate, educational TCP client library that wraps Perl's IO::Socket::INET
# with timeout support, buffered reading, and structured error handling.
#
# ## What is TCP?
#
# TCP (Transmission Control Protocol) provides reliable, ordered delivery of
# a stream of bytes between two programs on a network. Think of it like a
# phone call:
#
#   1. You dial (connect) -- DNS resolves the hostname, then the TCP
#      three-way handshake establishes the connection.
#   2. You talk back and forth (read/write) -- data flows in both
#      directions simultaneously (full-duplex).
#   3. You hang up (close) -- either side can initiate shutdown.
#
# Unlike UDP (which is like sending postcards), TCP guarantees:
#   - Every byte you send arrives at the other end
#   - Bytes arrive in the same order you sent them
#   - Duplicate bytes are discarded
#
# ## Architecture
#
# This module provides three classes:
#
#   ConnectOptions  -- configuration for timeouts and buffer size
#   TcpConnection   -- the connected socket with read/write methods
#   TcpError        -- structured error with a `type` field for matching
#
# And one free function:
#
#   connect($host, $port, $options) -- establish a connection
#
# ## Connection Lifecycle
#
#   1. connect()        -- Open a TCP socket to a remote host:port
#   2. write_all()      -- Send data through the connection
#   3. flush()          -- Ensure buffered data is actually sent
#   4. read_line()      -- Read a line of text (until \n)
#      read_exact()     -- Read exactly N bytes
#      read_until()     -- Read until a specific byte appears
#   5. shutdown_write() -- Signal "I'm done sending" (half-close)
#   6. close()          -- Fully close the connection
#
# ============================================================================

use strict;
use warnings;

# --- Dependencies ---
#
# IO::Socket::INET -- the core TCP socket class in Perl. It handles DNS
#     resolution, socket creation, and the three-way handshake in one call.
#
# IO::Select -- lets us wait for a socket to become readable/writable with
#     a timeout. This wraps the POSIX select() system call, which monitors
#     one or more file descriptors for readiness.
#
# Socket -- provides low-level socket constants. We need SHUT_WR to perform
#     a half-close (shutdown the write direction only).
#
# Errno -- portable error number constants. We use these to classify OS-level
#     socket errors into our structured TcpError types.

use IO::Socket::INET;
use IO::Select;
use Socket qw(SHUT_WR);
use Errno qw(ETIMEDOUT ECONNREFUSED ECONNRESET EPIPE);
use Exporter 'import';

our $VERSION = '0.1.0';

# We export `connect` as a free function so callers can write:
#   use CodingAdventures::TcpClient qw(connect);
#   my $conn = connect('localhost', 8080);

our @EXPORT_OK = qw(connect);
our %EXPORT_TAGS = (all => \@EXPORT_OK);

# ============================================================================
# CodingAdventures::TcpClient::TcpError -- Structured Error Class
# ============================================================================
#
# TCP operations can fail in many ways, and each failure mode requires a
# different response from the caller. Rather than returning opaque error
# strings, we return TcpError objects with a `type` field that callers
# can match on programmatically.
#
# ## Error Types
#
# Each type corresponds to a specific failure scenario:
#
#   dns_resolution_failed -- The hostname could not be resolved to an IP.
#       Cause: typo in hostname, no internet, DNS server unreachable.
#
#   connection_refused -- The server's OS actively rejected the connection.
#       Cause: nothing is listening on that port. The server sent TCP RST.
#
#   timeout -- An operation took longer than the configured timeout.
#       The `phase` field tells you which operation timed out:
#       "connect", "read", or "write".
#
#   connection_reset -- The remote side closed the connection unexpectedly.
#       Cause: server process crashed, was killed, or sent TCP RST.
#
#   broken_pipe -- Tried to write to a connection the remote side closed.
#       Cause: the server hung up and we kept talking.
#
#   unexpected_eof -- The connection closed before we read enough data.
#       Cause: read_exact(100) but only 50 bytes arrived before EOF.
#
#   io_error -- A low-level I/O error not covered by the types above.
#       This is the catch-all for edge cases like permission denied.
#
# ## Usage Pattern
#
#   eval { $conn->read_line() };
#   if (my $err = $@) {
#       if (ref $err && $err->isa('CodingAdventures::TcpClient::TcpError')) {
#           if ($err->type eq 'timeout') {
#               warn "Read timed out, retrying...";
#           } elsif ($err->type eq 'connection_reset') {
#               warn "Server crashed!";
#           }
#       }
#   }
#
# ============================================================================

package CodingAdventures::TcpClient::TcpError;

use strict;
use warnings;

# Overload stringification so that TcpError objects can be used in string
# context (e.g., die $error) and produce a human-readable message.
# Without this, die $error would print something like:
#   CodingAdventures::TcpClient::TcpError=HASH(0x...)
# which is useless for debugging.

use overload '""' => sub { $_[0]->{message} }, fallback => 1;

# -- Constructor --
#
# Creates a new TcpError with the given type and message. Additional
# fields can be passed as key-value pairs and will be stored on the object.
#
# Arguments:
#   type    -- one of the error type strings listed above
#   message -- human-readable description of what went wrong
#   %extra  -- optional additional fields (e.g., host, phase, expected)
#
# Example:
#   my $err = CodingAdventures::TcpClient::TcpError->new(
#       'timeout', 'read timed out after 5 seconds',
#       phase => 'read', duration => 5,
#   );

sub new {
    my ($class, $type, $message, %extra) = @_;
    return bless {
        type    => $type,
        message => $message,
        %extra,
    }, $class;
}

# -- Accessors --
#
# Simple read-only accessors. In Perl, a method is just a subroutine that
# receives the object as its first argument. We access the underlying hash
# fields directly since we control the object structure.

sub type    { return $_[0]->{type} }
sub message { return $_[0]->{message} }

# -- _throw (class method) --
#
# Convenience method to create and immediately die with a TcpError.
# This is the standard way to raise structured errors in this library.
#
# Example:
#   CodingAdventures::TcpClient::TcpError->_throw(
#       'connection_refused', "connection refused by 127.0.0.1:9999"
#   );

sub _throw {
    my ($class, $type, $message, %extra) = @_;
    die $class->new($type, $message, %extra);
}


# ============================================================================
# CodingAdventures::TcpClient::ConnectOptions -- Connection Configuration
# ============================================================================
#
# Encapsulates the four tuneable parameters for a TCP connection:
#
#   connect_timeout (default: 30 seconds)
#       Maximum time to wait for the TCP three-way handshake to complete.
#       If the server is down or behind a firewall that silently drops
#       packets, the OS might wait minutes. This timeout prevents that.
#
#   read_timeout (default: 30 seconds)
#       Maximum time to wait for data when calling read_line, read_exact,
#       or read_until. A well-behaved server responds promptly; a crashed
#       or overloaded server may stall indefinitely. This prevents hangs.
#
#   write_timeout (default: 30 seconds)
#       Maximum time to wait for the OS to accept data on a write.
#       Writes usually complete instantly (data goes to the OS send buffer),
#       but can block if the buffer is full because the remote side stopped
#       reading.
#
#   buffer_size (default: 8192 bytes = 8 KiB)
#       Size of internal read chunks. 8192 is a good balance between memory
#       usage and system call reduction -- it corresponds to roughly 5-6
#       TCP segments (each ~1460 bytes with typical MSS).
#
# ## Builder Pattern
#
# ConnectOptions uses a builder pattern where each setter returns $self,
# allowing chained calls:
#
#   my $opts = CodingAdventures::TcpClient::ConnectOptions->new()
#       ->connect_timeout(10)
#       ->read_timeout(60)
#       ->buffer_size(16384);
#
# ============================================================================

package CodingAdventures::TcpClient::ConnectOptions;

use strict;
use warnings;

# -- Constructor --
#
# Creates a ConnectOptions with sensible defaults. All fields are stored
# in a plain hashref and blessed into this package.

sub new {
    my ($class, %args) = @_;
    return bless {
        connect_timeout => $args{connect_timeout} // 30,
        read_timeout    => $args{read_timeout}    // 30,
        write_timeout   => $args{write_timeout}   // 30,
        buffer_size     => $args{buffer_size}     // 8192,
    }, $class;
}

# -- Getter/Setter Methods --
#
# Each method serves dual purpose:
#   - Called with no argument: returns the current value (getter)
#   - Called with an argument: sets the value and returns $self (setter)
#
# The setter-returns-$self pattern enables method chaining:
#   $opts->connect_timeout(5)->read_timeout(10)
#
# This is a common Perl idiom. In Rust, this would be the builder pattern;
# in Python, it would be keyword arguments to the constructor.

sub connect_timeout {
    my ($self, $val) = @_;
    if (defined $val) { $self->{connect_timeout} = $val; return $self }
    return $self->{connect_timeout};
}

sub read_timeout {
    my ($self, $val) = @_;
    if (defined $val) { $self->{read_timeout} = $val; return $self }
    return $self->{read_timeout};
}

sub write_timeout {
    my ($self, $val) = @_;
    if (defined $val) { $self->{write_timeout} = $val; return $self }
    return $self->{write_timeout};
}

sub buffer_size {
    my ($self, $val) = @_;
    if (defined $val) { $self->{buffer_size} = $val; return $self }
    return $self->{buffer_size};
}


# ============================================================================
# CodingAdventures::TcpClient::TcpConnection -- Connected TCP Socket
# ============================================================================
#
# Represents an active TCP connection with buffered I/O and timeouts.
# This is the main workhorse of the library -- all data transfer happens
# through TcpConnection methods.
#
# ## Internal Structure
#
# A TcpConnection holds:
#   socket        -- the IO::Socket::INET object (the OS file descriptor)
#   read_timeout  -- seconds to wait on reads before timing out
#   write_timeout -- seconds to wait on writes before timing out
#   buffer_size   -- bytes to request per sysread() call
#   host          -- the hostname we connected to (for error messages)
#   port          -- the port we connected to (for error messages)
#   _read_buf     -- internal read buffer (bytes read from socket but not
#                    yet consumed by the caller)
#   _closed       -- flag to track whether close() has been called
#
# ## Why sysread() Instead of Perl's Buffered I/O?
#
# Perl's <> operator and read() function use an internal IO buffer. This
# interacts poorly with IO::Select timeout checking: select() monitors the
# OS socket, not Perl's buffer. If Perl has buffered data, select() may
# report "not readable" even though data is available. This causes deadlocks.
#
# We use sysread() which bypasses Perl's buffer and reads directly from
# the OS. We maintain our own _read_buf, giving us full control over
# buffering and timeout behavior.
#
# ============================================================================

package CodingAdventures::TcpClient::TcpConnection;

use strict;
use warnings;
use IO::Select;
use Socket qw(SHUT_WR);

# -- Constructor (internal) --
#
# Not called directly by users -- use the connect() free function instead.
# This stores the socket and configuration in a blessed hashref.

sub new {
    my ($class, %args) = @_;
    return bless {
        socket        => $args{socket},
        read_timeout  => $args{read_timeout},
        write_timeout => $args{write_timeout},
        buffer_size   => $args{buffer_size},
        host          => $args{host},
        port          => $args{port},
        _read_buf     => '',          # internal read buffer for sysread
        _closed       => 0,
    }, $class;
}

# -- _validate --
#
# Checks that the connection is still open before performing any operation.
# Dies with a TcpError if the connection has been closed. This prevents
# cryptic errors from calling methods on a closed/undef socket.

sub _validate {
    my ($self) = @_;
    if ($self->{_closed} || !defined $self->{socket}) {
        CodingAdventures::TcpClient::TcpError->_throw(
            'io_error', 'TcpClient: connection is closed',
        );
    }
}

# -- _wait_readable --
#
# Uses IO::Select to wait until the socket has data available, or until
# the read timeout expires. IO::Select wraps the POSIX select() syscall.
#
# ## How select() Works
#
# select() takes a set of file descriptors and a timeout. It blocks until:
#   - At least one descriptor is ready for the requested operation, OR
#   - The timeout expires
#
# This is how we implement non-blocking timeouts without threads:
#   1. Ask select() "is this socket readable within N seconds?"
#   2. If yes -> proceed with the read
#   3. If no (timeout) -> throw a TcpError
#
# Returns 1 if the socket is readable. Dies with TcpError on timeout.

sub _wait_readable {
    my ($self) = @_;

    # If we already have data in the internal buffer, no need to wait
    # on the socket -- we can serve from the buffer immediately.
    return 1 if length($self->{_read_buf}) > 0;

    my $sel = IO::Select->new($self->{socket});
    my @ready = $sel->can_read($self->{read_timeout});
    unless (@ready) {
        CodingAdventures::TcpClient::TcpError->_throw(
            'timeout',
            "TcpClient: read timed out after $self->{read_timeout} seconds",
            phase    => 'read',
            duration => $self->{read_timeout},
        );
    }
    return 1;
}

# -- _fill_buffer --
#
# Reads up to buffer_size bytes from the socket into the internal read
# buffer using sysread(). This is the only place where we call sysread().
#
# ## Why sysread()?
#
# Perl's built-in read() uses buffered I/O with an internal buffer that
# conflicts with IO::Select. sysread() bypasses this entirely, reading
# directly from the OS file descriptor.
#
# Returns the number of bytes read, 0 on EOF, or throws TcpError.

sub _fill_buffer {
    my ($self) = @_;

    $self->_wait_readable();

    my $bytes_read = sysread($self->{socket}, my $chunk, $self->{buffer_size});

    if (!defined $bytes_read) {
        # sysread returned undef -- classify the error via errno.
        my $errno = $! + 0;
        if ($errno == Errno::ECONNRESET) {
            CodingAdventures::TcpClient::TcpError->_throw(
                'connection_reset', "TcpClient: connection reset by peer",
            );
        }
        CodingAdventures::TcpClient::TcpError->_throw(
            'io_error', "TcpClient: read error: $!",
        );
    }

    if ($bytes_read == 0) {
        return 0;   # EOF -- peer closed the connection
    }

    $self->{_read_buf} .= $chunk;
    return $bytes_read;
}

# ============================================================================
# read_line() -> $line
# ============================================================================
#
# Reads bytes until a newline (\n, byte 0x0A) is found. Returns the line
# INCLUDING the trailing \n (and \r\n if present).
#
# This is the workhorse for line-oriented protocols like HTTP/1.x, SMTP,
# POP3, and Redis RESP, which frame messages as lines terminated by \r\n.
#
# ## Algorithm
#
# 1. Search the internal buffer for \n using index().
# 2. If found: extract everything up to and including \n, return it.
# 3. If not found: call _fill_buffer() to read more from the socket.
# 4. Repeat until \n is found or EOF.
#
# ## At EOF
#
# If the peer closes the connection and we have leftover data without a
# trailing \n, we return it anyway. If there's no data at all, we throw
# unexpected_eof.
#
# ## Example
#
#   my $status = $conn->read_line();   # "HTTP/1.0 200 OK\r\n"

sub read_line {
    my ($self) = @_;
    $self->_validate();

    while (1) {
        # index() returns the position of \n, or -1 if not found.
        my $nl_pos = index($self->{_read_buf}, "\n");

        if ($nl_pos >= 0) {
            # Found a newline. Extract everything up to and including it.
            # The 4-argument substr removes the extracted portion in place.
            my $line = substr($self->{_read_buf}, 0, $nl_pos + 1, '');
            return $line;
        }

        # No newline yet -- read more data from the socket.
        my $n = $self->_fill_buffer();

        if ($n == 0) {
            # EOF. Return leftover data if any, otherwise error.
            if (length($self->{_read_buf}) > 0) {
                my $remaining = $self->{_read_buf};
                $self->{_read_buf} = '';
                return $remaining;
            }
            CodingAdventures::TcpClient::TcpError->_throw(
                'unexpected_eof',
                "TcpClient: connection closed by peer during read_line",
                expected => 'newline',
                received => 0,
            );
        }
    }
}

# ============================================================================
# read_exact($n) -> $data
# ============================================================================
#
# Reads exactly $n bytes. Blocks until all bytes arrive or an error occurs.
# Essential for binary protocols with length-prefixed messages.
#
# ## Why Multiple Reads May Be Needed
#
# TCP is a byte stream. A single sysread() may return fewer bytes than
# requested -- it depends on how data arrived over the network:
#
#   Requesting 1000 bytes:
#     sysread() -> 500 bytes   (first TCP segment)
#     sysread() -> 400 bytes   (second segment)
#     sysread() -> 100 bytes   (third segment, done!)
#
# We loop until exactly $n bytes have been accumulated.
#
# ## Example
#
#   my $header  = $conn->read_exact(4);
#   my $length  = unpack('N', $header);    # big-endian uint32
#   my $payload = $conn->read_exact($length);

sub read_exact {
    my ($self, $n) = @_;
    $self->_validate();

    CodingAdventures::TcpClient::TcpError->_throw(
        'io_error', "TcpClient: read_exact requires a positive byte count",
    ) unless defined $n && $n > 0;

    # Keep filling the buffer until we have at least $n bytes.
    while (length($self->{_read_buf}) < $n) {
        my $bytes_read = $self->_fill_buffer();

        if ($bytes_read == 0) {
            my $got = length($self->{_read_buf});
            CodingAdventures::TcpClient::TcpError->_throw(
                'unexpected_eof',
                "TcpClient: connection closed after $got of $n bytes",
                expected => $n,
                received => $got,
            );
        }
    }

    # Extract exactly $n bytes from the front of the buffer.
    my $data = substr($self->{_read_buf}, 0, $n, '');
    return $data;
}

# ============================================================================
# read_until($delimiter) -> $data
# ============================================================================
#
# Reads bytes until the specified delimiter byte is found. Returns all
# bytes INCLUDING the delimiter. The delimiter is an integer 0-255.
#
# ## Why an Integer Delimiter?
#
# Using an integer is unambiguous for binary protocols:
#   0          -> null terminator (\0)
#   10         -> newline (\n)
#   ord(';')   -> semicolon
#
# This matches the Rust API where the delimiter is a u8.
#
# ## Algorithm
#
# 1. Convert integer to character via chr().
# 2. Search buffer with index().
# 3. If found, extract and return.
# 4. If not, read more data, repeat.
#
# ## Example
#
#   my $data = $conn->read_until(0);         # null-terminated string
#   my $cmd  = $conn->read_until(ord(';'));   # semicolon-delimited

sub read_until {
    my ($self, $delimiter) = @_;
    $self->_validate();

    CodingAdventures::TcpClient::TcpError->_throw(
        'io_error', "TcpClient: read_until requires a delimiter byte (0-255)",
    ) unless defined $delimiter && $delimiter >= 0 && $delimiter <= 255;

    my $delim_char = chr($delimiter);

    while (1) {
        my $pos = index($self->{_read_buf}, $delim_char);

        if ($pos >= 0) {
            my $data = substr($self->{_read_buf}, 0, $pos + 1, '');
            return $data;
        }

        my $n = $self->_fill_buffer();

        if ($n == 0) {
            CodingAdventures::TcpClient::TcpError->_throw(
                'unexpected_eof',
                "TcpClient: connection closed before delimiter found",
            );
        }
    }
}

# ============================================================================
# write_all($data) -> 1
# ============================================================================
#
# Writes all bytes in $data to the connection. Loops until every byte is
# accepted by the OS. Dies with TcpError on failure.
#
# ## Why We Loop
#
# syswrite() may not send all data at once -- the OS send buffer has a
# limited size (typically 64-256 KiB). If nearly full, syswrite() accepts
# only what fits. We must retry with the remaining bytes.
#
# ## syswrite vs print
#
# We use syswrite() because:
#   1. It reports exactly how many bytes were written (partial writes)
#   2. It bypasses Perl's IO buffer, matching our sysread() approach
#   3. Error detection is immediate
#
# ## Example
#
#   $conn->write_all("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");

sub write_all {
    my ($self, $data) = @_;
    $self->_validate();

    # Empty write is a no-op (matches Rust's write_all(&[]) behavior).
    return 1 unless defined $data && length($data) > 0;

    my $offset    = 0;
    my $remaining = length($data);

    while ($remaining > 0) {
        my $written = syswrite($self->{socket}, $data, $remaining, $offset);

        unless (defined $written) {
            my $errno = $! + 0;
            if ($errno == Errno::EPIPE) {
                CodingAdventures::TcpClient::TcpError->_throw(
                    'broken_pipe',
                    "TcpClient: broken pipe (remote side closed connection)",
                );
            }
            if ($errno == Errno::ECONNRESET) {
                CodingAdventures::TcpClient::TcpError->_throw(
                    'connection_reset',
                    "TcpClient: connection reset by peer during write",
                );
            }
            CodingAdventures::TcpClient::TcpError->_throw(
                'io_error', "TcpClient: write error: $!",
            );
        }

        $offset    += $written;
        $remaining -= $written;
    }

    return 1;
}

# ============================================================================
# flush() -> 1
# ============================================================================
#
# Flushes output to the network. Since we use syswrite() which goes
# directly to the OS, this is mostly a safety call on the socket handle.
#
# ## When to Call
#
# After writing a complete request, before waiting for a response:
#
#   $conn->write_all("GET / HTTP/1.0\r\n\r\n");
#   $conn->flush();
#   my $response = $conn->read_line();

sub flush {
    my ($self) = @_;
    $self->_validate();
    $self->{socket}->flush();
    return 1;
}

# ============================================================================
# shutdown_write() -> 1
# ============================================================================
#
# Half-close: tells the peer "I'm done sending" while keeping reads open.
#
# ## How It Works
#
# TCP is bidirectional (full-duplex). shutdown(SHUT_WR) closes only the
# write direction:
#
#   Before:  Client <=======> Server   (bidirectional)
#   After:   Client <-------- Server   (read-only from client's view)
#
# This sends a TCP FIN packet. The server's next read() returns 0 (EOF).
#
# ## Why It Matters
#
# HTTP/1.0 uses this: client sends request, half-closes, server reads
# until EOF, processes, responds. Without half-close, the server can't
# detect end-of-request.
#
# ## Example
#
#   $conn->write_all($request);
#   $conn->flush();
#   $conn->shutdown_write();          # "I'm done sending"
#   my $response = $conn->read_line();   # still can read!

sub shutdown_write {
    my ($self) = @_;
    $self->_validate();

    $self->{socket}->shutdown(SHUT_WR)
        or CodingAdventures::TcpClient::TcpError->_throw(
            'io_error', "TcpClient: shutdown_write failed: $!",
        );

    return 1;
}

# ============================================================================
# peer_addr() -> $string
# ============================================================================
#
# Returns the remote address as "host:port". Useful for logging which IP
# you connected to when a hostname resolves to multiple addresses.
#
# ## Example
#
#   print $conn->peer_addr();   # "127.0.0.1:8080"

sub peer_addr {
    my ($self) = @_;
    $self->_validate();
    my $s = $self->{socket};
    return $s->peerhost() . ':' . $s->peerport();
}

# ============================================================================
# local_addr() -> $string
# ============================================================================
#
# Returns the local address as "host:port". The OS assigns an ephemeral
# port (typically 49152-65535) when connecting.
#
# ## Example
#
#   print $conn->local_addr();   # "127.0.0.1:54321"

sub local_addr {
    my ($self) = @_;
    $self->_validate();
    my $s = $self->{socket};
    return $s->sockhost() . ':' . $s->sockport();
}

# ============================================================================
# close() -> 1
# ============================================================================
#
# Fully closes the TCP connection. After this, no reads or writes are
# possible. Calling close() on an already-closed connection is a safe
# no-op.
#
# ## TCP Teardown
#
#   Client              Server
#     |  --- FIN ----->   |    "I'm done"
#     |  <-- ACK ------   |    "Got it"
#     |  <-- FIN ------   |    "I'm done too"
#     |  --- ACK ----->   |    "Got it, goodbye"

sub close {
    my ($self) = @_;
    return 1 if $self->{_closed};

    if (defined $self->{socket}) {
        $self->{socket}->close();
    }

    $self->{socket}  = undef;
    $self->{_closed} = 1;
    return 1;
}

# -- Destructor --
#
# Perl calls DESTROY when an object goes out of scope (reference count
# hits zero). We close the socket here to prevent resource leaks.
# This is the Perl equivalent of Rust's Drop trait.

sub DESTROY {
    my ($self) = @_;
    $self->close() unless $self->{_closed};
}


# ============================================================================
# Back to the main package for the free connect() function
# ============================================================================

package CodingAdventures::TcpClient;

# ============================================================================
# connect($host, $port, $options) -> TcpConnection
# ============================================================================
#
# Establishes a TCP connection to the given host and port. This is the
# primary entry point to the library.
#
# ## Steps
#
# 1. Validate inputs (host non-empty, port 1-65535)
# 2. Create IO::Socket::INET which handles:
#    a. DNS resolution: "example.com" -> 93.184.216.34
#    b. Socket creation: OS allocates file descriptor
#    c. TCP handshake: SYN -> SYN-ACK -> ACK
# 3. Configure socket (autoflush)
# 4. Wrap in TcpConnection object
#
# ## Error Classification
#
# IO::Socket::INET doesn't give structured errors, so we pattern-match
# on the error message to classify failures:
#
#   "Connection refused"              -> connection_refused
#   "timeout" / "timed out"           -> timeout
#   "Name or service not known"       -> dns_resolution_failed
#   anything else                     -> io_error
#
# ## Arguments
#
#   $host    -- hostname ("example.com") or IP ("127.0.0.1")
#   $port    -- TCP port (1-65535)
#   $options -- ConnectOptions object, hashref, or undef for defaults
#
# ## Example
#
#   use CodingAdventures::TcpClient qw(connect);
#
#   my $conn = connect('example.com', 80);
#
#   my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
#       connect_timeout => 5, read_timeout => 60,
#   );
#   my $conn = connect('example.com', 80, $opts);

sub connect {
    my ($host, $port, $options) = @_;

    # Use defaults if no options provided.
    $options //= CodingAdventures::TcpClient::ConnectOptions->new();

    # Accept a plain hashref for convenience.
    if (ref $options eq 'HASH') {
        $options = CodingAdventures::TcpClient::ConnectOptions->new(%$options);
    }

    # -- Input validation --

    CodingAdventures::TcpClient::TcpError->_throw(
        'io_error', 'TcpClient: host is required',
    ) unless defined $host && length $host;

    CodingAdventures::TcpClient::TcpError->_throw(
        'io_error', 'TcpClient: port is required',
    ) unless defined $port;

    CodingAdventures::TcpClient::TcpError->_throw(
        'io_error', "TcpClient: port must be 1-65535 (got '$port')",
    ) unless $port =~ /^\d+$/ && $port > 0 && $port <= 65535;

    # -- Create the TCP socket --
    #
    # IO::Socket::INET->new() resolves DNS, creates the socket, and
    # performs the three-way handshake. Timeout limits the total time.

    my $socket = IO::Socket::INET->new(
        PeerAddr => $host,
        PeerPort => $port,
        Proto    => 'tcp',
        Timeout  => $options->connect_timeout(),
    );

    unless ($socket) {
        my $err = $@ || $!;
        my $err_str = "$err";

        # DNS failure patterns vary by OS:
        #   Linux:   "Name or service not known"
        #   macOS:   "nodename nor servname provided"
        #   Windows: "No such host is known" / "getaddrinfo failed"
        if ($err_str =~ /(?:Name or service not known|No such host|nodename|getaddrinfo|resolve)/i) {
            CodingAdventures::TcpClient::TcpError->_throw(
                'dns_resolution_failed',
                "TcpClient: DNS resolution failed for '$host': $err",
                host => $host,
            );
        }

        if ($err_str =~ /refused/i) {
            CodingAdventures::TcpClient::TcpError->_throw(
                'connection_refused',
                "TcpClient: connection refused by $host:$port",
                addr => "$host:$port",
            );
        }

        if ($err_str =~ /timed?\s*out/i) {
            CodingAdventures::TcpClient::TcpError->_throw(
                'timeout',
                "TcpClient: connect to $host:$port timed out after " .
                    $options->connect_timeout() . " seconds",
                phase    => 'connect',
                duration => $options->connect_timeout(),
            );
        }

        CodingAdventures::TcpClient::TcpError->_throw(
            'io_error',
            "TcpClient: failed to connect to $host:$port: $err",
        );
    }

    # autoflush(1) sends data immediately rather than batching.
    $socket->autoflush(1);

    return CodingAdventures::TcpClient::TcpConnection->new(
        socket        => $socket,
        read_timeout  => $options->read_timeout(),
        write_timeout => $options->write_timeout(),
        buffer_size   => $options->buffer_size(),
        host          => $host,
        port          => $port,
    );
}

1;

__END__

=head1 NAME

CodingAdventures::TcpClient - Educational TCP client library with buffered I/O

=head1 SYNOPSIS

    use CodingAdventures::TcpClient qw(connect);

    # Connect with default options (30s timeouts, 8K buffer)
    my $conn = connect('example.com', 80);

    # Or with custom options
    my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
        connect_timeout => 5,
        read_timeout    => 60,
    );
    my $conn = connect('example.com', 80, $opts);

    # Send a request
    $conn->write_all("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
    $conn->flush();

    # Read the response line by line
    my $status = $conn->read_line();
    print "Status: $status";

    # Clean up
    $conn->close();

=head1 DESCRIPTION

A clean, literate TCP client library for educational use. Wraps IO::Socket::INET
with configurable timeouts, buffered reading via sysread(), and structured error
handling via TcpError objects.

=head1 CLASSES

=head2 CodingAdventures::TcpClient::ConnectOptions

Configuration object with connect_timeout, read_timeout, write_timeout, buffer_size.

=head2 CodingAdventures::TcpClient::TcpConnection

Active TCP connection with read_line, read_exact, read_until, write_all, flush,
shutdown_write, peer_addr, local_addr, close methods.

=head2 CodingAdventures::TcpClient::TcpError

Structured error with type() and message() accessors. Types: dns_resolution_failed,
connection_refused, timeout, connection_reset, broken_pipe, unexpected_eof, io_error.

=cut
