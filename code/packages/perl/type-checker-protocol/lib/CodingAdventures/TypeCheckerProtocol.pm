package CodingAdventures::TypeCheckerProtocol;

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(new_type_error_diagnostic new_type_check_result);

sub new_type_error_diagnostic {
    my ($message, $line, $column) = @_;
    return {
        message => $message,
        line    => defined $line ? $line : 1,
        column  => defined $column ? $column : 1,
    };
}

sub new_type_check_result {
    my ($typed_ast, $errors) = @_;
    $errors ||= [];
    return {
        typed_ast => $typed_ast,
        errors    => $errors,
        ok        => @$errors == 0 ? 1 : 0,
    };
}

package CodingAdventures::TypeCheckerProtocol::GenericTypeChecker;

use strict;
use warnings;

use Scalar::Util qw(blessed);

sub new {
    my ($class, %args) = @_;
    return bless {
        hooks     => {},
        errors    => [],
        node_kind => $args{node_kind},
        locate    => $args{locate} || sub { return (1, 1); },
    }, $class;
}

sub reset {
    my ($self) = @_;
    $self->{errors} = [];
    return $self;
}

sub register_hook {
    my ($self, $phase, $kind, $hook) = @_;
    my $key_kind = $kind eq '*' ? '*' : _normalize_kind($kind);
    my $key = $phase . ':' . $key_kind;
    push @{ $self->{hooks}{$key} ||= [] }, $hook;
    return $self;
}

sub dispatch {
    my ($self, $phase, $node, @args) = @_;
    my $kind = '';
    if ($self->{node_kind}) {
        $kind = _normalize_kind($self->{node_kind}->($node) // '');
    }

    for my $key ($phase . ':' . $kind, $phase . ':*') {
        for my $hook (@{ $self->{hooks}{$key} || [] }) {
            my $result = $hook->($node, @args);
            return $result unless blessed($result) && $result->isa('CodingAdventures::TypeCheckerProtocol::NotHandled');
        }
    }

    return undef;
}

sub not_handled {
    return bless {}, 'CodingAdventures::TypeCheckerProtocol::NotHandled';
}

sub error {
    my ($self, $message, $subject) = @_;
    my ($line, $column) = $self->{locate}->($subject);
    push @{ $self->{errors} }, CodingAdventures::TypeCheckerProtocol::new_type_error_diagnostic($message, $line, $column);
    return undef;
}

sub errors {
    my ($self) = @_;
    return [ @{ $self->{errors} } ];
}

sub check {
    my ($self, $ast, $run) = @_;
    $self->reset;
    $run->($self, $ast) if $run;
    return CodingAdventures::TypeCheckerProtocol::new_type_check_result($ast, $self->errors);
}

sub _normalize_kind {
    my ($kind) = @_;
    my $normalized = '';
    my $last_underscore = 0;

    for my $char (split //, $kind) {
        if ($char =~ /[[:alnum:]]/) {
            $normalized .= $char;
            $last_underscore = 0;
        } elsif (!$last_underscore) {
            $normalized .= '_';
            $last_underscore = 1;
        }
    }

    $normalized =~ s/^_+//;
    $normalized =~ s/_+$//;
    return $normalized;
}

1;
