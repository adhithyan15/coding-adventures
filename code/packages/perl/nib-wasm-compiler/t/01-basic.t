use strict;
use warnings;
use File::Temp qw(tempdir);
use Test2::V0;

use CodingAdventures::NibWasmCompiler qw(compile_source pack_source write_wasm_file);
use CodingAdventures::WasmRuntime;

subtest 'compile_source returns pipeline artifacts' => sub {
    my $result = compile_source(<<'NIB');
fn answer() -> u4 {
    return 7;
}
NIB

    ok($result->{typed_ast}, 'typed AST captured');
    ok($result->{raw_ir}, 'raw IR captured');
    ok($result->{module}, 'module hash captured');
    ok(length($result->{binary}) > 8, 'binary bytes emitted');
};

subtest 'pack_source aliases compile_source' => sub {
    my $compiled = compile_source('fn answer() -> u4 { return 7; }');
    my $packed = pack_source('fn answer() -> u4 { return 7; }');

    is($packed->{binary}, $compiled->{binary}, 'pack_source matches compile_source');
};

subtest 'write_wasm_file persists the compiled binary' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $path = $dir . '/program.wasm';

    my $result = write_wasm_file('fn answer() -> u4 { return 7; }', $path);

    ok(-f $path, 'output file written');
    open my $fh, '<:raw', $path or die "unable to read '$path': $!";
    local $/;
    my $bytes = <$fh>;
    close $fh;

    is($bytes, $result->{binary}, 'written bytes match compile result');
};

subtest 'compiled functions run in the Perl Wasm runtime' => sub {
    my $result = compile_source(<<'NIB');
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() -> u4 {
    return add(3, 4);
}
NIB

    my $runtime = CodingAdventures::WasmRuntime->new();
    my $runtime_result = $runtime->load_and_run($result->{binary}, 'main', []);
    is($runtime_result, [7], 'main() returns 7');
};

subtest 'compiled loops run in the Perl Wasm runtime' => sub {
    my $result = compile_source(<<'NIB');
fn count_to(n: u4) -> u4 {
    let acc: u4 = 0;
    for i: u4 in 0..n {
        acc = acc +% 1;
    }
    return acc;
}
NIB

    my $runtime = CodingAdventures::WasmRuntime->new();
    my $runtime_result = $runtime->load_and_run($result->{binary}, 'count_to', [5]);
    is($runtime_result, [5], 'count_to(5) returns 5');
};

subtest 'type errors raise package errors with stage metadata' => sub {
    my $error;
    eval { compile_source('fn main() { let x: bool = 1 +% 2; }'); 1 } or $error = $@;

    isa_ok($error, 'CodingAdventures::NibWasmCompiler::PackageError');
    is($error->stage, 'type-check', 'stage identifies the failing pass');
};

subtest 'parse errors raise package errors with stage metadata' => sub {
    my $compiler = CodingAdventures::NibWasmCompiler->new();
    my $error;
    eval { $compiler->compile_source('fn main('); 1 } or $error = $@;

    isa_ok($error, 'CodingAdventures::NibWasmCompiler::PackageError');
    is($error->stage, 'parse', 'stage identifies the parser');
};

done_testing;
