use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LatticeParser;
use CodingAdventures::LatticeAstToCss;

# ============================================================================
# Helper: parse Lattice source and compile to CSS
# ============================================================================

sub compile {
    my ($source) = @_;
    my $ast = CodingAdventures::LatticeParser->parse($source);
    return CodingAdventures::LatticeAstToCss->compile($ast);
}

# ============================================================================
# 1. Module loads
# ============================================================================

subtest 'module loads' => sub {
    ok( CodingAdventures::LatticeAstToCss->can('compile'), 'compile method exists' );
};

# ============================================================================
# 2. Plain CSS pass-through
# ============================================================================

subtest 'plain CSS pass-through' => sub {
    my $css = compile('h1 { color: red; }');
    like( $css, qr/h1/,         'selector present' );
    like( $css, qr/color: red/, 'declaration present' );
};

subtest 'class selector' => sub {
    my $css = compile('.card { background: white; }');
    like( $css, qr/\.card/,            'class selector present' );
    like( $css, qr/background: white/, 'value present' );
};

subtest 'empty source returns empty string' => sub {
    my $css = compile('');
    is( $css, '', 'empty input → empty output' );
};

subtest 'output ends with newline' => sub {
    my $css = compile('h1 { color: red; }');
    like( $css, qr/\n$/, 'ends with newline' );
};

# ============================================================================
# 3. Variable expansion
# ============================================================================

subtest 'variable expansion — simple' => sub {
    my $css = compile('$color: red; h1 { color: $color; }');
    like  ( $css, qr/color: red/, 'variable expanded' );
    unlike( $css, qr/\$color/,    'variable name not in output' );
};

subtest 'variable expansion — hex color' => sub {
    my $css = compile('$primary: #4a90d9; a { color: $primary; }');
    like( $css, qr/#4a90d9/, 'hex color variable expanded' );
};

subtest 'variable expansion — dimension' => sub {
    my $css = compile('$size: 16px; body { font-size: $size; }');
    like( $css, qr/font-size: 16px/, 'dimension variable expanded' );
};

subtest 'variable scoping inside block' => sub {
    my $css = compile('$color: red; .a { $color: blue; color: $color; }');
    like( $css, qr/color: blue/, 'inner variable shadows outer' );
};

# ============================================================================
# 4. Nested rule flattening
# ============================================================================

subtest 'nested rule flattening — one level' => sub {
    my $css = compile('.parent { .child { color: blue; } }');
    like( $css, qr/\.parent \.child/, 'selector flattened' );
    like( $css, qr/color: blue/,      'declaration present' );
};

subtest 'nested rule flattening — two levels' => sub {
    my $css = compile('.a { .b { .c { color: green; } } }');
    like( $css, qr/\.a \.b \.c/, 'selector doubly flattened' );
};

subtest 'parent reference &' => sub {
    my $css = compile('a { &:hover { color: red; } }');
    like( $css, qr/a:hover/, '& replaced with parent selector' );
};

subtest 'parent rule has own declarations' => sub {
    my $css = compile('.parent { color: black; .child { color: blue; } }');
    like( $css, qr/\.parent \{/, 'parent rule present' );
    like( $css, qr/color: black/, 'parent declaration present' );
    like( $css, qr/\.parent \.child/, 'nested selector flattened' );
};

# ============================================================================
# 5. Mixin expansion
# ============================================================================

subtest 'mixin with no parameters' => sub {
    my $css = compile('@mixin flex-center { display: flex; align-items: center; } .box { @include flex-center; }');
    like( $css, qr/display: flex/,       'mixin body expanded' );
    like( $css, qr/align-items: center/, 'mixin body expanded' );
};

subtest 'mixin with parameter' => sub {
    my $css = compile('@mixin color-box($bg) { background: $bg; } .red { @include color-box(red); }');
    like( $css, qr/background: red/, 'mixin parameter substituted' );
};

subtest 'mixin with default parameter' => sub {
    my $css = compile('@mixin button($bg, $fg: white) { background: $bg; color: $fg; } .btn { @include button(blue); }');
    like( $css, qr/background: blue/, 'explicit arg used' );
    like( $css, qr/color: white/,     'default arg used' );
};

subtest 'mixin definition not in output' => sub {
    my $css = compile('@mixin flex { display: flex; } .box { @include flex; }');
    unlike( $css, qr/@mixin/,   '@mixin not in CSS output' );
    unlike( $css, qr/@include/, '@include not in CSS output' );
};

# ============================================================================
# 6. @if control flow
# ============================================================================

subtest '@if truthy condition' => sub {
    my $css = compile('$debug: true; @if $debug { .debug { display: block; } }');
    like( $css, qr/display: block/, 'truthy @if block included' );
};

subtest '@if falsy condition' => sub {
    my $css = compile('$debug: false; @if $debug { .debug { display: block; } }');
    unlike( $css, qr/display: block/, 'falsy @if block excluded' );
};

subtest '@if/@else' => sub {
    my $css = compile('$theme: light; h1 { @if $theme == dark { color: white; } @else { color: black; } }');
    unlike( $css, qr/color: white/, 'dark branch excluded' );
    like  ( $css, qr/color: black/, 'else branch included' );
};

subtest '@if numeric comparison' => sub {
    my $css = compile('$n: 3; @if $n < 10 { .small { font-size: 12px; } }');
    like( $css, qr/font-size: 12px/, 'numeric comparison works' );
};

# ============================================================================
# 7. @for loop
# ============================================================================

subtest '@for through (inclusive)' => sub {
    my $css = compile('@for $i from 1 through 3 { .item { order: $i; } }');
    like( $css, qr/order: 1/, 'iteration 1' );
    like( $css, qr/order: 2/, 'iteration 2' );
    like( $css, qr/order: 3/, 'iteration 3' );
};

subtest '@for to (exclusive)' => sub {
    my $css = compile('@for $i from 1 to 3 { .col { z-index: $i; } }');
    like  ( $css, qr/z-index: 1/, 'iteration 1' );
    like  ( $css, qr/z-index: 2/, 'iteration 2' );
    unlike( $css, qr/z-index: 3/, 'iteration 3 excluded (exclusive)' );
};

# ============================================================================
# 8. @each loop
# ============================================================================

subtest '@each loop' => sub {
    my $css = compile('@each $color in red, green, blue { .text { color: $color; } }');
    like( $css, qr/color: red/,   'red iteration' );
    like( $css, qr/color: green/, 'green iteration' );
    like( $css, qr/color: blue/,  'blue iteration' );
};

# ============================================================================
# 9. @function evaluation
# ============================================================================

subtest '@function and call' => sub {
    my $css = compile('@function double($n) { @return $n * 2; } .box { width: double(8); }');
    like( $css, qr/width: 16/, 'function result substituted' );
};

# ============================================================================
# 10. Multi-rule stylesheets
# ============================================================================

subtest 'multiple top-level rules' => sub {
    my $css = compile('h1 { color: red; } h2 { color: blue; }');
    like( $css, qr/h1/,         'h1 present' );
    like( $css, qr/h2/,         'h2 present' );
    like( $css, qr/color: red/, 'h1 value' );
    like( $css, qr/color: blue/, 'h2 value' );
};

subtest 'variable used across multiple rules' => sub {
    my $css = compile('$brand: #ff6600; h1 { color: $brand; } a { border-color: $brand; }');
    my @matches = ($css =~ /#ff6600/g);
    ok( scalar @matches >= 2, 'brand color expanded in both rules' );
};

done_testing();
