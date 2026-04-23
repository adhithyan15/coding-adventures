use strict;
use warnings;
use File::Temp qw(tempdir);
use Test2::V0;

use CodingAdventures::BrainfuckWasmCompiler qw(compile_source pack_source write_wasm_file);
use CodingAdventures::WasmRuntime;

sub _run_binary {
    my ($binary, $stdin) = @_;
    my @output;
    my $offset = 0;
    my $host = CodingAdventures::WasmRuntime::WasiHost->new(
        stdout => sub { push @output, $_[0] },
        stdin  => sub {
            my ($count) = @_;
            my $chunk = substr($stdin // q{}, $offset, $count);
            $offset += length $chunk;
            return $chunk;
        },
    );

    my $runtime = CodingAdventures::WasmRuntime->new(host => $host);
    my $result = $runtime->load_and_run($binary, '_start', []);
    return ($result, \@output);
}

subtest 'compile_source returns pipeline artifacts' => sub {
    my $result = compile_source('+.');

    ok($result->{raw_ir}, 'raw IR captured');
    ok($result->{optimized_ir}, 'optimized IR captured');
    ok($result->{module}, 'module hash captured');
    ok(length($result->{binary}) > 8, 'binary bytes emitted');
    is($result->{filename}, 'program.bf', 'default filename recorded');
};

subtest 'pack_source aliases compile_source' => sub {
    my $compiled = compile_source('+.');
    my $packed = pack_source('+.');

    is($packed->{binary}, $compiled->{binary}, 'pack_source reuses compile_source');
};

subtest 'write_wasm_file persists the compiled binary' => sub {
    my $dir = tempdir(CLEANUP => 1);
    my $path = $dir . '/program.wasm';

    my $result = write_wasm_file('+.', $path);

    ok(-f $path, 'output file written');
    open my $fh, '<:raw', $path or die "unable to read '$path': $!";
    local $/;
    my $bytes = <$fh>;
    close $fh;

    is($bytes, $result->{binary}, 'written bytes match compile result');
};

subtest 'compiled output programs run in the Perl Wasm runtime' => sub {
    my $result = compile_source(('+' x 65) . '.');
    my ($runtime_result, $output) = _run_binary($result->{binary}, q{});

    is($runtime_result, [0], 'Brainfuck program exits cleanly');
    is($output, ['A'], 'stdout receives the emitted character');
};

subtest 'compiled cat programs reuse stdin and stdout through WASI' => sub {
    my $result = compile_source(',[.,]');
    my ($runtime_result, $output) = _run_binary($result->{binary}, 'Hi');

    is($runtime_result, [0], 'cat program exits cleanly');
    is($output, ['H', 'i'], 'stdin bytes are echoed back out');
};

subtest 'compiler instances honor custom filenames' => sub {
    my $compiler = CodingAdventures::BrainfuckWasmCompiler->new(filename => 'hello.bf');
    my $result = $compiler->compile_source('+');

    is($result->{filename}, 'hello.bf', 'custom filename preserved');
};

done_testing;
