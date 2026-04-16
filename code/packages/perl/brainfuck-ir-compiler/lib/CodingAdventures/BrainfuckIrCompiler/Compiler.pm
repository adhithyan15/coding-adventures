package CodingAdventures::BrainfuckIrCompiler::Compiler;

# ============================================================================
# CodingAdventures::BrainfuckIrCompiler::Compiler — Brainfuck AST → IR
# ============================================================================
#
# This module is the Brainfuck-specific frontend of the AOT compiler
# pipeline.  It knows Brainfuck semantics (tape, cells, pointer, loops,
# I/O) and translates them into target-independent IR instructions.
#
# ## Inputs
#
#   $ast      — the root AST node from CodingAdventures::Brainfuck::Parser
#   $filename — source file path (for source map entries)
#   $config   — a BuildConfig object
#
# ## Outputs
#
#   A hashref: { program => IrProgram, source_map => SourceMapChain }
#   Or dies with a descriptive error message.
#
# ## Register allocation
#
# Brainfuck needs very few registers:
#
#   v0 = tape base address  (pointer to start of tape)
#   v1 = tape pointer offset  (current cell index, 0-based)
#   v2 = temporary  (cell value for loads/stores)
#   v3 = temporary  (for bounds checks)
#   v4 = temporary  (for syscall arguments)
#   v5 = max pointer value  (tape_size - 1, for bounds checks)
#   v6 = zero constant  (for bounds checks lower-bound check)
#
# ## IR instruction sequences per Brainfuck command
#
#   Command  │ IR sequence
#   ─────────┼────────────────────────────────────────────────────────────
#   > (RIGHT) │ ADD_IMM v1, v1, 1
#   < (LEFT)  │ ADD_IMM v1, v1, -1
#   + (INC)   │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;
#             │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1
#   - (DEC)   │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;
#             │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1
#   . (OUTPUT)│ LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1
#   , (INPUT) │ SYSCALL 2; STORE_BYTE v4, v0, v1
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(compile);

use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IDGenerator;
use CodingAdventures::CompilerSourceMap::SourceMapChain;
use CodingAdventures::CompilerSourceMap::SourcePosition;

# ── Register indices ──────────────────────────────────────────────────────
use constant REG_TAPE_BASE => 0;  # v0: base address of the tape
use constant REG_TAPE_PTR  => 1;  # v1: current cell offset (0-based)
use constant REG_TEMP      => 2;  # v2: temporary for cell values
use constant REG_TEMP2     => 3;  # v3: temporary for bounds checks
use constant REG_SYS_ARG   => 4;  # v4: syscall argument register
use constant REG_MAX_PTR   => 5;  # v5: tape_size - 1 (for bounds checks)
use constant REG_ZERO      => 6;  # v6: constant 0 (for bounds checks)

# ── Syscall numbers (matching RISC-V simulator ecall dispatch) ────────────
use constant SYSCALL_WRITE => 1;   # write byte in a0 to stdout
use constant SYSCALL_READ  => 2;   # read byte from stdin into a0
use constant SYSCALL_EXIT  => 10;  # halt with exit code in a0

# ============================================================================
# Public API
# ============================================================================

# compile($ast, $filename, $config) — compile a Brainfuck AST into IR.
#
# $ast      — root hashref from CodingAdventures::Brainfuck::Parser->parse()
#             must have { type => 'program', children => [...], ... }
# $filename — string, used in source map entries
# $config   — BuildConfig object
#
# Returns a hashref: { program => IrProgram, source_map => SourceMapChain }
# Dies with a descriptive error on invalid input.
sub compile {
    my ($ast, $filename, $config) = @_;

    # Validate the root node
    unless (defined $ast && ref $ast eq 'HASH' && ($ast->{type} // '') eq 'program') {
        die sprintf(
            "BrainfuckIrCompiler: expected 'program' AST node, got '%s'",
            (defined $ast && ref $ast eq 'HASH') ? ($ast->{type} // 'undef') : 'undef'
        );
    }
    unless ($config->{tape_size} > 0) {
        die sprintf(
            "BrainfuckIrCompiler: invalid tape_size %d: must be positive",
            $config->{tape_size}
        );
    }

    # Build the internal compiler state object
    my $c = {
        config     => $config,
        filename   => $filename,
        id_gen     => CodingAdventures::CompilerIr::IDGenerator->new,
        node_id    => 0,
        program    => CodingAdventures::CompilerIr::IrProgram->new('_start'),
        source_map => CodingAdventures::CompilerSourceMap::SourceMapChain->new_chain,
        loop_count => 0,
    };

    # Add tape data declaration
    $c->{program}->add_data(
        CodingAdventures::CompilerIr::IrDataDecl->new(
            label => 'tape',
            size  => $config->{tape_size},
            init  => 0,
        )
    );

    # Emit prologue, body, epilogue
    _emit_prologue($c);
    _compile_program($c, $ast);
    _emit_epilogue($c);

    return {
        program    => $c->{program},
        source_map => $c->{source_map},
    };
}

# ============================================================================
# Internal helpers
# ============================================================================

# _next_node_id($c) — return the next unique AST node ID.
sub _next_node_id {
    my ($c) = @_;
    return $c->{node_id}++;
}

# _emit($c, $opcode, @operands) — add one instruction, return its ID.
sub _emit {
    my ($c, $opcode, @operands) = @_;
    my $id = $c->{id_gen}->next;
    $c->{program}->add_instruction(
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => $opcode,
            operands => \@operands,
            id       => $id,
        )
    );
    return $id;
}

# _emit_label($c, $name) — add a LABEL pseudo-instruction (id = -1).
sub _emit_label {
    my ($c, $name) = @_;
    $c->{program}->add_instruction(
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::LABEL(),
            operands => [ CodingAdventures::CompilerIr::IrLabel->new($name) ],
            id       => -1,
        )
    );
}

# _reg($n) — shorthand: IrRegister->new($n)
sub _reg { CodingAdventures::CompilerIr::IrRegister->new($_[0]) }

# _imm($n) — shorthand: IrImmediate->new($n)
sub _imm { CodingAdventures::CompilerIr::IrImmediate->new($_[0]) }

# _lbl($name) — shorthand: IrLabel->new($name)
sub _lbl { CodingAdventures::CompilerIr::IrLabel->new($_[0]) }

# ── Opcode aliases ──────────────────────────────────────────────────────
my $OpLoadAddr  = CodingAdventures::CompilerIr::IrOp::LOAD_ADDR();
my $OpLoadImm   = CodingAdventures::CompilerIr::IrOp::LOAD_IMM();
my $OpLoadByte  = CodingAdventures::CompilerIr::IrOp::LOAD_BYTE();
my $OpStoreByte = CodingAdventures::CompilerIr::IrOp::STORE_BYTE();
my $OpAdd       = CodingAdventures::CompilerIr::IrOp::ADD();
my $OpAddImm    = CodingAdventures::CompilerIr::IrOp::ADD_IMM();
my $OpAndImm    = CodingAdventures::CompilerIr::IrOp::AND_IMM();
my $OpCmpGt     = CodingAdventures::CompilerIr::IrOp::CMP_GT();
my $OpCmpLt     = CodingAdventures::CompilerIr::IrOp::CMP_LT();
my $OpLabel     = CodingAdventures::CompilerIr::IrOp::LABEL();
my $OpJump      = CodingAdventures::CompilerIr::IrOp::JUMP();
my $OpBranchZ   = CodingAdventures::CompilerIr::IrOp::BRANCH_Z();
my $OpBranchNz  = CodingAdventures::CompilerIr::IrOp::BRANCH_NZ();
my $OpSyscall   = CodingAdventures::CompilerIr::IrOp::SYSCALL();
my $OpHalt      = CodingAdventures::CompilerIr::IrOp::HALT();

# ============================================================================
# Prologue and Epilogue
# ============================================================================

# _emit_prologue($c) — set up execution environment.
#
# Emits:
#   _start:
#   LOAD_ADDR v0, tape    ← v0 = &tape (base address)
#   LOAD_IMM  v1, 0       ← v1 = 0 (tape pointer starts at cell 0)
#   [debug only:]
#   LOAD_IMM  v5, tape_size-1  ← v5 = max valid pointer
#   LOAD_IMM  v6, 0             ← v6 = 0 (lower bound)
sub _emit_prologue {
    my ($c) = @_;

    _emit_label($c, '_start');

    # v0 = &tape
    _emit($c, $OpLoadAddr, _reg(REG_TAPE_BASE), _lbl('tape'));

    # v1 = 0
    _emit($c, $OpLoadImm, _reg(REG_TAPE_PTR), _imm(0));

    if ($c->{config}{insert_bounds_checks}) {
        # v5 = tape_size - 1
        _emit($c, $OpLoadImm, _reg(REG_MAX_PTR), _imm($c->{config}{tape_size} - 1));
        # v6 = 0
        _emit($c, $OpLoadImm, _reg(REG_ZERO), _imm(0));
    }
}

# _emit_epilogue($c) — terminate the program.
#
# Emits HALT and (in debug mode) the __trap_oob handler.
sub _emit_epilogue {
    my ($c) = @_;

    _emit($c, $OpHalt);

    if ($c->{config}{insert_bounds_checks}) {
        _emit_label($c, '__trap_oob');
        # Load error exit code 1 into syscall argument register
        _emit($c, $OpLoadImm, _reg(REG_SYS_ARG), _imm(1));
        # Exit with code 1
        _emit($c, $OpSyscall, _imm(SYSCALL_EXIT));
    }
}

# ============================================================================
# AST Walking
# ============================================================================
#
# The Perl Brainfuck AST uses hashrefs with these types:
#
#   program     → { type => 'program',     children => [...] }
#   instruction → { type => 'instruction', children => [loop_or_command] }
#   loop        → { type => 'loop',        children => [...instructions...] }
#   command     → { type => 'command',     token => { type => ..., value => ..., line => N, col => N } }

# _compile_program($c, $node) — compile the root program node.
sub _compile_program {
    my ($c, $node) = @_;
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        _compile_node($c, $child);
    }
}

# _compile_node($c, $node) — dispatch on AST node type.
sub _compile_node {
    my ($c, $node) = @_;
    my $type = $node->{type} // '';

    if ($type eq 'instruction') {
        for my $child (@{ $node->{children} }) {
            next unless ref $child eq 'HASH';
            _compile_node($c, $child);
        }
    }
    elsif ($type eq 'command') {
        _compile_command($c, $node);
    }
    elsif ($type eq 'loop') {
        _compile_loop($c, $node);
    }
    else {
        die "BrainfuckIrCompiler: unexpected AST node type '$type'";
    }
}

# ============================================================================
# Command compilation
# ============================================================================

sub _compile_command {
    my ($c, $node) = @_;

    my $tok = $node->{token};
    unless (defined $tok) {
        die "BrainfuckIrCompiler: command node has no token";
    }

    my $ast_node_id = _next_node_id($c);

    # Record source position → AST node ID (Segment 1)
    $c->{source_map}{source_to_ast}->add(
        CodingAdventures::CompilerSourceMap::SourcePosition->new(
            file   => $c->{filename},
            line   => $tok->{line} // 0,
            column => $tok->{col}  // 0,
            length => 1,
        ),
        $ast_node_id,
    );

    my @ir_ids;
    my $cmd = $tok->{value};

    if ($cmd eq '>') {
        # RIGHT: move tape pointer right (+1)
        if ($c->{config}{insert_bounds_checks}) {
            push @ir_ids, _emit_bounds_check_right($c);
        }
        push @ir_ids, _emit($c, $OpAddImm,
            _reg(REG_TAPE_PTR), _reg(REG_TAPE_PTR), _imm(1));
    }
    elsif ($cmd eq '<') {
        # LEFT: move tape pointer left (-1)
        if ($c->{config}{insert_bounds_checks}) {
            push @ir_ids, _emit_bounds_check_left($c);
        }
        push @ir_ids, _emit($c, $OpAddImm,
            _reg(REG_TAPE_PTR), _reg(REG_TAPE_PTR), _imm(-1));
    }
    elsif ($cmd eq '+') {
        # INC: increment current cell
        push @ir_ids, _emit_cell_mutation($c, 1);
    }
    elsif ($cmd eq '-') {
        # DEC: decrement current cell
        push @ir_ids, _emit_cell_mutation($c, -1);
    }
    elsif ($cmd eq '.') {
        # OUTPUT: write current cell to stdout
        # Load cell value
        my $id1 = _emit($c, $OpLoadByte,
            _reg(REG_TEMP), _reg(REG_TAPE_BASE), _reg(REG_TAPE_PTR));
        push @ir_ids, $id1;
        # Copy cell value to the syscall argument register without using v6.
        my $id2 = _emit($c, $OpAddImm,
            _reg(REG_SYS_ARG), _reg(REG_TEMP), _imm(0));
        push @ir_ids, $id2;
        # Syscall 1 = write byte
        my $id3 = _emit($c, $OpSyscall, _imm(SYSCALL_WRITE));
        push @ir_ids, $id3;
    }
    elsif ($cmd eq ',') {
        # INPUT: read byte from stdin into current cell
        # Syscall 2 = read byte (result in syscall arg register)
        my $id1 = _emit($c, $OpSyscall, _imm(SYSCALL_READ));
        push @ir_ids, $id1;
        # Store result to current cell
        my $id2 = _emit($c, $OpStoreByte,
            _reg(REG_SYS_ARG), _reg(REG_TAPE_BASE), _reg(REG_TAPE_PTR));
        push @ir_ids, $id2;
    }
    else {
        die "BrainfuckIrCompiler: unknown command token '$cmd'";
    }

    # Record AST node → IR IDs (Segment 2)
    $c->{source_map}{ast_to_ir}->add($ast_node_id, \@ir_ids);
}

# _emit_cell_mutation($c, $delta) — LOAD_BYTE, ADD_IMM, [AND_IMM,] STORE_BYTE
#
# Returns a list of emitted instruction IDs.
sub _emit_cell_mutation {
    my ($c, $delta) = @_;
    my @ids;

    # Load current cell value
    push @ids, _emit($c, $OpLoadByte,
        _reg(REG_TEMP), _reg(REG_TAPE_BASE), _reg(REG_TAPE_PTR));

    # Add delta
    push @ids, _emit($c, $OpAddImm,
        _reg(REG_TEMP), _reg(REG_TEMP), _imm($delta));

    # Mask to byte range 0-255 (if enabled)
    if ($c->{config}{mask_byte_arithmetic}) {
        push @ids, _emit($c, $OpAndImm,
            _reg(REG_TEMP), _reg(REG_TEMP), _imm(255));
    }

    # Store back to cell
    push @ids, _emit($c, $OpStoreByte,
        _reg(REG_TEMP), _reg(REG_TAPE_BASE), _reg(REG_TAPE_PTR));

    return @ids;
}

# ============================================================================
# Bounds checking
# ============================================================================
#
# In debug builds, the compiler inserts range checks before every pointer move.
# If the pointer goes out of bounds, the program jumps to __trap_oob
# (which calls exit(1) via SYSCALL 10).
#
# RIGHT (>):
#   CMP_GT  v3, v1, v5        ← is ptr > tape_size-1 (i.e., >= tape_size)?
#   BRANCH_NZ v3, __trap_oob  ← if so, trap
#
# LEFT (<):
#   CMP_LT  v1, v1, v6        ← is ptr < 0?
#   BRANCH_NZ v1, __trap_oob  ← if so, trap

sub _emit_bounds_check_right {
    my ($c) = @_;
    my @ids;
    push @ids, _emit($c, $OpCmpGt,
        _reg(REG_TEMP2), _reg(REG_TAPE_PTR), _reg(REG_MAX_PTR));
    push @ids, _emit($c, $OpBranchNz,
        _reg(REG_TEMP2), _lbl('__trap_oob'));
    return @ids;
}

sub _emit_bounds_check_left {
    my ($c) = @_;
    my @ids;
    push @ids, _emit($c, $OpCmpLt,
        _reg(REG_TAPE_PTR), _reg(REG_TAPE_PTR), _reg(REG_ZERO));
    push @ids, _emit($c, $OpBranchNz,
        _reg(REG_TAPE_PTR), _lbl('__trap_oob'));
    return @ids;
}

# ============================================================================
# Loop compilation
# ============================================================================
#
# A Brainfuck loop [body] compiles to:
#
#   LABEL      loop_N_start
#   LOAD_BYTE  v2, v0, v1          ← load current cell
#   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
#   ...compile body...
#   JUMP       loop_N_start        ← repeat
#   LABEL      loop_N_end
#
# Each loop gets a unique number N (from $c->{loop_count}).

sub _compile_loop {
    my ($c, $node) = @_;

    my $loop_num   = $c->{loop_count}++;
    my $start_lbl  = "loop_${loop_num}_start";
    my $end_lbl    = "loop_${loop_num}_end";

    # Create source map entry for the loop bracket itself
    my $ast_node_id = _next_node_id($c);
    my $start_line  = $node->{line} // 0;
    my $start_col   = $node->{col}  // 0;
    if ($start_line > 0) {
        $c->{source_map}{source_to_ast}->add(
            CodingAdventures::CompilerSourceMap::SourcePosition->new(
                file   => $c->{filename},
                line   => $start_line,
                column => $start_col,
                length => 1,
            ),
            $ast_node_id,
        );
    }

    my @ir_ids;

    # Emit loop start label
    _emit_label($c, $start_lbl);

    # Load current cell and branch if zero
    push @ir_ids, _emit($c, $OpLoadByte,
        _reg(REG_TEMP), _reg(REG_TAPE_BASE), _reg(REG_TAPE_PTR));
    push @ir_ids, _emit($c, $OpBranchZ,
        _reg(REG_TEMP), _lbl($end_lbl));

    # Compile loop body (all instruction children)
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        _compile_node($c, $child);
    }

    # Jump back to loop start
    push @ir_ids, _emit($c, $OpJump, _lbl($start_lbl));

    # Emit loop end label
    _emit_label($c, $end_lbl);

    # Record AST → IR mapping for the loop construct
    $c->{source_map}{ast_to_ir}->add($ast_node_id, \@ir_ids) if $start_line > 0;
}

1;

__END__

=head1 NAME

CodingAdventures::BrainfuckIrCompiler::Compiler - compile a Brainfuck AST into IR

=head1 SYNOPSIS

  use CodingAdventures::BrainfuckIrCompiler::Compiler qw(compile);
  use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
  use CodingAdventures::Brainfuck::Parser;

  my $ast    = CodingAdventures::Brainfuck::Parser->parse('+.');
  my $cfg    = CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;
  my $result = compile($ast, 'hello.bf', $cfg);

  my $program    = $result->{program};      # IrProgram
  my $source_map = $result->{source_map};   # SourceMapChain

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
