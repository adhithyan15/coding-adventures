package CodingAdventures::TestBootstrap::Fuzz::Mutate;

# The mutator algebra (VER06 section 3, VER06q section 3).
#
# Every mutator takes a Perl byte string and returns a NEW string (a mutator never
# changes its argument).  Offsets, counts, deltas and values are `Int`s: native
# integers or normalized `Math::BigInt` values of up to the corpus's 1,000 digits.
# Offsets wrap with floored `mod`.
#
# No big integer is ever built from INPUT BYTES (the Elixir lane crashed on one of
# 2**(8 x 1 MiB)): the data is only ever read as bytes, and never with
# `unpack 'C*'` (lesson 93: one scalar per byte).  Arithmetic and boundary values are
# reduced to their low 64 bits and masked to 8*width bits, which is exact for every
# width the algebra has (1, 2, 4, 8).

use strict;
use warnings;
no warnings q(portable);    # 64-bit hex constants are the point

use CodingAdventures::TestBootstrap::Int            ();
use CodingAdventures::TestBootstrap::Prng           ();
use CodingAdventures::TestBootstrap::Fuzz::Limits   ();

# --- captured at load (VER06q section 1): replacing a sub in another package later
# changes nothing this module does.
my $int_check   = \&CodingAdventures::TestBootstrap::Int::check;
my $int_is_big  = \&CodingAdventures::TestBootstrap::Int::is_big;
my $int_mod     = \&CodingAdventures::TestBootstrap::Int::mod;
my $int_sign    = \&CodingAdventures::TestBootstrap::Int::sign;
my $int_cmp     = \&CodingAdventures::TestBootstrap::Int::cmp;
my $int_to_u64  = \&CodingAdventures::TestBootstrap::Int::to_u64;
my $int_from_u64 = \&CodingAdventures::TestBootstrap::Int::from_u64;
my $add64       = \&CodingAdventures::TestBootstrap::Prng::add64;
my $new_rng     = \&CodingAdventures::TestBootstrap::Prng::new_rng;
my $MAX_ITERATIONS  = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_ITERATIONS();
my $MAX_INPUT_BYTES = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_INPUT_BYTES();
my $MAX_SEQ_WORK    = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_SEQUENCE_WORK();

# Interest values of section 3.4, by width in bytes.  The 64-bit values above 2**63 - 1
# are Ints (normalized Math::BigInt), written here as words.
our @INTEREST_8  = ( 0, 1, 127, 128, 255 );
our @INTEREST_16 = ( 0, 1, 255, 256, 32767, 32768, 65535 );
our @INTEREST_32 = ( 0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF );
our @INTEREST_64 = (
    0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF,
    $int_from_u64->(0x8000000000000000), $int_from_u64->(0xFFFFFFFFFFFFFFFF),
);
# Arithmetic deltas of section 3.3.
our @DELTAS = ( -35, -8, -1, 1, 8, 35 );

my %WIDTHS = ( 1 => 1, 2 => 1, 4 => 1, 8 => 1 );
my %PACK   = (
    little => { 1 => 'C', 2 => 'v',  4 => 'V',  8 => 'Q<' },
    big    => { 1 => 'C', 2 => 'n',  4 => 'N',  8 => 'Q>' },
);

# ------------------------------------------------------------------ arguments

# A byte string argument: defined, not a reference; a utf8-flagged string is reduced
# to bytes (downgraded if every character fits, else its UTF-8 octets).
sub _bytes {
    my ( $v, $name ) = @_;
    die "$name must be a string\n" if !defined $v || ref $v;
    my $c = $v;
    if ( utf8::is_utf8($c) && !utf8::downgrade( $c, 1 ) ) {
        utf8::encode($c);
    }
    return $c;
}

sub _int { return $int_check->( $_[0], $_[1] ) }

sub _endian {
    my ($e) = @_;
    die "endian must be 'little' or 'big'\n" if !defined $e || ref $e || ( $e ne 'little' && $e ne 'big' );
    return $e;
}

# x mod n for an Int x and a native n >= 1: a native in [0, n).
sub _mod_n {
    my ( $x, $n ) = @_;
    return $int_is_big->($x) ? $int_mod->( $x, $n ) : $x % $n;
}

# An Int as a native, saturating at the int64 bounds.
sub _native {
    my ($x) = @_;
    return $x if !$int_is_big->($x);
    return $int_sign->($x) < 0 ? -9223372036854775807 - 1 : 9223372036854775807;
}

sub _clamp {
    my ( $x, $lo, $hi ) = @_;
    my $n = _native($x);
    $n = $lo if $n < $lo;
    $n = $hi if $n > $hi;
    return $n;
}

# ------------------------------------------------------------------- inversion

# 255 - byte, for a whole run, as ONE string XOR.  `'' . $s` and `"\xff" x n` are fresh
# strings with no numeric flags, so Perl uses the string form of `^` (a scalar that
# once saw a numeric context would be XORed as a NUMBER).
sub _invert {
    my ($s) = @_;
    return ( '' . $s ) ^ ( "\xff" x length $s );
}

# Inverts the bytes [a, b) of `$s` (0-based, half open).
sub _invert_bytes {
    my ( $s, $a, $b ) = @_;
    return $s if $b <= $a;
    my $out = $s;
    substr( $out, $a, $b - $a ) = _invert( substr( $s, $a, $b - $a ) );
    return $out;
}

sub _xor_byte {
    my ( $sref, $at, $mask ) = @_;
    substr( ${$sref}, $at, 1 ) = chr( ord( substr( ${$sref}, $at, 1 ) ) ^ $mask );
}

# Inverts the bits [lo, hi) (0-based, half open; bit i is bit i%8 of byte i/8, LSB first).
sub _invert_bits {
    my ( $s, $lo, $hi ) = @_;
    return $s if $hi <= $lo;
    my ( $first, $last ) = ( $lo >> 3, ( $hi - 1 ) >> 3 );
    my $out = $s;
    if ( $first == $last ) {
        _xor_byte( \$out, $first, ( ( 1 << ( $hi - $lo ) ) - 1 ) << ( $lo % 8 ) );
        return $out;
    }
    _xor_byte( \$out, $first, ( 0xFF << ( $lo % 8 ) ) & 0xFF );
    substr( $out, $first + 1, $last - $first - 1 ) = _invert( substr( $s, $first + 1, $last - $first - 1 ) ) if $last > $first + 1;
    _xor_byte( \$out, $last, ( 1 << ( ( $hi - 1 ) % 8 + 1 ) ) - 1 );
    return $out;
}

# Inverts `count` consecutive units (bits, unit 1, or bytes, unit 8) from `offset mod
# units`, wrapping around the buffer.  Only the PARITY of each unit's flips matters (XOR
# twice is a no-op): every unit is hit `count div units` times and the first
# `count mod units` (from the wrapped start) once more, so the work is O(len) for ANY
# count, where a loop of `count` single flips would never end (VER06 9.2).
#
# Worked example, 2 bytes = 16 bits, bit_flip(offset 0, count 17): 17 div 16 = 1 (every bit
# once, odd) and 17 mod 16 = 1 (bit 0 once more, so twice, even): bits 1..15 are inverted
# and bit 0 is unchanged.
sub _parity_flip {
    my ( $raw, $unit, $offset, $count ) = @_;
    my $n = length $raw;
    return $raw if $n == 0 || $int_sign->($count) <= 0;
    my $units = ( $n * 8 ) / $unit;
    my $base  = _mod_n( $offset, $units );
    my ( $odd, $rem );
    if ( $int_is_big->($count) ) {
        my ( $q, $r ) = $count->copy->bdiv($units);
        ( $odd, $rem ) = ( $q->is_odd ? 1 : 0, 0 + $r->bstr );
    }
    else {
        $odd = int( ( $count - $count % $units ) / $units ) % 2;    # exact: both parts are native
        $rem = $count % $units;
    }
    my $out = $raw;
    $out = _invert($out) if $odd;
    if ( $rem > 0 ) {
        # the window [base, base + rem) may run past the end and re-enter at 0; as
        # rem < units the two pieces never overlap
        my $stop = $base + $rem;
        my $flip = $unit == 1 ? \&_invert_bits : \&_invert_bytes;
        $out = $flip->( $out, $base, $stop < $units ? $stop : $units );
        $out = $flip->( $out, 0, $stop - $units ) if $stop > $units;
    }
    return $out;
}

# --------------------------------------------------------------------- public

# Invert `count` consecutive bits from `bit_offset mod (8 * len)`.
# bit_flip("\0\0", 9) is 00 02.
sub bit_flip {
    my ( $data, $off, $count ) = @_;
    $data = _bytes( $data, 'data' );
    $off  = _int( $off, 'bit_offset' );
    $count = defined $count ? _int( $count, 'count' ) : 1;
    return _parity_flip( $data, 1, $off, $count );
}

# XOR `count` consecutive bytes with 0xFF from `byte_offset mod len`.
sub byte_flip {
    my ( $data, $off, $count ) = @_;
    $data = _bytes( $data, 'data' );
    $off  = _int( $off, 'byte_offset' );
    $count = defined $count ? _int( $count, 'count' ) : 1;
    return _parity_flip( $data, 8, $off, $count );
}

# Add signed `delta` modulo 2**(8*width) to the integer at the offset.
# u16 little-endian: ff 00 is 0x00ff; +1 gives 0x0100, stored 00 01.
sub byte_arithmetic {
    my ( $data, $off, $delta, $width, $endian ) = @_;
    $data  = _bytes( $data, 'data' );
    $off   = _int( $off, 'byte_offset' );
    $delta = _int( $delta, 'delta' );
    $width = defined $width ? _int( $width, 'width' ) : 1;
    my $order = _endian( defined $endian ? $endian : 'little' );
    return $data if $int_is_big->($width) || !$WIDTHS{$width} || length($data) < $width;
    my $at  = _mod_n( $off, length($data) - $width + 1 );
    my $fmt = $PACK{$order}{$width};
    my $value = $add64->( scalar unpack( $fmt, substr( $data, $at, $width ) ), $int_to_u64->($delta) );    # words wrap
    $value &= ( ( 1 << ( 8 * $width ) ) - 1 ) if $width < 8;
    my $out = $data;
    substr( $out, $at, $width ) = pack( $fmt, $value );
    return $out;
}

# Overwrite `width` bytes with `val mod 2**(8*width)` at the offset.
# boundary_insert(00 00 00 00, 0, 4, 0x80000000, "little") is 00 00 00 80.
sub boundary_insert {
    my ( $data, $off, $width, $val, $endian ) = @_;
    $data  = _bytes( $data, 'data' );
    $off   = _int( $off, 'byte_offset' );
    $width = _int( $width, 'width' );
    $val   = _int( $val, 'val' );
    my $order = _endian( defined $endian ? $endian : 'little' );
    return $data if $int_is_big->($width) || !$WIDTHS{$width} || length($data) < $width;
    my $at    = _mod_n( $off, length($data) - $width + 1 );
    my $value = $int_to_u64->($val);
    $value &= ( ( 1 << ( 8 * $width ) ) - 1 ) if $width < 8;
    my $out = $data;
    substr( $out, $at, $width ) = pack( $PACK{$order}{$width}, $value );
    return $out;
}

# Delete up to `length` bytes at `offset mod len`; clamped at the end.
sub block_delete {
    my ( $data, $off, $length ) = @_;
    $data   = _bytes( $data, 'data' );
    $off    = _int( $off, 'offset' );
    $length = _int( $length, 'length' );
    my $size = length $data;
    return $data if $size == 0 || $int_sign->($length) <= 0;
    my $at  = _mod_n( $off, $size );
    my $n   = _native($length);
    my $cut = $n < $size - $at ? $n : $size - $at;
    my $out = $data;
    substr( $out, $at, $cut ) = '';
    return $out;
}

# Insert at `offset mod (len + 1)`; an empty input becomes the insert.
sub block_insert {
    my ( $data, $off, $insert ) = @_;
    $data   = _bytes( $data, 'data' );
    $off    = _int( $off, 'offset' );
    $insert = _bytes( $insert, 'insert_bytes' );
    my $size = length $data;
    return $insert if $size == 0;
    my $at  = _mod_n( $off, $size + 1 );
    my $out = $data;
    substr( $out, $at, 0 ) = $insert;
    return $out;
}

# Copy up to `length` bytes from `src_offset mod len` to `dst_offset mod (len + 1)` (the end
# of the buffer is a valid insertion point).
sub block_duplicate {
    my ( $data, $src_off, $length, $dst_off ) = @_;
    $data    = _bytes( $data, 'data' );
    $src_off = _int( $src_off, 'src_offset' );
    $length  = _int( $length, 'length' );
    $dst_off = _int( $dst_off, 'dst_offset' );
    my $size = length $data;
    return $data if $size == 0 || $int_sign->($length) <= 0;
    my $src = _mod_n( $src_off, $size );
    my $n   = _native($length);
    my $chunk = substr( $data, $src, $n < $size - $src ? $n : $size - $src );
    my $dst = _mod_n( $dst_off, $size + 1 );
    my $out = $data;
    substr( $out, $dst, 0 ) = $chunk;
    return $out;
}

# target[0, ts) . donor[ds, ...) with both splits clamped into their buffers.
sub splice_bytes {
    my ( $target, $donor, $ts, $ds ) = @_;
    $target = _bytes( $target, 'target' );
    $donor  = _bytes( $donor, 'donor' );
    $ts     = _int( $ts, 'target_split' );
    $ds     = _int( $ds, 'donor_split' );
    my $t = _clamp( $ts, 0, length $target );
    my $d = _clamp( $ds, 0, length $donor );
    return substr( $target, 0, $t ) . substr( $donor, $d );
}

# `steps` seeded mutations, each chosen by one bounded(7) draw:
#
#   0 bit_flip, count 1            3 boundary_insert, width 1, INTEREST_8
#   1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
#   2 byte_arithmetic, DELTAS      5 block_insert of one random byte
#                                  6 block_duplicate (len 1..n/2), skipped if n <= 1
#
# An empty buffer is refilled to 00 before every step.  The draw order is the cross-lane
# contract (every draw goes into a lexical first).  `max_len` is VER06 section 3.5's length
# budget: an insert or duplicate that would pass it is skipped AFTER its draws, so the stream
# stays aligned and memory stays bounded.
#
# Returns ($bytes, $work) where work is the bytes copied (the initial copy plus one buffer
# per step), or (undef, $work) when the step that would cross `work_cap` is reached.  A
# failed call is charged what it did.
sub mutate_sequence_counted {
    my ( $data, $seed, $steps, $max_len, $work_cap ) = @_;
    $data     = _bytes( $data, 'data' );
    $seed     = _int( $seed, 'seed' );
    $steps    = _int( $steps, 'steps' );
    $max_len  = _int( $max_len, 'max_len' );
    $work_cap = _int( $work_cap, 'work_cap' );
    die "steps exceeds MAX_ITERATIONS ($MAX_ITERATIONS)\n" if $int_cmp->( $steps, $MAX_ITERATIONS ) > 0;
    die "input exceeds MAX_INPUT_BYTES\n" if length($data) > $MAX_INPUT_BYTES;
    my ( $cap, $limit ) = ( _native($work_cap), _native($max_len) );
    my $cur  = $data;
    my $work = length $cur;    # the initial copy is work too
    return ( undef, 0 ) if $work > $cap;
    my $rng   = $new_rng->($seed);
    my $count = _native($steps);
    $count = 0 if $count < 0;
    for ( 1 .. $count ) {
        $cur = "\0" if length($cur) == 0;
        return ( undef, $work ) if $work + length($cur) > $cap;
        $work += length $cur;
        my $n      = length $cur;
        my $choice = $rng->bounded(7);
        if ( $choice == 0 ) {
            my $at = $rng->bounded( $n * 8 );
            $cur = _parity_flip( $cur, 1, $at, 1 );
        }
        elsif ( $choice == 1 ) {
            my $at = $rng->bounded($n);
            $cur = _parity_flip( $cur, 8, $at, 1 );
        }
        elsif ( $choice == 2 ) {
            my $di    = $rng->bounded( scalar @DELTAS );
            my $at    = $rng->bounded($n);
            $cur = byte_arithmetic( $cur, $at, $DELTAS[$di], 1, 'little' );
        }
        elsif ( $choice == 3 ) {
            my $vi = $rng->bounded( scalar @INTEREST_8 );
            my $at = $rng->bounded($n);
            $cur = boundary_insert( $cur, $at, 1, $INTEREST_8[$vi], 'little' );
        }
        elsif ( $choice == 4 ) {
            if ( $n > 1 ) {
                my $half   = int( $n / 2 );
                my $length = 1 + $rng->bounded( $half > 1 ? $half : 1 );
                my $at     = $rng->bounded($n);
                $cur = block_delete( $cur, $at, $length );
            }
        }
        elsif ( $choice == 5 ) {
            my $byte = chr( $rng->bounded(256) );
            my $at   = $rng->bounded( $n + 1 );
            $cur = block_insert( $cur, $at, $byte ) if $n + 1 <= $limit;
        }
        else {
            if ( $n > 1 ) {
                my $half   = int( $n / 2 );
                my $length = 1 + $rng->bounded( $half > 1 ? $half : 1 );
                my $src    = $rng->bounded($n);
                my $dst    = $rng->bounded( $n + 1 );
                my $room   = $n - $src;
                $cur = block_duplicate( $cur, $src, $length, $dst ) if $n + ( $length < $room ? $length : $room ) <= $limit;
            }
        }
    }
    return ( $cur, $work );
}

# The bytes of `mutate_sequence_counted` under the per-call cap; an error if it is crossed.
sub mutate_sequence {
    my ( $data, $seed, $steps, $max_len ) = @_;
    $max_len = $MAX_INPUT_BYTES if !defined $max_len;
    my ($out) = mutate_sequence_counted( $data, $seed, $steps, $max_len, $MAX_SEQ_WORK );
    die "mutate_sequence work cap of $MAX_SEQ_WORK bytes exceeded\n" if !defined $out;
    return $out;
}

1;
