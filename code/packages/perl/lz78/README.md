# LZ78 — Lossless Compression Algorithm (Perl)

Perl implementation of the LZ78 compression algorithm (Lempel & Ziv, 1978),
part of the CMP series in coding-adventures.

## Usage

```perl
use CodingAdventures::LZ78 qw(compress decompress encode decode);

# One-shot
my $compressed = compress("hello hello hello");
my $original   = decompress($compressed);

# Token-level API
my @tokens = encode("AABCBBABC");
my $data   = decode(\@tokens, 9);
```

## TrieCursor

The TrieCursor functions are exported for reuse in streaming dictionary
algorithms like LZW (CMP03):

```perl
use CodingAdventures::LZ78 qw(new_cursor cursor_step cursor_insert cursor_reset cursor_dict_id cursor_at_root);

my $cursor = new_cursor();
cursor_insert($cursor, 65, 1);         # add root→'A'→id=1
if (cursor_step($cursor, 65)) {        # true
    print cursor_dict_id($cursor);     # 1
}
cursor_reset($cursor);                 # back to root
```

## Development

```bash
cpanm --installdeps .
prove -l -v t/
```
