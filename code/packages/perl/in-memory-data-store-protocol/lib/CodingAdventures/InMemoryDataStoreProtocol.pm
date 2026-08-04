package CodingAdventures::InMemoryDataStoreProtocol;

use strict;
use warnings;

our $VERSION = '0.1.0';

sub ascii_upper {
    my ($data) = @_;
    die "data must be a string\n" if !defined($data) || ref($data);

    my @bytes = unpack('C*', $data);
    for my $byte (@bytes) {
        die "data must contain only ASCII bytes\n" if $byte > 127;
        $byte -= 32 if $byte >= ord('a') && $byte <= ord('z');
    }
    return pack('C*', @bytes);
}

package CodingAdventures::InMemoryDataStoreProtocol::CommandFrame;

use strict;
use warnings;

sub _copy_strings {
    my ($values, $name) = @_;
    return [] if !defined $values;
    die "$name must be an array reference\n" if ref($values) ne 'ARRAY';

    my @copy;
    for my $index (0 .. $#$values) {
        my $value = $values->[$index];
        die "$name\[$index] must be a string\n" if !defined($value) || ref($value);
        push @copy, "$value";
    }
    return \@copy;
}

sub new {
    my ($class, $command, $args) = @_;
    die "command must be a string\n" if !defined($command) || ref($command);
    return bless {command => "$command", args => _copy_strings($args, 'args')}, $class;
}

sub from_parts {
    my ($class, $parts) = @_;
    die "parts must be an array reference\n" if ref($parts) ne 'ARRAY';
    return undef if !@$parts;

    my $copy = _copy_strings($parts, 'parts');
    my $command = shift @$copy;
    return $class->new(CodingAdventures::InMemoryDataStoreProtocol::ascii_upper($command), $copy);
}

sub command { return $_[0]->{command}; }

sub args {
    my ($self) = @_;
    return [@{ $self->{args} }];
}

sub to_parts {
    my ($self) = @_;
    return [$self->{command}, @{ $self->{args} }];
}

package CodingAdventures::InMemoryDataStoreProtocol::EngineResponse;

use strict;
use warnings;
use Scalar::Util qw(blessed looks_like_number);

my %VALID_KINDS = map { $_ => 1 } qw(simple_string error integer bulk_string array);

sub _copy_response_array {
    my ($values) = @_;
    return undef if !defined $values;
    die "array response value must be an array reference or undef\n"
        if ref($values) ne 'ARRAY';

    my @copy;
    for my $index (0 .. $#$values) {
        my $value = $values->[$index];
        die "array response value\[$index] must be an EngineResponse\n"
            if !blessed($value) || !$value->isa(__PACKAGE__);
        push @copy, $value;
    }
    return \@copy;
}

sub new {
    my ($class, $kind, $value) = @_;
    die "invalid response kind: " . (defined($kind) ? $kind : 'undef') . "\n"
        if !defined($kind) || !$VALID_KINDS{$kind};
    die "$kind value must be a string\n"
        if ($kind eq 'simple_string' || $kind eq 'error')
            && (!defined($value) || ref($value));
    die "integer value must be an integer\n"
        if $kind eq 'integer'
            && (!defined($value) || !looks_like_number($value) || int($value) != $value);
    die "bulk_string value must be a string or undef\n"
        if $kind eq 'bulk_string' && defined($value) && ref($value);
    $value = _copy_response_array($value) if $kind eq 'array';
    return bless {kind => $kind, value => $value}, $class;
}

sub kind  { return $_[0]->{kind}; }
sub value { return $_[0]->{value}; }

sub simple_string { return $_[0]->new('simple_string', $_[1]); }
sub error         { return $_[0]->new('error', $_[1]); }
sub integer       { return $_[0]->new('integer', $_[1]); }
sub bulk_string   { return $_[0]->new('bulk_string', $_[1]); }
sub array         { return $_[0]->new('array', $_[1]); }
sub ok            { return $_[0]->simple_string('OK'); }
sub null          { return $_[0]->bulk_string(undef); }
sub zero          { return $_[0]->integer(0); }
sub one           { return $_[0]->integer(1); }

1;
