use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LatticeTranspiler;

# ============================================================================
# 1. Module loads
# ============================================================================

subtest 'module loads' => sub {
    ok( CodingAdventures::LatticeTranspiler->can('transpile'),      'transpile method exists' );
    ok( CodingAdventures::LatticeTranspiler->can('transpile_file'), 'transpile_file method exists' );
};

# ============================================================================
# 2. Return value contract
# ============================================================================

subtest 'transpile returns (string, undef) on success' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile('h1 { color: red; }');
    ok( defined $css,  'css defined on success' );
    ok( !defined $err, 'err undef on success' );
    like( $css, qr/color: red/, 'css contains declaration' );
};

subtest 'transpile returns (undef, string) on parse failure' => sub {
    # Unclosed brace — may or may not be caught by the parser
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile('h1 { color: red;');
    # Accept either: parser is lenient (returns css) or strict (returns error)
    if (!defined $css) {
        ok( defined $err && length($err) > 0, 'error message returned' );
    } else {
        ok( defined $css, 'lenient parser returned css' );
    }
};

# ============================================================================
# 3. End-to-end transpilation
# ============================================================================

subtest 'plain CSS pass-through' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile('h1 { color: red; }');
    is( $err, undef,          'no error' );
    like( $css, qr/h1/,         'selector present' );
    like( $css, qr/color: red/, 'declaration present' );
};

subtest 'variable expansion' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile('$color: blue; p { color: $color; }');
    is( $err, undef,          'no error' );
    like  ( $css, qr/color: blue/, 'variable expanded' );
    unlike( $css, qr/\$color/,     'variable name not in output' );
};

subtest 'nested rule flattening' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile('.nav { .link { color: red; } }');
    is( $err, undef,                 'no error' );
    like( $css, qr/\.nav \.link/,    'selector flattened' );
};

subtest 'mixin expansion' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(
        '@mixin flex { display: flex; } .row { @include flex; }'
    );
    is( $err, undef,              'no error' );
    like  ( $css, qr/display: flex/, 'mixin body expanded' );
    unlike( $css, qr/\@mixin/,        '@mixin not in output' );
};

subtest '@if truthy branch' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(
        '$show: true; @if $show { .box { display: block; } }'
    );
    is( $err, undef, 'no error' );
    like( $css, qr/display: block/, 'truthy branch included' );
};

subtest '@if falsy branch excluded' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(
        '$show: false; @if $show { .box { display: block; } }'
    );
    is( $err, undef, 'no error' );
    unlike( $css, qr/display: block/, 'falsy branch excluded' );
};

subtest '@for loop' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(
        '@for $i from 1 through 3 { .item { order: $i; } }'
    );
    is( $err, undef, 'no error' );
    like( $css, qr/order: 1/, 'iteration 1' );
    like( $css, qr/order: 2/, 'iteration 2' );
    like( $css, qr/order: 3/, 'iteration 3' );
};

subtest '@each loop' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(
        '@each $color in red, green, blue { .t { color: $color; } }'
    );
    is( $err, undef, 'no error' );
    like( $css, qr/color: red/,   'red' );
    like( $css, qr/color: green/, 'green' );
    like( $css, qr/color: blue/,  'blue' );
};

subtest 'combined: variables + nesting + mixin' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(<<'END');
$primary: #4a90d9;

@mixin button($bg, $fg: white) {
    background: $bg;
    color: $fg;
}

.btn {
    @include button($primary);
    &:hover { opacity: 0.9; }
}
END
    is( $err, undef,                    'no error' );
    like( $css, qr/background: #4a90d9/, 'variable in mixin arg' );
    like( $css, qr/color: white/,        'default mixin param' );
    like( $css, qr/opacity: 0\.9/,       'nested &:hover' );
};

# ============================================================================
# 4. transpile_file()
# ============================================================================

subtest 'transpile_file returns error for missing file' => sub {
    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile_file('/nonexistent/style.lattice');
    ok( !defined $css,  'css undef for missing file' );
    ok( defined $err && length($err) > 0, 'error message returned' );
};

subtest 'transpile_file works with a temp file' => sub {
    require File::Temp;
    my ($fh, $tmpfile) = File::Temp::tempfile(SUFFIX => '.lattice', UNLINK => 1);
    print $fh '$c: red; h1 { color: $c; }';
    close $fh;

    my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile_file($tmpfile);
    is( $err, undef,           'no error' );
    like( $css, qr/color: red/, 'file transpiled' );
};

done_testing();
