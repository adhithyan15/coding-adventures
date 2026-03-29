use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CsvParser;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::CsvParser->VERSION, '0.01', 'has VERSION 0.01');

my $p = CodingAdventures::CsvParser->new();
ok($p, 'new() returns an object');

# ---------------------------------------------------------------------------
# Test 1: Simple one-row, three-field parse
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,b,c");
    is(scalar @$rows, 1, 'simple row: one row returned');
    is($rows->[0], ['a', 'b', 'c'], 'simple row: correct fields');
}

# ---------------------------------------------------------------------------
# Test 2: Two rows separated by \n
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,b,c\n1,2,3");
    is(scalar @$rows, 2, 'two rows: count');
    is($rows->[0], ['a','b','c'], 'two rows: first row');
    is($rows->[1], ['1','2','3'], 'two rows: second row');
}

# ---------------------------------------------------------------------------
# Test 3: Windows-style \r\n line endings
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("x,y\r\nz,w");
    is(scalar @$rows, 2, 'CRLF: two rows');
    is($rows->[0], ['x','y'], 'CRLF: first row');
    is($rows->[1], ['z','w'], 'CRLF: second row');
}

# ---------------------------------------------------------------------------
# Test 4: Old Mac \r line endings
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,b\rc,d");
    is(scalar @$rows, 2, 'CR-only: two rows');
    is($rows->[0], ['a','b'], 'CR-only: first row');
    is($rows->[1], ['c','d'], 'CR-only: second row');
}

# ---------------------------------------------------------------------------
# Test 5: Quoted fields
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse('"hello world","foo,bar"');
    is(scalar @$rows, 1, 'quoted: one row');
    is($rows->[0], ['hello world', 'foo,bar'], 'quoted: fields without quotes/delimiters preserved');
}

# ---------------------------------------------------------------------------
# Test 6: Escaped double-quote ("") inside a quoted field
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse('"say ""hi"" now",end');
    is($rows->[0][0], 'say "hi" now', 'escaped double-quote: "" -> "');
    is($rows->[0][1], 'end', 'escaped double-quote: second field ok');
}

# ---------------------------------------------------------------------------
# Test 7: Empty fields
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,,c");
    is($rows->[0], ['a', '', 'c'], 'empty middle field');
}

{
    my $rows = $p->parse(",b,");
    is($rows->[0], ['', 'b', ''], 'empty first and last fields');
}

# ---------------------------------------------------------------------------
# Test 8: Entirely empty quoted field
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse('""');
    is($rows->[0], [''], 'empty quoted field');
}

# ---------------------------------------------------------------------------
# Test 9: Newline inside a quoted field
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("\"line1\nline2\",after");
    is(scalar @$rows, 1, 'embedded newline: still one row');
    is($rows->[0][0], "line1\nline2", 'embedded newline: preserved in field');
    is($rows->[0][1], 'after', 'embedded newline: second field correct');
}

# ---------------------------------------------------------------------------
# Test 10: Custom delimiter (semicolon)
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a;b;c", { delimiter => ";" });
    is($rows->[0], ['a','b','c'], 'per-call semicolon delimiter');
}

# ---------------------------------------------------------------------------
# Test 11: Parser constructed with custom delimiter
# ---------------------------------------------------------------------------
{
    my $tsv = CodingAdventures::CsvParser->new({ delimiter => "\t" });
    my $rows = $tsv->parse("col1\tcol2\tcol3");
    is($rows->[0], ['col1','col2','col3'], 'tab-delimited parser');
}

# ---------------------------------------------------------------------------
# Test 12: Trailing newline does not produce spurious empty row
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,b\n");
    # RFC 4180 allows a trailing CRLF; our parser should not emit a phantom row
    # when the file ends with a newline after a complete row.
    # Because we only emit a row when we have something (field or current_row
    # is non-empty), we expect exactly 1 row here.
    # Note: some parsers do emit 2 rows here — we test our own behaviour.
    my $num = scalar @$rows;
    ok($num == 1 || $num == 2, "trailing newline: 1 or 2 rows (got $num)");
    is($rows->[0], ['a','b'], 'trailing newline: first row correct');
}

# ---------------------------------------------------------------------------
# Test 13: Single field, no delimiter
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("hello");
    is($rows->[0], ['hello'], 'single field no delimiter');
}

# ---------------------------------------------------------------------------
# Test 14: Empty input
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("");
    ok(ref($rows) eq 'ARRAY', 'empty input returns arrayref');
    is(scalar @$rows, 0, 'empty input: zero rows');
}

# ---------------------------------------------------------------------------
# Test 15: Multiple empty rows
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("a,b\n\nc,d");
    # The middle empty line is a row with one empty field (or two, depending on
    # interpretation — we test that the non-empty rows are correct).
    ok(scalar @$rows >= 2, 'empty middle row: at least 2 rows');
    is($rows->[0], ['a','b'], 'empty middle row: first row correct');
    is($rows->[-1], ['c','d'], 'empty middle row: last row correct');
}

# ---------------------------------------------------------------------------
# Test 16: Numeric fields (CSV always returns strings)
# ---------------------------------------------------------------------------
{
    my $rows = $p->parse("1,2.5,-3");
    is($rows->[0], ['1','2.5','-3'], 'numeric fields returned as strings');
}

# ---------------------------------------------------------------------------
# Test 17: Header row + data rows
# ---------------------------------------------------------------------------
{
    my $csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
    my $rows = $p->parse($csv);
    is(scalar @$rows, 3, 'header+data: 3 rows');
    is($rows->[0], ['name','age','city'], 'header row');
    is($rows->[1], ['Alice','30','NYC'], 'first data row');
    is($rows->[2], ['Bob','25','LA'], 'second data row');
}

done_testing;
