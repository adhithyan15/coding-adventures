use strict;
use warnings;
use Test2::V0;

require CodingAdventures::Code39;
my $pkg = 'CodingAdventures::Code39';

# ============================================================================
# Pattern table
# ============================================================================

subtest 'PATTERNS table' => sub {
  my $pats = $pkg->patterns;
  is(ref $pats, 'HASH', 'patterns returns hash-ref');
  is(scalar keys %$pats, 44, '44 supported characters');

  for my $ch (keys %$pats) {
    is(length($pats->{$ch}), 9, "pattern for '$ch' is 9 chars");
  }

  is($pats->{'0'}, 'bwbWBwBwb', "pattern for '0'");
  is($pats->{'A'}, 'BwbwbWbwB', "pattern for 'A'");
  is($pats->{'*'}, 'bWbwBwBwb', "pattern for '*'");
  is($pats->{'-'}, 'bWbwbwBwB', "pattern for '-'");
};

subtest 'every pattern has 3 wide elements' => sub {
  my $pats = $pkg->patterns;
  for my $ch (keys %$pats) {
    my $raw  = $pats->{$ch};
    my $wide = () = $raw =~ /[A-Z]/g;
    is($wide, 3, "pattern for '$ch' has 3 wide elements");
  }
};

subtest 'patterns start with a bar' => sub {
  my $pats = $pkg->patterns;
  for my $ch (keys %$pats) {
    my $first = substr($pats->{$ch}, 0, 1);
    ok($first eq 'b' || $first eq 'B', "pattern for '$ch' starts with bar");
  }
};

# ============================================================================
# normalize_code39
# ============================================================================

subtest 'normalize_code39' => sub {
  is($pkg->normalize_code39('HELLO'),    'HELLO',    'uppercase unchanged');
  is($pkg->normalize_code39('hello'),    'HELLO',    'lowercase converted');
  is($pkg->normalize_code39('hello123'), 'HELLO123', 'mixed converted');
  is($pkg->normalize_code39(''),         '',         'empty string');
  is($pkg->normalize_code39('0123456789'), '0123456789', 'all digits');
  is($pkg->normalize_code39('-. $/+%'), '-. $/+%', 'special chars');

  ok( dies { $pkg->normalize_code39('*')      }, 'star rejected' );
  ok( dies { $pkg->normalize_code39('ABC*')   }, 'star in middle rejected' );
  ok( dies { $pkg->normalize_code39('@')      }, '@ rejected' );
  ok( dies { $pkg->normalize_code39('!')      }, '! rejected' );
  ok( dies { $pkg->normalize_code39(';')      }, '; rejected' );
};

# ============================================================================
# encode_code39_char
# ============================================================================

subtest 'encode_code39_char' => sub {
  my $enc = $pkg->encode_code39_char('A');
  is(ref $enc, 'HASH', 'returns hash-ref');
  is($enc->{char}, 'A', 'char field');
  ok(!$enc->{is_start_stop}, 'A is not start/stop');
  is(length($enc->{pattern}), 9, 'pattern is 9 chars');
  ok($enc->{pattern} =~ /^[NW]+$/, 'pattern contains only N/W');

  my $star = $pkg->encode_code39_char('*');
  ok($star->{is_start_stop}, '* is start/stop');

  # Pattern for A: BwbwbWbwB → W=1,N=2,N=3,N=4,N=5,W=6,N=7,N=8,W=9
  is(substr($enc->{pattern}, 0, 1), 'W', 'A[0] = W (wide bar)');
  is(substr($enc->{pattern}, 5, 1), 'W', 'A[5] = W (wide space)');
  is(substr($enc->{pattern}, 8, 1), 'W', 'A[8] = W (wide bar)');

  my $wide_count = () = $enc->{pattern} =~ /W/g;
  is($wide_count, 3, 'A has 3 wide elements');

  ok( dies { $pkg->encode_code39_char('@') }, 'invalid char dies' );
};

# ============================================================================
# encode_code39
# ============================================================================

subtest 'encode_code39' => sub {
  my $enc = $pkg->encode_code39('A');
  is(ref $enc, 'ARRAY', 'returns array-ref');
  is(scalar @$enc, 3, 'single char produces 3 encoded chars (*/A/*)');
  is($enc->[0]{char}, '*', 'first is start *');
  is($enc->[1]{char}, 'A', 'middle is A');
  is($enc->[2]{char}, '*', 'last is stop *');

  ok($enc->[0]{is_start_stop}, 'first is_start_stop');
  ok(!$enc->[1]{is_start_stop}, 'middle not is_start_stop');
  ok($enc->[2]{is_start_stop}, 'last is_start_stop');

  my $hello = $pkg->encode_code39('HELLO123');
  is(scalar @$hello, 10, 'HELLO123 → 10 encoded chars');

  # lowercase conversion
  my $lower = $pkg->encode_code39('hello');
  is($lower->[1]{char}, 'H', 'lowercase h becomes H');

  # empty input
  my $empty = $pkg->encode_code39('');
  is(scalar @$empty, 2, 'empty → 2 encoded chars');
};

# ============================================================================
# expand_code39_runs
# ============================================================================

subtest 'expand_code39_runs' => sub {
  my $runs = $pkg->expand_code39_runs('A');
  is(ref $runs, 'ARRAY', 'returns array-ref');
  is(scalar @$runs, 29, '*/A/* → 29 runs (9+1+9+1+9)');

  # Check all required fields
  for my $run (@$runs) {
    ok($run->{color} eq 'bar' || $run->{color} eq 'space', 'color is bar or space');
    ok($run->{width} eq 'narrow' || $run->{width} eq 'wide', 'width is narrow or wide');
    ok(defined $run->{source_char}, 'source_char defined');
    ok(defined $run->{source_index}, 'source_index defined');
    ok(defined $run->{is_inter_character_gap}, 'is_inter_character_gap defined');
  }

  # First run is a bar
  is($runs->[0]{color}, 'bar', 'first run is a bar');

  # Alternation of bar/space in first char
  my @expected = ('bar','space','bar','space','bar','space','bar','space','bar');
  for my $i (0..8) {
    is($runs->[$i]{color}, $expected[$i], "color at position $i");
  }

  # Inter-character gaps
  my $gap_count = scalar grep { $_->{is_inter_character_gap} } @$runs;
  is($gap_count, 2, '2 inter-character gaps for A (between */A and A/*)');

  # Last run is not an inter-character gap
  ok(!$runs->[-1]{is_inter_character_gap}, 'last run is not a gap');

  # All inter-character gaps are narrow spaces
  for my $run (@$runs) {
    if ($run->{is_inter_character_gap}) {
      is($run->{color}, 'space', 'gap color=space');
      is($run->{width}, 'narrow', 'gap width=narrow');
    }
  }

  # Two-char input: * A B * → 9+1+9+1+9+1+9 = 39 runs
  my $two = $pkg->expand_code39_runs('AB');
  is(scalar @$two, 39, 'AB → 39 runs');
};

# ============================================================================
# draw_code39
# ============================================================================

subtest 'draw_code39' => sub {
  my $scene = $pkg->draw_code39('A');
  is(ref $scene, 'HASH', 'returns hash-ref');
  ok(defined $scene->{svg},    'has svg');
  ok(defined $scene->{width},  'has width');
  ok(defined $scene->{height}, 'has height');
  is($scene->{symbology}, 'code39', 'symbology=code39');
  is($scene->{data}, 'A', 'data=A');

  # SVG structure
  ok($scene->{svg} =~ /<svg/, 'svg contains <svg>');
  ok($scene->{svg} =~ /<rect/, 'svg contains rects');
  ok($scene->{svg} =~ m{</svg>}, 'svg is closed');

  # Width includes quiet zones
  my $cfg = { narrow_unit=>4, wide_unit=>12, bar_height=>100,
              quiet_zone_units=>10, include_human_readable_text=>0 };
  my $s = $pkg->draw_code39('A', $cfg);
  ok($s->{width} > 80, 'width includes quiet zones (2 x 10 x 4 = 80)');

  # Larger narrow_unit → wider output
  my $s1 = $pkg->draw_code39('A', {%$cfg, narrow_unit=>2, wide_unit=>6});
  my $s2 = $pkg->draw_code39('A', {%$cfg, narrow_unit=>4, wide_unit=>12});
  ok($s2->{width} > $s1->{width}, 'larger narrow_unit → wider output');

  # Text label in SVG when enabled
  my $with_text = $pkg->draw_code39('HELLO', {narrow_unit=>4, wide_unit=>12,
    bar_height=>100, quiet_zone_units=>5, include_human_readable_text=>1});
  ok($with_text->{svg} =~ /HELLO/, 'SVG contains text label');

  # Normalizes input
  ok( lives { $pkg->draw_code39('hello') }, 'lowercase input accepted' );

  # Raises for invalid character
  ok( dies { $pkg->draw_code39('ABC@DEF') }, 'invalid char raises' );
};

# ============================================================================
# compute_checksum
# ============================================================================

subtest 'compute_checksum' => sub {
  my $c = $pkg->compute_checksum('HELLO');
  is(length($c), 1, 'checksum is 1 character');

  # "1" has value 1; 1 mod 43 = 1 → "1"
  is($pkg->compute_checksum('1'), '1', 'checksum of "1" is "1"');

  # "0" has value 0; 0 mod 43 = 0 → "0"
  is($pkg->compute_checksum('0'), '0', 'checksum of "0" is "0"');

  # KL: K=20, L=21 → 41 → chars[41] = '+' (0-indexed: pos41 in 0-based = '+')
  is($pkg->compute_checksum('KL'), '+', 'checksum of KL is +');

  # Result must be in the Code 39 alphabet
  my $alphabet = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%';
  for my $test ('HELLO', '123', 'ABC', 'TEST') {
    my $ch = $pkg->compute_checksum($test);
    ok(index($alphabet, $ch) >= 0, "checksum of '$test' is in alphabet");
  }

  ok( dies { $pkg->compute_checksum('@') }, 'invalid char dies' );
};

done_testing;
