package CodingAdventures::HttpCore;

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';
our @EXPORT_OK = qw(find_header parse_content_length parse_content_type);

sub find_header {
    my ($headers, $name) = @_;
    my $lowered = lc($name);

    foreach my $header (@{$headers}) {
        return $header->{value} if lc($header->{name}) eq $lowered;
    }

    return undef;
}

sub parse_content_length {
    my ($headers) = @_;
    my $value = find_header($headers, 'Content-Length');
    return undef if !defined $value || $value !~ /\A\d+\z/;
    return int($value);
}

sub parse_content_type {
    my ($headers) = @_;
    my $value = find_header($headers, 'Content-Type');
    return undef if !defined $value;

    my @pieces = map { s/^\s+|\s+$//gr } split(/;/, $value);
    my $media_type = shift @pieces;
    return undef if !defined $media_type || $media_type eq '';

    my $charset;
    foreach my $piece (@pieces) {
        my ($key, $raw_value) = split(/=/, $piece, 2);
        next if !defined $raw_value;
        $key =~ s/^\s+|\s+$//g;
        next if lc($key) ne 'charset';

        $raw_value =~ s/^\s+|\s+$//g;
        $raw_value =~ s/^"//;
        $raw_value =~ s/"$//;
        $charset = $raw_value;
        last;
    }

    return ($media_type, $charset);
}

package CodingAdventures::HttpCore::Header;

sub new {
    my ($class, %args) = @_;
    return bless {
        name  => $args{name},
        value => $args{value},
    }, $class;
}

package CodingAdventures::HttpCore::HttpVersion;

sub new {
    my ($class, %args) = @_;
    return bless {
        major => $args{major},
        minor => $args{minor},
    }, $class;
}

sub parse {
    my ($class, $text) = @_;
    die "invalid HTTP version: $text" if $text !~ /\AHTTP\/(\d+)\.(\d+)\z/;
    return $class->new(major => int($1), minor => int($2));
}

sub as_string {
    my ($self) = @_;
    return "HTTP/$self->{major}.$self->{minor}";
}

package CodingAdventures::HttpCore::BodyKind;

sub new {
    my ($class, %args) = @_;
    return bless {
        mode   => $args{mode},
        length => $args{length},
    }, $class;
}

sub none            { return __PACKAGE__->new(mode => 'none', length => undef); }
sub content_length  { return __PACKAGE__->new(mode => 'content-length', length => $_[1]); }
sub until_eof       { return __PACKAGE__->new(mode => 'until-eof', length => undef); }
sub chunked         { return __PACKAGE__->new(mode => 'chunked', length => undef); }

package CodingAdventures::HttpCore::RequestHead;

sub new {
    my ($class, %args) = @_;
    return bless {
        method  => $args{method},
        target  => $args{target},
        version => $args{version},
        headers => $args{headers} || [],
    }, $class;
}

sub header         { return CodingAdventures::HttpCore::find_header($_[0]->{headers}, $_[1]); }
sub content_length { return CodingAdventures::HttpCore::parse_content_length($_[0]->{headers}); }

package CodingAdventures::HttpCore::ResponseHead;

sub new {
    my ($class, %args) = @_;
    return bless {
        version => $args{version},
        status  => $args{status},
        reason  => $args{reason},
        headers => $args{headers} || [],
    }, $class;
}

sub header         { return CodingAdventures::HttpCore::find_header($_[0]->{headers}, $_[1]); }
sub content_length { return CodingAdventures::HttpCore::parse_content_length($_[0]->{headers}); }
sub content_type   { return CodingAdventures::HttpCore::parse_content_type($_[0]->{headers}); }

1;
