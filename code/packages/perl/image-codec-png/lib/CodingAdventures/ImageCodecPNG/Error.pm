package CodingAdventures::ImageCodecPNG::Error;

use strict;
use warnings;
use overload
    '""'     => sub { $_[0]->message },
    fallback => 1;

use CodingAdventures::ImageCodecPNG::Constants qw(:all);
use CodingAdventures::ImageCodecPNG::Messages qw(message_for);

# Mapping set for constant-time code validation.
my %KNOWN_CODES = map { $_ => 1 } @CodingAdventures::ImageCodecPNG::Constants::PNG_ERROR_CODES;

# Constructs a new typed PngError object.
# To enforce Standing Rule #20, this method accepts an error code ONLY.
# The error message is strictly derived from the static Messages catalog.
sub new {
    my ($class, $code) = @_;

    my $safe_code = (defined $code && $KNOWN_CODES{$code})
        ? $code
        : ERROR_UNKNOWN();

    my $self = bless {
        code    => $safe_code,
        message => message_for($safe_code),
    }, ref($class) || $class;

    return $self;
}

# The canonical error code identifier.
sub code { $_[0]->{code} }

# The human-readable diagnostic message.
sub message { $_[0]->{message} }

# Throws a new PngError instance.
sub throw {
    my ($class, $code) = @_;
    die $class->new($code);
}

# Package alias matching canonical naming convention.
{
    no strict 'refs';
    *CodingAdventures::ImageCodecPNG::PngError:: = \*CodingAdventures::ImageCodecPNG::Error::;
}

1;
