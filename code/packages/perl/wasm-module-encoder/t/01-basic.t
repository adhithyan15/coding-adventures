use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmModuleEncoder qw(encode_module);
use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmValidator qw(validate);

subtest 'encodes a minimal exported function module' => sub {
    my $bytes = encode_module({
        types     => [{ params => [], results => [0x7F] }],
        functions => [0],
        exports   => [{ name => 'answer', desc => { kind => 'func', idx => 0 } }],
        codes     => [{ locals => [], body => "\x41\x07\x0f\x0b" }],
    });

    is(substr($bytes, 0, 4), "\x00asm", 'module starts with WASM magic');

    my $parsed = parse($bytes);
    is($parsed->{exports}[0]{name}, 'answer', 'export survives round-trip');
    ok(validate($parsed), 'encoded module validates');
};

subtest 'accepts alias keys used elsewhere in the Perl stack' => sub {
    my $bytes = encode_module({
        types   => [{ params => [], results => [0x7F] }],
        imports => [
            {
                mod  => 'wasi_snapshot_preview1',
                name => 'proc_exit',
                desc => { kind => 'func', type_idx => 0 },
            },
        ],
        functions => [0],
        exports   => [{ name => 'answer', desc => { kind => 'func', idx => 1 } }],
        code      => [{ locals => [], code => "\x41\x07\x0f\x0b" }],
    });

    my $parsed = parse($bytes);
    is($parsed->{imports}[0]{mod}, 'wasi_snapshot_preview1', 'module alias encoded');
    is($parsed->{imports}[0]{desc}{type_idx}, 0, 'type_idx alias encoded');
};

done_testing;
