use strict;
use warnings;
use Test::More;
use IO::Socket::INET;
use CodingAdventures::IrcServerNative;

# End-to-end tests: start the real Rust IRC engine on an ephemeral port and
# drive live IRC clients over real TCP sockets. serve_background is safe here
# because the spawned Rust thread never enters the Perl interpreter.

sub recv_until {
    my ($sock, $needle, $timeout) = @_;
    $timeout //= 5;
    my $deadline = time + $timeout;
    my $buf = '';
    while (time < $deadline) {
        return $buf if index($buf, $needle) >= 0;
        my $rin = '';
        vec($rin, fileno($sock), 1) = 1;
        my $nfound = select(my $rout = $rin, undef, undef, 0.3);
        if ($nfound && $nfound > 0) {
            my $data = '';
            my $got = sysread($sock, $data, 4096);
            last if !defined $got || $got == 0;
            $buf .= $data;
        }
    }
    return $buf;
}

sub register {
    my ($sock, $nick) = @_;
    syswrite($sock, "NICK $nick\r\nUSER $nick 0 * :$nick\r\n");
    my $welcome = recv_until($sock, '001');
    like($welcome, qr/001/, "001 welcome for $nick");
}

sub connect_client {
    my ($port) = @_;
    my $sock = IO::Socket::INET->new(
        PeerAddr => '127.0.0.1',
        PeerPort => $port,
        Proto    => 'tcp',
        Timeout  => 2,
    );
    die "could not connect to 127.0.0.1:$port: $!" unless $sock;
    return $sock;
}

my $server = CodingAdventures::IrcServerNative->new(port => 0, server_name => 'irc.test');
is($server->local_host, '127.0.0.1', 'local_host is loopback');
ok($server->local_port > 0, 'ephemeral port assigned');
is($server->local_addr, '127.0.0.1:' . $server->local_port, 'local_addr');

ok(!$server->running, 'not running before serve');
$server->serve_background;
for (1 .. 200) { last if $server->running; select(undef, undef, undef, 0.005); }
ok($server->running, 'running after serve_background');

my $port = $server->local_port;
my $alice = connect_client($port);
my $bob   = connect_client($port);

register($alice, 'alice');
register($bob, 'bob');

# PING/PONG liveness.
syswrite($alice, "PING :liveness\r\n");
like(recv_until($alice, 'PONG'), qr/PONG/, 'PING gets PONG');

# Join and broadcast.
syswrite($alice, "JOIN #test\r\n");
syswrite($bob, "JOIN #test\r\n");
recv_until($alice, 'JOIN');
recv_until($bob, 'JOIN');

# Alice speaks; Bob (a different connection) must receive it — exercises the
# Rust engine's in-process mailbox fan-out.
syswrite($alice, "PRIVMSG #test :hello bob\r\n");
my $received = recv_until($bob, 'hello bob');
like($received, qr/PRIVMSG/, 'bob received a PRIVMSG');
like($received, qr/hello bob/, "bob received alice's message");

close($alice);
close($bob);
$server->stop;

done_testing();
