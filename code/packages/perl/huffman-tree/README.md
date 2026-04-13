# CodingAdventures::HuffmanTree (Perl)

DT27: Huffman Tree — Optimal prefix-free entropy coding.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What Is a Huffman Tree?

A Huffman tree is a full binary tree (every internal node has exactly two
children) built from a symbol alphabet so that each symbol gets a unique
variable-length bit code. Symbols that appear often get short codes; symbols
that appear rarely get long codes. The total bits needed to encode a message is
minimised — it is the theoretically optimal prefix-free code for a given symbol
frequency distribution.

## Installation

```bash
cpanm CodingAdventures::HuffmanTree
```

Tree construction uses the separate `CodingAdventures::Heap` package for the
shared min-heap implementation.

## Usage

```perl
use CodingAdventures::HuffmanTree;

# Build a tree from [symbol, frequency] pairs.
my $tree = CodingAdventures::HuffmanTree->build([
    [65, 3],   # 'A' appears 3 times
    [66, 2],   # 'B' appears 2 times
    [67, 1],   # 'C' appears 1 time
]);

# Get the code table: { symbol => bit_string }
my $table = $tree->code_table();
# $table->{65} = "0"    (A gets the shortest code)
# $table->{67} = "10"   (C)
# $table->{66} = "11"   (B)

# Encode a message
my @message = (65, 65, 66, 67);   # AABC
my $bits = join('', map { $table->{$_} } @message);
# $bits = "001110"

# Decode
my @decoded = $tree->decode_all($bits, 4);
# @decoded = (65, 65, 66, 67)

# Canonical codes (DEFLATE-style)
my $canon = $tree->canonical_code_table();

# Inspection
print $tree->weight();        # 6
print $tree->depth();         # 2
print $tree->symbol_count();  # 3

# In-order leaf traversal
for my $pair ($tree->leaves()) {
    print "$pair->[0] => $pair->[1]\n";  # symbol => code
}

# Validity check
print $tree->is_valid() ? "valid" : "invalid";
```

## API

| Method | Description |
|---|---|
| `build(\@weights)` | Build tree from `[[symbol, freq], ...]` |
| `code_table()` | Returns `{symbol => bitstring}` |
| `code_for($symbol)` | Returns the bit string for one symbol, or undef |
| `canonical_code_table()` | Returns DEFLATE-style canonical codes |
| `decode_all($bits, $count)` | Decode `$count` symbols from a bit string |
| `weight()` | Total weight (sum of all frequencies) |
| `depth()` | Maximum code length |
| `symbol_count()` | Number of distinct symbols |
| `leaves()` | In-order leaf list: `[$symbol, $code]` pairs |
| `is_valid()` | Check structural invariants; returns 1 or 0 |

## Running Tests

```bash
cpanm --installdeps --quiet .
prove -l -v t/
```

## License

MIT
