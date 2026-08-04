<!-- learning-concepts: lz77, lz78, lzss, lzw, huffman-compression, huffman-tree, deflate, brotli, zstd, rans, range-coder, reed-solomon -->

# Dictionary and Entropy Coding

Compression works by describing likely or repeated data more cheaply. It does
not make information disappear: a decoder must be able to reconstruct the
original bytes from the compressed representation.

## Two Complementary Ideas

Dictionary coding replaces repeated sequences with references. Entropy coding
assigns short representations to likely symbols and longer representations to
unlikely ones. Production formats commonly compose the two.

## Sliding Windows: LZ77 and LZSS

LZ77 treats already-decoded output as a dictionary. A match can be represented
as a distance backward and a length. The decoder copies those earlier bytes,
which also allows overlapping copies to represent long runs.

LZSS adds a choice between a literal and a match. A match is emitted only when
its reference costs less than the bytes it replaces. The token stream therefore
needs a flag or another way to distinguish the two cases.

The window size limits memory and determines the maximum backward distance.
The match finder determines compression effort: searching harder may shrink
output, but it also costs more CPU time.

## Growing Dictionaries: LZ78 and LZW

LZ78 builds a table of phrases and emits a phrase reference plus a new symbol.
LZW begins with a known alphabet and emits dictionary codes; encoder and
decoder grow identical tables by following the same rules.

Real formats must define code width changes and dictionary reset behavior.
Those boundary rules are a common source of interoperability bugs.

## Huffman Coding

Huffman coding builds a prefix code from symbol frequencies. No code is a
prefix of another, so the decoder can identify symbols without separators.
A priority queue repeatedly combines the two least-frequent nodes to build the
tree.

Formats usually serialize code lengths and reconstruct canonical codes instead
of serializing pointer-shaped trees. Canonical codes are deterministic and
compact to describe.

## Deflate, Brotli, and Zstandard

Deflate combines LZ77-style matches with Huffman coding. The dictionary stage
finds repetition; the entropy stage compresses the resulting literals,
lengths, and distances.

Brotli and Zstandard use the same broad composition but add richer modeling,
larger or reusable dictionaries, and format-specific entropy machinery. Their
details differ, yet the useful reading strategy is the same:

1. identify the token alphabet
2. find how matches are represented
3. find how probabilities or code tables are transmitted
4. trace one block through encoder and decoder

## Range Coding and rANS

Range coding represents a sequence as a progressively narrowed numeric
interval. rANS represents state transitions whose sizes reflect symbol
probabilities. Both can approach the information content predicted by the
model more closely than an integer-bit-length prefix code.

They are not probability models by themselves. The model supplies frequencies;
the coder converts those frequencies and symbols into bits.

## Reed-Solomon Is Different

Reed-Solomon adds redundancy so a receiver can repair missing or corrupted
symbols. It is error correction, not compression. Its arithmetic happens over
a finite field, and parity symbols provide enough constraints to recover a
bounded amount of damage.

It often appears beside compressed data in barcodes and storage formats:
compression removes redundancy, then error correction deliberately adds
structured redundancy suitable for recovery.

## Invariants Worth Testing

- decoding an encoded input returns the exact original bytes
- malformed distances and truncated bit streams fail safely
- dictionary-width and block boundaries round-trip
- canonical tables are deterministic
- error correction succeeds within its promised damage budget and fails
  clearly beyond it
