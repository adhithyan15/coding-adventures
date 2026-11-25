package CodingAdventures::StarlarkCompiler::Errors;

use strict;
use warnings;
use v5.28;

use parent 'Exporter';

use CodingAdventures::StarlarkCompiler::Messages;

our @EXPORT = qw(
    invalid_byte
    unknown_opcode
    unknown_operator
    corpus_byte_limit_exceeded
    corpus_malformed_transport
    corpus_schema_violation
    is_starlark_compiler_error
);

package CodingAdventures::StarlarkCompiler::Error;

use overload
    '""'     => sub { $_[0]->{message} },
    fallback => 1;

sub error_id { $_[0]->{error_id} }
sub message  { $_[0]->{message} }

package CodingAdventures::StarlarkCompiler::Errors;

my $_make_error = sub {
    my ($error_id, $message) = @_;
    return bless {
        error_id => $error_id,
        message  => $message,
    }, 'CodingAdventures::StarlarkCompiler::Error';
};

sub invalid_byte {
    return $_make_error->(
        ERROR_ID_INVALID_BYTE,
        MSG_INVALID_BYTE,
    );
}

sub unknown_opcode {
    return $_make_error->(
        ERROR_ID_UNKNOWN_OPCODE,
        MSG_UNKNOWN_OPCODE,
    );
}

sub unknown_operator {
    return $_make_error->(
        ERROR_ID_UNKNOWN_OPERATOR,
        MSG_UNKNOWN_OPERATOR,
    );
}

sub corpus_byte_limit_exceeded {
    return $_make_error->(
        ERROR_ID_INVALID_CORPUS,
        MSG_CORPUS_BYTE_LIMIT_EXCEEDED,
    );
}

sub corpus_malformed_transport {
    return $_make_error->(
        ERROR_ID_INVALID_CORPUS,
        MSG_CORPUS_MALFORMED_TRANSPORT,
    );
}

sub corpus_schema_violation {
    return $_make_error->(
        ERROR_ID_INVALID_CORPUS,
        MSG_CORPUS_SCHEMA_VIOLATION,
    );
}

sub is_starlark_compiler_error {
    my ($err) = @_;
    return 0 unless defined $err && ref($err);
    return ref($err) eq 'CodingAdventures::StarlarkCompiler::Error' ? 1 : 0;
}

1;
