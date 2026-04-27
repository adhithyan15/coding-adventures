#!/usr/bin/perl
# ============================================================================
# t/01-basic.t -- Comprehensive tests for CodingAdventures::TcpClient
# ============================================================================
#
# These tests verify every aspect of the TCP client library:
#
#   1. ConnectOptions defaults and builder pattern
#   2. Echo server: write/read roundtrip
#   3. read_line: line-oriented reading
#   4. read_exact: fixed-size binary reads
#   5. read_until: delimiter-terminated reads
#   6. Timeouts: connect timeout, read timeout
#   7. Errors: connection refused, DNS failure
#   8. Half-close: shutdown_write
#   9. Edge cases: EOF, peer_addr, local_addr, close
#
# ## Test Server Pattern
#
# Most tests spin up a local TCP server using IO::Socket::INET with
# Listen => 1 on port 0 (the OS picks a free port). A child process
# (fork) handles the server side. The test connects as a client and
# verifies behavior.
#
# ## Why fork()?
#
# TCP requires two endpoints. We can't test a client without a server.
# fork() creates a child process to act as the server while the parent
# acts as the client. This is simpler than threads for socket testing
# because each process gets its own file descriptor table.
#
# On Windows, fork() is emulated by Perl using threads (pseudo-fork).
# It works for our purposes but may behave slightly differently than
# real Unix fork().
#
# ============================================================================

use strict;
use warnings;
use Test::More;
use IO::Socket::INET;
use IO::Select;

# Load our library
use lib 'lib';
use CodingAdventures::TcpClient qw(connect);

# ============================================================================
# Helper: start a TCP server on a random port
# ============================================================================
#
# Creates a listening socket on 127.0.0.1 with a random port (port 0 tells
# the OS to pick one). Returns the listener socket. The caller is responsible
# for accepting connections.
#
# Arguments:
#   None
#
# Returns:
#   ($listener, $port) -- the listener socket and the port it's bound to

sub start_listener {
    my $listener = IO::Socket::INET->new(
        LocalAddr => '127.0.0.1',
        LocalPort => 0,           # OS picks a free port
        Proto     => 'tcp',
        Listen    => 5,           # backlog queue size
        ReuseAddr => 1,
    ) or die "Cannot create listener: $!";

    my $port = $listener->sockport();
    return ($listener, $port);
}

# ============================================================================
# Helper: fork an echo server
# ============================================================================
#
# Starts a child process that accepts one connection, echoes back everything
# it receives, then closes. The parent process acts as the TCP client.
#
# Arguments:
#   $listener -- a listening socket from start_listener()
#
# Returns:
#   $pid -- the child process ID (parent should waitpid on it)
#
# The echo server reads data in a loop and writes it back byte-for-byte.
# This is the simplest possible server for testing read/write roundtrips.

sub fork_echo_server {
    my ($listener) = @_;

    my $pid = fork();
    die "fork failed: $!" unless defined $pid;

    if ($pid == 0) {
        # Child process: accept one connection, echo data, then exit.
        my $client = $listener->accept();
        $listener->close();

        if ($client) {
            while (my $bytes = sysread($client, my $buf, 4096)) {
                syswrite($client, $buf, $bytes);
            }
            $client->close();
        }
        exit(0);
    }

    # Parent: close our copy of the listener (child owns it now for accept)
    # Actually, we keep it open so the child can accept -- close after connect.
    return $pid;
}

# ============================================================================
# Test 1: ConnectOptions defaults
# ============================================================================
#
# Verify that ConnectOptions starts with the documented default values:
#   connect_timeout = 30, read_timeout = 30, write_timeout = 30,
#   buffer_size = 8192.

subtest 'ConnectOptions defaults' => sub {
    my $opts = CodingAdventures::TcpClient::ConnectOptions->new();
    is($opts->connect_timeout(), 30,   'default connect_timeout is 30');
    is($opts->read_timeout(),    30,   'default read_timeout is 30');
    is($opts->write_timeout(),   30,   'default write_timeout is 30');
    is($opts->buffer_size(),     8192, 'default buffer_size is 8192');
};

# ============================================================================
# Test 2: ConnectOptions builder pattern
# ============================================================================
#
# Verify that setters return $self for chaining, and that custom values
# override the defaults.

subtest 'ConnectOptions builder pattern' => sub {
    my $opts = CodingAdventures::TcpClient::ConnectOptions->new()
        ->connect_timeout(5)
        ->read_timeout(10)
        ->write_timeout(15)
        ->buffer_size(4096);

    is($opts->connect_timeout(), 5,    'custom connect_timeout');
    is($opts->read_timeout(),    10,   'custom read_timeout');
    is($opts->write_timeout(),   15,   'custom write_timeout');
    is($opts->buffer_size(),     4096, 'custom buffer_size');
};

# ============================================================================
# Test 3: ConnectOptions constructor with arguments
# ============================================================================

subtest 'ConnectOptions constructor args' => sub {
    my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
        connect_timeout => 7,
        buffer_size     => 1024,
    );
    is($opts->connect_timeout(), 7,    'constructor connect_timeout');
    is($opts->read_timeout(),    30,   'constructor read_timeout default');
    is($opts->buffer_size(),     1024, 'constructor buffer_size');
};

# ============================================================================
# Test 4: TcpError structure
# ============================================================================
#
# Verify that TcpError objects have the correct type, message, and
# stringify properly.

subtest 'TcpError structure' => sub {
    my $err = CodingAdventures::TcpClient::TcpError->new(
        'timeout', 'read timed out after 5 seconds',
        phase => 'read', duration => 5,
    );

    is($err->type(),    'timeout',                        'error type');
    is($err->message(), 'read timed out after 5 seconds', 'error message');
    is("$err",          'read timed out after 5 seconds', 'stringification');
    isa_ok($err, 'CodingAdventures::TcpClient::TcpError');
};

# ============================================================================
# Test 5: Echo server -- write and read_line
# ============================================================================
#
# The most fundamental test: connect to a local echo server, send a line,
# and verify that read_line() returns the same line.

subtest 'echo server - write_all and read_line' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    # Give the child a moment to call accept()
    my $conn = connect('127.0.0.1', $port);
    $listener->close();    # parent doesn't need the listener anymore

    # Send a line and read it back
    $conn->write_all("Hello, world!\n");
    my $line = $conn->read_line();
    is($line, "Hello, world!\n", 'echo server returned our line');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 6: read_line with \r\n (CRLF)
# ============================================================================
#
# HTTP and many other protocols use \r\n as line terminator. Verify that
# read_line() returns the full line including \r\n.

subtest 'read_line with CRLF' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("HTTP/1.0 200 OK\r\n");
    my $line = $conn->read_line();
    is($line, "HTTP/1.0 200 OK\r\n", 'read_line preserves CRLF');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 7: Multiple read_line calls
# ============================================================================
#
# Send multiple lines, verify each is read separately.

subtest 'multiple read_line calls' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("line one\nline two\nline three\n");

    is($conn->read_line(), "line one\n",   'first line');
    is($conn->read_line(), "line two\n",   'second line');
    is($conn->read_line(), "line three\n", 'third line');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 8: read_exact
# ============================================================================
#
# Send exactly N bytes, verify read_exact(N) returns them all.

subtest 'read_exact' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    my $data = "ABCDEFGHIJ";    # exactly 10 bytes
    $conn->write_all($data);

    my $result = $conn->read_exact(10);
    is($result, $data, 'read_exact returned all 10 bytes');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 9: read_exact partial -- read in two chunks
# ============================================================================
#
# Send 20 bytes, read as two 10-byte chunks.

subtest 'read_exact in chunks' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("12345678901234567890");

    is($conn->read_exact(10), "1234567890", 'first 10 bytes');
    is($conn->read_exact(10), "1234567890", 'second 10 bytes');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 10: read_until with null delimiter
# ============================================================================
#
# Send a null-terminated string, verify read_until(0) returns it.

subtest 'read_until null delimiter' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("hello\0world\0");

    my $first = $conn->read_until(0);    # delimiter = null byte
    is($first, "hello\0", 'read_until null: first segment');

    my $second = $conn->read_until(0);
    is($second, "world\0", 'read_until null: second segment');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 11: read_until with semicolon delimiter
# ============================================================================

subtest 'read_until semicolon' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("key:value;next:data;");

    my $first = $conn->read_until(ord(';'));
    is($first, "key:value;", 'read_until semicolon: first segment');

    my $second = $conn->read_until(ord(';'));
    is($second, "next:data;", 'read_until semicolon: second segment');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 12: peer_addr and local_addr
# ============================================================================
#
# After connecting, verify that peer_addr() returns the server's address
# and local_addr() returns our local address.

subtest 'peer_addr and local_addr' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    my $peer = $conn->peer_addr();
    like($peer, qr/^127\.0\.0\.1:\d+$/, 'peer_addr format is host:port');
    like($peer, qr/:$port$/,            'peer_addr has correct port');

    my $local = $conn->local_addr();
    like($local, qr/^127\.0\.0\.1:\d+$/, 'local_addr format is host:port');

    # Local and peer ports should be different
    my ($local_port) = $local =~ /:(\d+)$/;
    isnt($local_port, $port, 'local port differs from peer port');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 13: Connection refused
# ============================================================================
#
# Try to connect to a port where nothing is listening. On most systems,
# connecting to 127.0.0.1 on a random high port will be refused immediately.
#
# We bind a listener, get the port, then close the listener so the port is
# definitely not listening.

subtest 'connection refused' => sub {
    # Bind to get a port, then close immediately
    my $tmp = IO::Socket::INET->new(
        LocalAddr => '127.0.0.1',
        LocalPort => 0,
        Proto     => 'tcp',
        Listen    => 1,
    ) or die "Cannot create temp listener: $!";
    my $port = $tmp->sockport();
    $tmp->close();

    eval { connect('127.0.0.1', $port) };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'connection refused throws TcpError');
    like($err->type(), qr/^(connection_refused|io_error)$/,
         'error type is connection_refused or io_error');
};

# ============================================================================
# Test 14: DNS resolution failure
# ============================================================================
#
# Try to connect to a hostname that does not exist. The DNS resolver
# should fail, producing a dns_resolution_failed error.

subtest 'DNS resolution failure' => sub {
    eval {
        connect('this.host.definitely.does.not.exist.example.invalid', 80);
    };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'DNS failure throws TcpError');
    like($err->type(), qr/^(dns_resolution_failed|io_error)$/,
         'error type is dns_resolution_failed or io_error');
};

# ============================================================================
# Test 15: Read timeout
# ============================================================================
#
# Connect to a server that accepts but never sends data. The client's
# read_line() should time out.

subtest 'read timeout' => sub {
    my ($listener, $port) = start_listener();

    # Fork a server that accepts but never sends
    my $pid = fork();
    die "fork failed" unless defined $pid;
    if ($pid == 0) {
        my $client = $listener->accept();
        $listener->close();
        # Just sit here and never send anything.
        # Sleep long enough for the test to complete.
        sleep(10);
        $client->close() if $client;
        exit(0);
    }

    my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
        read_timeout => 1,    # 1 second timeout for fast test
    );
    my $conn = connect('127.0.0.1', $port, $opts);
    $listener->close();

    eval { $conn->read_line() };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'read timeout throws TcpError');
    is($err->type(), 'timeout', 'error type is timeout');
    like($err->message(), qr/timed out/, 'message mentions timeout');

    $conn->close();

    # Clean up child
    kill('TERM', $pid);
    waitpid($pid, 0);
};

# ============================================================================
# Test 16: Connect timeout (non-routable address)
# ============================================================================
#
# 10.255.255.1 is a non-routable address that silently drops packets.
# Connecting to it with a short timeout should produce a timeout error.
#
# This test uses a 2-second timeout to keep it fast.

subtest 'connect timeout' => sub {
    my $opts = CodingAdventures::TcpClient::ConnectOptions->new(
        connect_timeout => 2,
    );

    eval { connect('10.255.255.1', 1, $opts) };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'connect timeout throws TcpError');
    like($err->type(), qr/^(timeout|io_error)$/,
         'error type is timeout or io_error');
};

# ============================================================================
# Test 17: Shutdown write (half-close)
# ============================================================================
#
# Client sends data, calls shutdown_write(), and then reads the server's
# response. The server detects EOF (from the half-close) and sends back
# a confirmation.

subtest 'shutdown_write half-close' => sub {
    my ($listener, $port) = start_listener();

    my $pid = fork();
    die "fork failed" unless defined $pid;
    if ($pid == 0) {
        # Server: read until EOF, then send confirmation
        my $client = $listener->accept();
        $listener->close();
        if ($client) {
            my $data = '';
            while (my $n = sysread($client, my $buf, 4096)) {
                $data .= $buf;
            }
            # Client half-closed, we got EOF. Send back what we received.
            syswrite($client, "GOT:$data");
            $client->close();
        }
        exit(0);
    }

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("hello");
    $conn->shutdown_write();    # "I'm done sending"

    # We can still read the server's response after half-close
    my $response = $conn->read_exact(9);    # "GOT:hello" = 9 bytes
    is($response, "GOT:hello", 'read after shutdown_write works');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 18: Unexpected EOF on read_exact
# ============================================================================
#
# Server sends 5 bytes then closes. Client tries to read_exact(10).
# Should get an unexpected_eof error.

subtest 'unexpected EOF on read_exact' => sub {
    my ($listener, $port) = start_listener();

    my $pid = fork();
    die "fork failed" unless defined $pid;
    if ($pid == 0) {
        my $client = $listener->accept();
        $listener->close();
        if ($client) {
            syswrite($client, "SHORT");    # only 5 bytes
            $client->close();              # close immediately
        }
        exit(0);
    }

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    # Give server time to send and close
    select(undef, undef, undef, 0.1);

    eval { $conn->read_exact(10) };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'unexpected EOF throws TcpError');
    is($err->type(), 'unexpected_eof', 'error type is unexpected_eof');
    like($err->message(), qr/5 of 10/, 'message shows received vs expected');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 19: Close and use-after-close
# ============================================================================
#
# After close(), any operation should throw an error.

subtest 'close and use-after-close' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    ok($conn->close(), 'close() returns true');

    # Double-close should be a safe no-op
    ok($conn->close(), 'double close is a no-op');

    # Operations on closed connection should throw
    eval { $conn->write_all("test") };
    my $err = $@;
    ok(ref $err && $err->isa('CodingAdventures::TcpClient::TcpError'),
       'write_all on closed connection throws');
    is($err->type(), 'io_error', 'closed connection error type is io_error');

    eval { $conn->read_line() };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'read_line on closed connection throws');

    waitpid($pid, 0);
};

# ============================================================================
# Test 20: Empty write is a no-op
# ============================================================================
#
# write_all("") should succeed without error (no bytes to send).

subtest 'empty write is no-op' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    ok($conn->write_all(""), 'empty write_all succeeds');
    ok($conn->write_all(undef), 'undef write_all succeeds');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 21: flush() works without error
# ============================================================================

subtest 'flush works' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("test data\n");
    ok($conn->flush(), 'flush() returns true');

    my $line = $conn->read_line();
    is($line, "test data\n", 'data arrives after flush');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 22: Connect with hashref options (convenience)
# ============================================================================
#
# The connect() function should accept a plain hashref as options.

subtest 'connect with hashref options' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port, { read_timeout => 5 });
    $listener->close();

    ok(defined $conn, 'connect with hashref options succeeds');
    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 23: Connection object is a TcpConnection
# ============================================================================

subtest 'connection isa TcpConnection' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    isa_ok($conn, 'CodingAdventures::TcpClient::TcpConnection');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 24: Large data transfer
# ============================================================================
#
# Send and receive a larger payload (64 KiB) to verify that the buffered
# read/write loop handles multi-chunk transfers correctly.

subtest 'large data transfer' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    # Create a 64 KiB payload of repeating bytes
    my $payload = 'X' x 65536;

    $conn->write_all($payload);

    my $result = $conn->read_exact(65536);
    is(length($result), 65536, 'received 64 KiB');
    is($result, $payload,      'large payload matches');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Test 25: Input validation -- bad port
# ============================================================================

subtest 'input validation - bad port' => sub {
    eval { connect('127.0.0.1', 0) };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'port 0 throws TcpError');

    eval { connect('127.0.0.1', 99999) };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'port 99999 throws TcpError');

    eval { connect('127.0.0.1', 'abc') };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'non-numeric port throws TcpError');
};

# ============================================================================
# Test 26: Input validation -- empty host
# ============================================================================

subtest 'input validation - empty host' => sub {
    eval { connect('', 80) };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'empty host throws TcpError');

    eval { connect(undef, 80) };
    ok(ref $@ && $@->isa('CodingAdventures::TcpClient::TcpError'),
       'undef host throws TcpError');
};

# ============================================================================
# Test 27: read_until with newline (byte 10) -- same as read_line
# ============================================================================

subtest 'read_until newline byte' => sub {
    my ($listener, $port) = start_listener();
    my $pid = fork_echo_server($listener);

    my $conn = connect('127.0.0.1', $port);
    $listener->close();

    $conn->write_all("hello\nworld\n");

    my $first = $conn->read_until(10);    # 10 = ord("\n")
    is($first, "hello\n", 'read_until(10) reads until newline');

    $conn->close();
    waitpid($pid, 0);
};

# ============================================================================
# Done
# ============================================================================

done_testing;
