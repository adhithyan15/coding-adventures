package CodingAdventures::Http1;

# ============================================================================
# CodingAdventures::Http1 — HTTP/1 request and response head parser with body framing detection
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::Http1;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(parse_request_head parse_response_head);

use CodingAdventures::HttpCore;

sub parse_request_head {
    my ($input) = @_;
    my ($lines, $body_offset) = _split_head_lines($input);
    die "invalid HTTP/1 start line" if !@{$lines};

    my @parts = split(/\s+/, $lines->[0]);
    die "invalid HTTP/1 start line: $lines->[0]" if scalar(@parts) != 3;

    my $version = CodingAdventures::HttpCore::HttpVersion->parse($parts[2]);
    my $headers = _parse_headers([ @{$lines}[1 .. $#{$lines}] ]);
    my $body_kind = _request_body_kind($headers);

    return CodingAdventures::Http1::ParsedRequestHead->new(
        head => CodingAdventures::HttpCore::RequestHead->new(
            method  => $parts[0],
            target  => $parts[1],
            version => $version,
            headers => $headers,
        ),
        body_offset => $body_offset,
        body_kind   => $body_kind,
    );
}

sub parse_response_head {
    my ($input) = @_;
    my ($lines, $body_offset) = _split_head_lines($input);
    die "invalid HTTP/1 status line" if !@{$lines};

    my @parts = split(/\s+/, $lines->[0]);
    die "invalid HTTP/1 status line: $lines->[0]" if scalar(@parts) < 2;

    my $version = CodingAdventures::HttpCore::HttpVersion->parse($parts[0]);
    die "invalid HTTP status: $parts[1]" if $parts[1] !~ /\A\d+\z/;
    my $headers = _parse_headers([ @{$lines}[1 .. $#{$lines}] ]);
    my $body_kind = _response_body_kind(int($parts[1]), $headers);

    return CodingAdventures::Http1::ParsedResponseHead->new(
        head => CodingAdventures::HttpCore::ResponseHead->new(
            version => $version,
            status  => int($parts[1]),
            reason  => join(' ', @parts[2 .. $#parts]),
            headers => $headers,
        ),
        body_offset => $body_offset,
        body_kind   => $body_kind,
    );
}

sub _split_head_lines {
    my ($input) = @_;
    my $index = 0;

    while (1) {
        if (substr($input, $index, 2) eq "\r\n") {
            $index += 2;
            next;
        }
        if (substr($input, $index, 1) eq "\n") {
            $index += 1;
            next;
        }
        last;
    }

    my @lines;
    while (1) {
        die "incomplete HTTP/1 head" if $index >= length($input);
        my $line_end = index($input, "\n", $index);
        die "incomplete HTTP/1 head" if $line_end < 0;

        my $line = substr($input, $index, $line_end - $index);
        $line =~ s/\r$//;
        $index = $line_end + 1;

        return (\@lines, $index) if $line eq '';
        push @lines, $line;
    }
}

sub _parse_headers {
    my ($lines) = @_;
    my @headers;

    foreach my $line (@{$lines}) {
        my ($name, $raw_value) = split(/:/, $line, 2);
        die "invalid HTTP/1 header: $line" if !defined($raw_value) || $name =~ /\A\s*\z/;

        $name =~ s/^\s+|\s+$//g;
        $raw_value =~ s/^\s+|\s+$//g;
        push @headers, CodingAdventures::HttpCore::Header->new(name => $name, value => $raw_value);
    }

    return \@headers;
}

sub _request_body_kind {
    my ($headers) = @_;
    return CodingAdventures::HttpCore::BodyKind->chunked() if _chunked_transfer_encoding($headers);

    my $length = _declared_content_length($headers);
    return CodingAdventures::HttpCore::BodyKind->none() if !defined($length) || $length == 0;
    return CodingAdventures::HttpCore::BodyKind->content_length($length);
}

sub _response_body_kind {
    my ($status, $headers) = @_;
    return CodingAdventures::HttpCore::BodyKind->none() if ($status >= 100 && $status < 200) || $status == 204 || $status == 304;
    return CodingAdventures::HttpCore::BodyKind->chunked() if _chunked_transfer_encoding($headers);

    my $length = _declared_content_length($headers);
    return CodingAdventures::HttpCore::BodyKind->until_eof() if !defined($length);
    return CodingAdventures::HttpCore::BodyKind->none() if $length == 0;
    return CodingAdventures::HttpCore::BodyKind->content_length($length);
}

sub _declared_content_length {
    my ($headers) = @_;
    my $value = CodingAdventures::HttpCore::find_header($headers, 'Content-Length');
    return undef if !defined $value;
    die "invalid Content-Length: $value" if $value !~ /\A\d+\z/;
    return int($value);
}

sub _chunked_transfer_encoding {
    my ($headers) = @_;
    foreach my $header (@{$headers}) {
        next if lc($header->{name}) ne 'transfer-encoding';
        foreach my $piece (split(/,/, $header->{value})) {
            $piece =~ s/^\s+|\s+$//g;
            return 1 if lc($piece) eq 'chunked';
        }
    }
    return 0;
}

package CodingAdventures::Http1::ParsedRequestHead;

sub new {
    my ($class, %args) = @_;
    return bless {
        head        => $args{head},
        body_offset => $args{body_offset},
        body_kind   => $args{body_kind},
    }, $class;
}

package CodingAdventures::Http1::ParsedResponseHead;

sub new {
    my ($class, %args) = @_;
    return bless {
        head        => $args{head},
        body_offset => $args{body_offset},
        body_kind   => $args{body_kind},
    }, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::Http1 - HTTP/1 request and response head parser with body framing detection

=head1 SYNOPSIS

    use CodingAdventures::Http1;

=head1 DESCRIPTION

HTTP/1 request and response head parser with body framing detection

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
