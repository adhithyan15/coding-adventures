package CodingAdventures::StarlarkCompiler::Messages;

use strict;
use warnings;
use v5.28;

use parent 'Exporter';

our @EXPORT = qw(
    ERROR_ID_INVALID_BYTE
    ERROR_ID_UNKNOWN_OPCODE
    ERROR_ID_UNKNOWN_OPERATOR
    ERROR_ID_INVALID_CORPUS
    MSG_INVALID_BYTE
    MSG_UNKNOWN_OPCODE
    MSG_UNKNOWN_OPERATOR
    MSG_CORPUS_BYTE_LIMIT_EXCEEDED
    MSG_CORPUS_MALFORMED_TRANSPORT
    MSG_CORPUS_SCHEMA_VIOLATION
    REDACTED
    INVALID
);

use constant {
    ERROR_ID_INVALID_BYTE          => 'invalid-byte',
    ERROR_ID_UNKNOWN_OPCODE        => 'unknown-opcode',
    ERROR_ID_UNKNOWN_OPERATOR      => 'unknown-operator',
    ERROR_ID_INVALID_CORPUS        => 'invalid-corpus',

    MSG_INVALID_BYTE               => 'Byte value is outside 0..255 or falls into an unassigned opcode hole.',
    MSG_UNKNOWN_OPCODE             => 'Opcode symbol is unknown or unassigned in the Starlark bytecode specification.',
    MSG_UNKNOWN_OPERATOR           => 'Operator symbol is not mapped to any Starlark bytecode opcode.',
    MSG_CORPUS_BYTE_LIMIT_EXCEEDED => 'STP01 corpus exceeds maximum byte limit of 131,072 bytes.',
    MSG_CORPUS_MALFORMED_TRANSPORT => 'STP01 corpus is malformed JSON or contains invalid UTF-8 encoding.',
    MSG_CORPUS_SCHEMA_VIOLATION    => 'STP01 corpus does not conform to standalone-provider-v1 schema specification.',

    REDACTED                       => '<redacted>',
    INVALID                        => '<invalid>',
};

1;
