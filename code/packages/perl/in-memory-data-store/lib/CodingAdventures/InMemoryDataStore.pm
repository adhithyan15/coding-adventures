package CodingAdventures::InMemoryDataStore;

use strict;
use warnings;
use Exporter 'import';
use Scalar::Util qw(blessed);
use CodingAdventures::InMemoryDataStoreEngine;
use CodingAdventures::InMemoryDataStoreProtocol;
use CodingAdventures::RespProtocol qw(encode);

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    command_from_resp response_to_resp_value encode_resp_stream
    command_to_frame frame_to_response_text ok
);

my $ENGINE = 'CodingAdventures::InMemoryDataStoreEngine';
my $FRAME = 'CodingAdventures::InMemoryDataStoreProtocol::CommandFrame';
my $VALUE = 'CodingAdventures::RespProtocol::Value';
my $DECODER = 'CodingAdventures::RespProtocol::Decoder';

sub new {
    my ($class, @arguments) = @_;
    die "constructor expects key/value options\n" if @arguments % 2;
    my %options = @arguments;
    die "engine and store are mutually exclusive\n"
        if defined($options{engine}) && defined($options{store});

    my $engine = $options{engine};
    if (!defined $engine) {
        my %engine_options;
        $engine_options{store} = $options{store} if defined $options{store};
        $engine_options{database_count} = $options{database_count}
            if defined $options{database_count};
        $engine_options{time_provider} = $options{time_provider}
            if defined $options{time_provider};
        $engine = $ENGINE->new(%engine_options);
    }
    return bless {engine => $engine, decoder => $DECODER->new}, $class;
}

sub engine { return $_[0]->{engine}; }
sub store { return $_[0]->{engine}->store; }

sub reset {
    my ($self, $store) = @_;
    $self->{engine} = defined($store) ? $ENGINE->new(store => $store) : $ENGINE->new;
    $self->{decoder} = $DECODER->new;
    return $self;
}

sub execute_command {
    my ($self, $command) = @_;
    return response_to_resp_value($self->{engine}->execute_frame($command));
}

sub execute_parts {
    my ($self, $parts) = @_;
    return response_to_resp_value($self->{engine}->execute_parts($parts));
}

sub execute_frame {
    my ($self, $frame) = @_;
    return $VALUE->error('ERR expected RESP array command')
        if !blessed($frame) || !$frame->isa($VALUE)
        || $frame->kind ne 'array' || $frame->is_null;
    return undef if !@{$frame->value};
    my $command = command_from_resp($frame);
    return $VALUE->error('ERR expected RESP command array') if !defined $command;
    return $self->execute_command($command);
}

sub process {
    my ($self, $input) = @_;
    $self->{decoder}->feed($input);
    my @responses;
    while ($self->{decoder}->has_message) {
        my $response = $self->execute_frame($self->{decoder}->get_message);
        push @responses, $response if defined $response;
    }
    return \@responses;
}

sub handle {
    my ($self, $input) = @_;
    return encode_resp_stream($self->process($input));
}

sub command_from_resp {
    my ($value) = @_;
    return undef if !blessed($value) || !$value->isa($VALUE)
        || $value->kind ne 'array' || $value->is_null;

    my @parts;
    for my $item (@{$value->value}) {
        return undef if !blessed($item) || !$item->isa($VALUE);
        if ($item->kind eq 'bulk_string' && !$item->is_null) {
            push @parts, $item->value;
        } elsif ($item->kind eq 'simple_string' || $item->kind eq 'integer') {
            push @parts, '' . $item->value;
        } else {
            return undef;
        }
    }
    return $FRAME->from_parts(\@parts);
}

sub response_to_resp_value {
    my ($response) = @_;
    my $kind = $response->kind;
    return $VALUE->simple_string($response->value) if $kind eq 'simple_string';
    return $VALUE->error($response->value) if $kind eq 'error';
    return $VALUE->integer($response->value) if $kind eq 'integer';
    if ($kind eq 'bulk_string') {
        return defined($response->value)
            ? $VALUE->bulk_string($response->value)
            : $VALUE->null_bulk_string;
    }
    if ($kind eq 'array') {
        my $values = $response->value;
        return $VALUE->null_array if !defined $values;
        return $VALUE->array([map { response_to_resp_value($_) } @$values]);
    }
    die "unknown engine response kind: $kind\n";
}

sub encode_resp_stream {
    my ($values) = @_;
    return join('', map { encode($_) } @$values);
}

sub command_to_frame {
    my ($command) = @_;
    return $VALUE->array([
        $VALUE->bulk_string($command->command),
        map { $VALUE->bulk_string($_) } @{$command->args},
    ]);
}

sub frame_to_response_text {
    my ($frame) = @_;
    return $frame->value if $frame->kind eq 'simple_string' || $frame->kind eq 'error';
    return '' . $frame->value if $frame->kind eq 'integer';
    return $frame->is_null ? '(nil)' : $frame->value if $frame->kind eq 'bulk_string';
    return $frame->is_null ? '(nil)' : '[array:' . scalar(@{$frame->value}) . ']';
}

sub ok { return $VALUE->simple_string('OK'); }

package CodingAdventures::InMemoryDataStore::DataStore;

use strict;
use warnings;
our @ISA = ('CodingAdventures::InMemoryDataStore');

1;
