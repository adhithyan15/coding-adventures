use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::IrToWasmCompiler qw(new_function_signature);
use CodingAdventures::IrToWasmValidator qw(validate);

sub _program_with {
    my ($instructions) = @_;
    my $program = CodingAdventures::CompilerIr::IrProgram->new('_start');
    for my $instruction (@$instructions) {
        $program->add_instruction($instruction);
    }
    return $program;
}

subtest 'returns no diagnostics for lowerable IR' => sub {
    my $program = _program_with([
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::LABEL,
            operands => [CodingAdventures::CompilerIr::IrLabel->new('_start')],
            id       => -1,
        ),
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
            operands => [
                CodingAdventures::CompilerIr::IrRegister->new(1),
                CodingAdventures::CompilerIr::IrImmediate->new(7),
            ],
            id => 0,
        ),
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::HALT,
            operands => [],
            id       => 1,
        ),
    ]);

    is(validate($program, [new_function_signature('_start', 0, '_start')]), [], 'validator accepts lowerable IR');
};

subtest 'reports lowering failures as diagnostics' => sub {
    my $program = _program_with([
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::LABEL,
            operands => [CodingAdventures::CompilerIr::IrLabel->new('_start')],
            id       => -1,
        ),
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::SYSCALL,
            operands => [CodingAdventures::CompilerIr::IrImmediate->new(999)],
            id       => 0,
        ),
    ]);

    my $errors = validate($program, [new_function_signature('_start', 0, '_start')]);
    ok(@$errors >= 1, 'validator emits at least one diagnostic');
    like($errors->[0]{message}, qr/unsupported SYSCALL number/i, 'error message identifies the unsupported syscall');
};

done_testing;
