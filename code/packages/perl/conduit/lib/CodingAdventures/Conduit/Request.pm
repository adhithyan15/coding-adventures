package CodingAdventures::Conduit::Request;

# ============================================================================
# CodingAdventures::Conduit::Request — read-only view of an HTTP request
# ============================================================================
#
# Constructed from the flat env hashref the Rust side passes to a handler.
# Route params, query params, and headers cross as percent-encoded k=v&…
# strings and are decoded lazily on first access.

use strict;
use warnings;

sub _new {
    my ($class, $env) = @_;
    bless { env => $env }, $class;
}

sub method       { $_[0]{env}{REQUEST_METHOD}        // 'GET' }
sub path         { $_[0]{env}{PATH_INFO}             // '/'   }
sub query_string { $_[0]{env}{QUERY_STRING}          // ''    }
sub body         { $_[0]{env}{'conduit.body'}        // ''    }
sub content_type { $_[0]{env}{'conduit.content_type'} // ''   }
sub remote_addr  { $_[0]{env}{REMOTE_ADDR}           // ''    }
sub error        { $_[0]{env}{'conduit.error'}       // ''    }

sub params       { $_[0]{_params}  //= _decode($_[0]{env}{'conduit.route_params'}) }
sub query_params { $_[0]{_query}   //= _decode($_[0]{env}{'conduit.query_params'}) }
sub headers      { $_[0]{_headers} //= _decode($_[0]{env}{'conduit.headers'}) }

sub param        { $_[0]->params->{ $_[1] } }
sub query_param  { $_[0]->query_params->{ $_[1] } }
sub header       { $_[0]->headers->{ lc $_[1] } }

sub env          { $_[0]{env} }

# Decode a "k=v&k2=v2" percent-encoded string into a hashref.
sub _decode {
    my ($enc) = @_;
    return {} unless defined $enc && length $enc;
    my %h;
    for my $pair (split /&/, $enc) {
        next unless length $pair;
        my ($k, $v) = split /=/, $pair, 2;
        $h{ _unpct($k) } = _unpct(defined $v ? $v : '');
    }
    return \%h;
}

sub _unpct {
    my ($s) = @_;
    $s =~ s/\+/ /g;
    $s =~ s/%([0-9A-Fa-f]{2})/chr(hex($1))/ge;
    return $s;
}

1;
