use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Display;

# ---------------------------------------------------------------------------
# Helper: make a fresh driver with zeroed memory
# ---------------------------------------------------------------------------
sub make_driver {
    my ($cols, $rows) = @_;
    $cols //= 80;
    $rows //= 25;
    my $cfg = CodingAdventures::Display::DisplayConfig->new({
        columns => $cols,
        rows    => $rows,
    });
    my @mem = (0) x ($cols * $rows * 2);
    my $drv = CodingAdventures::Display::DisplayDriver->new($cfg, \@mem);
    return $drv;
}

# ---------------------------------------------------------------------------
# 1. VERSION
# ---------------------------------------------------------------------------
is(CodingAdventures::Display->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# 2. DisplayConfig
# ---------------------------------------------------------------------------
my $cfg = CodingAdventures::Display::DisplayConfig->new({ columns => 80, rows => 25 });
is($cfg->columns, 80, 'DisplayConfig columns accessor');
is($cfg->rows,    25, 'DisplayConfig rows accessor');

my $default_cfg = CodingAdventures::Display::DisplayConfig->new();
is($default_cfg->columns, 80, 'default columns is 80');
is($default_cfg->rows,    25, 'default rows is 25');

# ---------------------------------------------------------------------------
# 3. Initial cursor position
# ---------------------------------------------------------------------------
my $drv = make_driver(80, 25);
is($drv->cursor_row, 0, 'initial cursor_row is 0');
is($drv->cursor_col, 0, 'initial cursor_col is 0');

# ---------------------------------------------------------------------------
# 4. puts_str simple text
# ---------------------------------------------------------------------------
$drv->puts_str("Hi");
my $snap = $drv->snapshot;
is($snap->{lines}[0], 'Hi', 'puts_str writes text to row 0');
is($snap->{lines}[1], '',   'row 1 is empty after two chars');
is($drv->cursor_col, 2, 'cursor advances 2 after "Hi"');
is($drv->cursor_row, 0, 'cursor still on row 0');

# ---------------------------------------------------------------------------
# 5. Newline (0x0A)
# ---------------------------------------------------------------------------
my $drv2 = make_driver(80, 25);
$drv2->puts_str("Hello");
$drv2->put_char(0x0A);
$drv2->puts_str("World");
my $snap2 = $drv2->snapshot;
is($snap2->{lines}[0], 'Hello', 'line 0 is Hello');
is($snap2->{lines}[1], 'World', 'line 1 is World after newline');
is($drv2->cursor_row, 1, 'cursor on row 1 after newline + text');

# ---------------------------------------------------------------------------
# 6. Carriage return (0x0D)
# ---------------------------------------------------------------------------
my $drv3 = make_driver(80, 25);
$drv3->puts_str("ABC");
$drv3->put_char(0x0D);   # CR -> col=0
$drv3->puts_str("XY");   # overwrite first two chars
my $snap3 = $drv3->snapshot;
is($snap3->{lines}[0], 'XYC', 'CR overwrites from col 0');
is($drv3->cursor_col, 2, 'cursor col is 2 after CR + "XY"');

# ---------------------------------------------------------------------------
# 7. Tab (0x09)
# ---------------------------------------------------------------------------
my $drv4 = make_driver(80, 25);
$drv4->puts_str("A");     # col=1
$drv4->put_char(0x09);    # tab -> next multiple of 8 = col 8
is($drv4->cursor_col, 8, 'tab stops at column 8');
$drv4->puts_str("B");     # col=9
$drv4->put_char(0x09);    # next stop = 16
is($drv4->cursor_col, 16, 'tab stops at column 16');

# ---------------------------------------------------------------------------
# 8. Backspace (0x08)
# ---------------------------------------------------------------------------
my $drv5 = make_driver(80, 25);
$drv5->puts_str("ABC");   # col=3
$drv5->put_char(0x08);    # col=2
is($drv5->cursor_col, 2, 'backspace moves cursor left');
$drv5->puts_str("X");     # overwrite col 2 (was 'C')
my $snap5 = $drv5->snapshot;
is($snap5->{lines}[0], 'ABX', 'backspace + write overwrites char');

# Backspace at col 0 does not go negative
my $drv6 = make_driver(80, 25);
$drv6->put_char(0x08);
is($drv6->cursor_col, 0, 'backspace at col 0 stays at 0');

# ---------------------------------------------------------------------------
# 9. Line wrap
# ---------------------------------------------------------------------------
my $drv7 = make_driver(5, 3);   # tiny 5-col display
$drv7->puts_str("ABCDE");  # fills row 0, cursor wraps to row 1 col 0
is($drv7->cursor_row, 1, 'cursor wraps to next row after filling a row');
is($drv7->cursor_col, 0, 'cursor col is 0 after wrap');
my $snap7 = $drv7->snapshot;
is($snap7->{lines}[0], 'ABCDE', 'row 0 is filled after wrap');

# ---------------------------------------------------------------------------
# 10. Scrolling
# ---------------------------------------------------------------------------
# Use a 10-col x 3-row display and write each line via newline to avoid
# wrap-induced cursor confusion.
my $drv8 = make_driver(10, 3);
$drv8->puts_str("ROW0");  $drv8->put_char(0x0A);   # row 0
$drv8->puts_str("ROW1");  $drv8->put_char(0x0A);   # row 1
$drv8->puts_str("ROW2");  $drv8->put_char(0x0A);   # triggers scroll
$drv8->puts_str("ROW3");                             # now on last row
my $snap8 = $drv8->snapshot;
is($snap8->{lines}[0], 'ROW1', 'after scroll: row 0 has old row 1');
is($snap8->{lines}[1], 'ROW2', 'after scroll: row 1 has old row 2');
is($snap8->{lines}[2], 'ROW3', 'after scroll: row 2 has new line');

# ---------------------------------------------------------------------------
# 11. clear()
# ---------------------------------------------------------------------------
my $drv9 = make_driver(80, 25);
$drv9->puts_str("Hello");
$drv9->put_char(0x0A);
$drv9->puts_str("World");
$drv9->clear;
is($drv9->cursor_row, 0, 'clear resets cursor row');
is($drv9->cursor_col, 0, 'clear resets cursor col');
my $snap9 = $drv9->snapshot;
is($snap9->{lines}[0], '', 'clear empties row 0');
is($snap9->{lines}[1], '', 'clear empties row 1');

# ---------------------------------------------------------------------------
# 12. snapshot strips trailing spaces
# ---------------------------------------------------------------------------
my $drv10 = make_driver(10, 5);
$drv10->puts_str("AB");
my $snap10 = $drv10->snapshot;
is($snap10->{lines}[0], 'AB', 'snapshot strips trailing spaces');
is($snap10->{lines}[1], '', 'empty line is empty string not spaces');

# ---------------------------------------------------------------------------
# 13. snapshot line count matches rows
# ---------------------------------------------------------------------------
my $drv11 = make_driver(20, 5);
my $snap11 = $drv11->snapshot;
is(scalar @{$snap11->{lines}}, 5, 'snapshot has exactly rows lines');

# ---------------------------------------------------------------------------
# 14. put_char individual characters
# ---------------------------------------------------------------------------
my $drv12 = make_driver(80, 25);
$drv12->put_char(ord('H'));
$drv12->put_char(ord('i'));
my $snap12 = $drv12->snapshot;
is($snap12->{lines}[0], 'Hi', 'put_char with ord values works');

# ---------------------------------------------------------------------------
# 15. Multi-line snapshot
# ---------------------------------------------------------------------------
my $drv13 = make_driver(80, 25);
$drv13->puts_str("Line one");
$drv13->put_char(0x0A);
$drv13->puts_str("Line two");
$drv13->put_char(0x0A);
$drv13->puts_str("Line three");
my $snap13 = $drv13->snapshot;
is($snap13->{lines}[0], 'Line one',   'snapshot line 0');
is($snap13->{lines}[1], 'Line two',   'snapshot line 1');
is($snap13->{lines}[2], 'Line three', 'snapshot line 2');

done_testing;
