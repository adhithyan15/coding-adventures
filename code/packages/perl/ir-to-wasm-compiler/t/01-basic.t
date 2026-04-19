use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Brainfuck::Parser;
use CodingAdventures::BrainfuckIrCompiler qw(compile);
use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::IrToWasmCompiler qw(new_function_signature);
use CodingAdventures::WasmModuleEncoder qw(encode_module);
use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmValidator qw(validate);

sub _minimal_program {
    my $program = CodingAdventures::CompilerIr::IrProgram->new('_start');
    $program->add_instruction(CodingAdventures::CompilerIr::IrInstruction->new(
        opcode   => CodingAdventures::CompilerIr::IrOp::LABEL,
        operands => [CodingAdventures::CompilerIr::IrLabel->new('_start')],
        id       => -1,
    ));
    $program->add_instruction(CodingAdventures::CompilerIr::IrInstruction->new(
        opcode   => CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
        operands => [
            CodingAdventures::CompilerIr::IrRegister->new(1),
            CodingAdventures::CompilerIr::IrImmediate->new(7),
        ],
        id => 0,
    ));
    $program->add_instruction(CodingAdventures::CompilerIr::IrInstruction->new(
        opcode   => CodingAdventures::CompilerIr::IrOp::HALT,
        operands => [],
        id       => 1,
    ));
    return $program;
}

subtest 'lowers a minimal IR program into a valid module' => sub {
    my $module = CodingAdventures::IrToWasmCompiler::compile(
        _minimal_program(),
        [new_function_signature('_start', 0, '_start')],
    );
    is($module->{functions}, [0], 'one function emitted');
    is($module->{exports}[0]{name}, '_start', 'entry export emitted');

    my $binary = encode_module($module);
    my $parsed = parse($binary);
    ok(validate($parsed), 'encoded module validates');
};

subtest 'brainfuck lowering requests wasi and linear memory' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse(',.');
    my $ir = compile(
        $ast,
        'stdin.bf',
        CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config,
    );

    my $module = CodingAdventures::IrToWasmCompiler::compile(
        $ir->{program},
        [new_function_signature('_start', 0, '_start')],
    );

    ok(@{ $module->{imports} } >= 2, 'read/write imports emitted');
    is($module->{imports}[0]{module}, 'wasi_snapshot_preview1', 'WASI module name recorded');
    ok(@{ $module->{memories} } >= 1, 'memory section emitted');
    ok(@{ $module->{data} } >= 1, 'data section emitted');
};

subtest 'rejects oversized function signatures before allocating Wasm types' => sub {
    my $error;
    eval {
        CodingAdventures::IrToWasmCompiler::compile(
            _minimal_program(),
            [{ label => '_start', param_count => 129, export_name => '_start' }],
        );
        1;
    } or $error = $@;

    like(
        "$error",
        qr/param_count must be a non-negative integer/,
        'oversized parameter counts fail closed',
    );
};

subtest 'rejects oversized data declarations before materializing data segments' => sub {
    my $program = _minimal_program();
    $program->add_data(CodingAdventures::CompilerIr::IrDataDecl->new(
        label => 'huge',
        size  => 16 * 1024 * 1024 + 1,
        init  => 0,
    ));

    my $error;
    eval {
        CodingAdventures::IrToWasmCompiler::compile(
            $program,
            [new_function_signature('_start', 0, '_start')],
        );
        1;
    } or $error = $@;

    like(
        "$error",
        qr/data declaration 'huge' size must be between 0 and/,
        'oversized data segments fail closed',
    );
};

subtest 'rejects malformed data declaration initializers before packing bytes' => sub {
    my $program = _minimal_program();
    $program->add_data(CodingAdventures::CompilerIr::IrDataDecl->new(
        label => 'wide',
        size  => 1,
        init  => 256,
    ));

    my $error;
    eval {
        CodingAdventures::IrToWasmCompiler::compile(
            $program,
            [new_function_signature('_start', 0, '_start')],
        );
        1;
    } or $error = $@;

    like(
        "$error",
        qr/data declaration 'wide' init must be a byte/,
        'out-of-range data initializers fail closed',
    );
};

done_testing;
