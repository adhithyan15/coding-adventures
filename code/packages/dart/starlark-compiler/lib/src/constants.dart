/// Package-wide constants, protocol specifications, and validation bounds
/// for the Starlark opcode provider.
///
/// All values are explicit constants to prevent inline magic numbers and
/// guarantee deterministic behavior across all supported execution targets.
library;

/// Semantic package version for coding_adventures_starlark_compiler.
const String starlarkCompilerPackageVersion = '0.1.0';

/// Schema version for the standalone Starlark opcode provider specification (STP01).
const int stp01SchemaVersion = 1;

/// Canonical profile identifier for standalone opcode providers.
const String stp01Profile = 'standalone-provider-v1';

/// Maximum byte limit for incoming STP01 test corpus documents (128 KB).
///
/// Enforces bounded memory allocation during corpus ingestion, preventing
/// denial-of-service via unbounded JSON payload buffering.
const int maxCorpusBytes = 131072;

/// Maximum allowed object/array nesting depth in STP01 JSON documents.
///
/// Protects against stack exhaustion attacks through deeply nested structures.
const int maxJsonNestingDepth = 8;

/// Minimum valid 8-bit unsigned byte value (0x00).
const int minByteValue = 0x00;

/// Maximum valid 8-bit unsigned byte value (0xFF = 255).
const int maxByteValue = 0xFF;

/// Total number of distinct defined opcodes in the Starlark bytecode specification.
const int expectedOpcodeCount = 61;

/// Total number of byte entries in the full 8-bit decode table (0..255).
const int expectedDecodeCount = 256;

/// Total number of standard operator mappings across all operator categories.
const int expectedOperatorCount = 34;

/// Total number of probe operators in the unknown lookups suite.
const int expectedUnknownLookupCount = 8;

/// Total number of unassigned byte holes in the 256-byte decode matrix (256 - 61).
const int expectedHoleCount = 195;

/// Lower bound of the Unicode UTF-16 surrogate range (U+D800).
const int minSurrogateCodePoint = 0xD800;

/// Upper bound of the Unicode UTF-16 surrogate range (U+DFFF).
const int maxSurrogateCodePoint = 0xDFFF;

/// Lower bound of high surrogate code units (U+D800).
const int minHighSurrogate = 0xD800;

/// Upper bound of high surrogate code units (U+DBFF).
const int maxHighSurrogate = 0xDBFF;

/// Lower bound of low surrogate code units (U+DC00).
const int minLowSurrogate = 0xDC00;

/// Upper bound of low surrogate code units (U+DFFF).
const int maxLowSurrogate = 0xDFFF;
