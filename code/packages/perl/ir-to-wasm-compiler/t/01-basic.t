use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Brainfuck::Parser;
use CodingAdventures::BrainfuckIrCompiler qw(compile);
use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
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

done_testing;
