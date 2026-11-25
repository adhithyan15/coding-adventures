package CodingAdventures::StarlarkCompiler::Constants;

use strict;
use warnings;
use v5.28;

use parent 'Exporter';

our @EXPORT = qw(
    PACKAGE_VERSION
    STP01_SCHEMA_VERSION
    STP01_PROFILE
    MAX_CORPUS_BYTES
    MAX_JSON_NESTING_DEPTH
    MIN_BYTE_VALUE
    MAX_BYTE_VALUE
    EXPECTED_OPCODE_COUNT
    EXPECTED_DECODE_COUNT
    EXPECTED_OPERATOR_COUNT
    EXPECTED_UNKNOWN_LOOKUP_COUNT
    EXPECTED_HOLE_COUNT
    MIN_SURROGATE_CODE_POINT
    MAX_SURROGATE_CODE_POINT
);

use constant {
    PACKAGE_VERSION               => '0.1.0',
    STP01_SCHEMA_VERSION          => 1,
    STP01_PROFILE                 => 'standalone-provider-v1',
    MAX_CORPUS_BYTES              => 131072,
    MAX_JSON_NESTING_DEPTH        => 8,
    MIN_BYTE_VALUE                => 0,
    MAX_BYTE_VALUE                => 255,
    EXPECTED_OPCODE_COUNT         => 61,
    EXPECTED_DECODE_COUNT         => 256,
    EXPECTED_OPERATOR_COUNT       => 34,
    EXPECTED_UNKNOWN_LOOKUP_COUNT => 8,
    EXPECTED_HOLE_COUNT           => 195,
    MIN_SURROGATE_CODE_POINT      => 0xD800,
    MAX_SURROGATE_CODE_POINT      => 0xDFFF,
};

1;
