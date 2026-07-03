package CodingAdventures::IrcServerNative;

# ============================================================================
# CodingAdventures::IrcServerNative — a high-performance IRC server for Perl
# ============================================================================
#
# Every line of IRC and TCP logic runs in Rust (the irc-net-reactor engine on
# the home-grown kqueue/epoll reactor); Perl only launches and controls the
# server through an XS native library. There is no callback into Perl.
#
#   use CodingAdventures::IrcServerNative;
#
#   my $server = CodingAdventures::IrcServerNative->new(port => 6667);
#   $server->serve_background;
#   # ... connect IRC clients to $server->local_host : $server->local_port ...
#   $server->stop;

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA     = ('DynaLoader');

sub dl_load_flags { 0x01 }

__PACKAGE__->bootstrap($VERSION);

# ── Constructor ─────────────────────────────────────────────────────────────

# new(%opts) — opts: host, port, server_name, motd (arrayref), oper_password,
# max_connections. Returns a CodingAdventures::IrcServerNative::Server.
sub new {
    my ($class, %opts) = @_;
    my $host            = defined $opts{host}            ? $opts{host}            : '127.0.0.1';
    my $port            = defined $opts{port}            ? $opts{port}            : 6667;
    my $server_name     = defined $opts{server_name}     ? $opts{server_name}     : 'irc.local';
    my $motd            = $opts{motd} && @{ $opts{motd} } ? $opts{motd}           : ['Welcome.'];
    my $oper_password   = defined $opts{oper_password}   ? $opts{oper_password}   : '';
    my $max_connections = defined $opts{max_connections} ? $opts{max_connections} : 1024;

    # MOTD lines are joined with newlines for a single string arg; the Rust side
    # splits them back into lines.
    my $motd_joined = join("\n", @$motd);

    my $srv = CodingAdventures::IrcServerNative::Native::new_server(
        "$host", int($port), "$server_name", $motd_joined,
        "$oper_password", int($max_connections),
    );
    return CodingAdventures::IrcServerNative::Server->_new($srv);
}

# ============================================================================
package CodingAdventures::IrcServerNative::Server;

use strict;
use warnings;

sub _new {
    my ($class, $srv) = @_;
    return bless { _srv => $srv, _closed => 0 }, $class;
}

# Run the event loop in the calling process, blocking until stop().
sub serve { CodingAdventures::IrcServerNative::Native::server_serve($_[0]{_srv}) }

# Run the event loop on a background Rust thread; returns immediately.
sub serve_background {
    CodingAdventures::IrcServerNative::Native::server_serve_background($_[0]{_srv});
}

# Signal the server to stop and join the background thread.
sub stop { CodingAdventures::IrcServerNative::Native::server_stop($_[0]{_srv}) }

# Whether the event loop is currently running.
sub running {
    CodingAdventures::IrcServerNative::Native::server_running($_[0]{_srv}) ? 1 : 0;
}

# The bound IP address.
sub local_host { CodingAdventures::IrcServerNative::Native::server_local_host($_[0]{_srv}) }

# The bound TCP port (the OS-assigned port when constructed with port => 0).
sub local_port { CodingAdventures::IrcServerNative::Native::server_local_port($_[0]{_srv}) }

# The bound "host:port" address.
sub local_addr { my $s = shift; $s->local_host . ':' . $s->local_port }

# Free the native peer (stops and joins first). Called automatically on DESTROY.
sub close {
    my $self = shift;
    return if $self->{_closed};
    CodingAdventures::IrcServerNative::Native::dispose_server($self->{_srv});
    $self->{_closed} = 1;
}

sub DESTROY { $_[0]->close }

1;
