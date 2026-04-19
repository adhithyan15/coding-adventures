package CodingAdventures::IrToWasmCompiler;

# ============================================================================
# CodingAdventures::IrToWasmCompiler — lower generic IR into Wasm modules
# ============================================================================
#
# This package ports the conservative Python lowering strategy into Perl:
#
#   compiler-ir program
#     -> split into functions at LABEL _start / LABEL _fn_*
#     -> map IR registers to Wasm i32 locals
#     -> lower structured loops and conditionals into block/loop/if
#     -> import just the WASI functions required by SYSCALL opcodes
#     -> emit a plain Wasm module hashref that the local validator, encoder,
#        parser, and runtime can all understand
#
# The backend is intentionally narrow. It supports the structured IR patterns
# already emitted by the current Brainfuck and Nib frontends, not arbitrary
# unstructured control flow graphs.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Carp qw(croak);
use Exporter 'import';
use Scalar::Util qw(blessed reftype);

use CodingAdventures::CompilerIr;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::WasmLeb128 qw(encode_signed encode_unsigned);
use CodingAdventures::WasmTypes ();

our @EXPORT_OK = qw(
    compile
    infer_function_signatures_from_comments
    new_function_signature
);

use constant {
    _SYSCALL_WRITE      => 1,
    _SYSCALL_READ       => 2,
    _SYSCALL_EXIT       => 10,
    _SYSCALL_ARG0       => 4,
    _REG_SCRATCH        => 1,
    _REG_VAR_BASE       => 2,
    _WASI_MODULE        => 'wasi_snapshot_preview1',
    _WASI_IOVEC_OFFSET  => 0,
    _WASI_COUNT_OFFSET  => 8,
    _WASI_BYTE_OFFSET   => 12,
    _WASI_SCRATCH_SIZE  => 16,
    _MAX_FUNCTION_PARAMS => 128,
    _MAX_DATA_DECL_BYTES => 16 * 1024 * 1024,
    _MAX_TOTAL_DATA_BYTES => 16 * 1024 * 1024,
};

my %_OPCODE_BY_NAME = (
    'nop'         => 0x01,
    'block'       => 0x02,
    'loop'        => 0x03,
    'if'          => 0x04,
    'else'        => 0x05,
    'end'         => 0x0B,
    'br'          => 0x0C,
    'br_if'       => 0x0D,
    'return'      => 0x0F,
    'call'        => 0x10,
    'local.get'   => 0x20,
    'local.set'   => 0x21,
    'i32.load'    => 0x28,
    'i32.load8_u' => 0x2D,
    'i32.store'   => 0x36,
    'i32.store8'  => 0x3A,
    'i32.const'   => 0x41,
    'i32.eqz'     => 0x45,
    'i32.eq'      => 0x46,
    'i32.ne'      => 0x47,
    'i32.lt_s'    => 0x48,
    'i32.gt_s'    => 0x4A,
    'i32.add'     => 0x6A,
    'i32.sub'     => 0x6B,
    'i32.and'     => 0x71,
);

sub new_function_signature {
    my ($label, $param_count, $export_name) = @_;
    return _validate_function_signature({
        label       => $label,
        param_count => $param_count,
        export_name => $export_name,
    });
}

sub compile {
    my ($program, $function_signatures) = @_;
    croak 'CodingAdventures::IrToWasmCompiler::compile: IrProgram required'
        unless blessed($program) && $program->isa('CodingAdventures::CompilerIr::IrProgram');

    my $data_decls = _validate_data_decls($program->{data} || []);
    my %signatures = %{ infer_function_signatures_from_comments($program) };
    for my $signature (@{ $function_signatures || [] }) {
        my $validated = _validate_function_signature($signature);
        $signatures{$validated->{label}} = $validated;
    }

    my $functions = _split_functions($program, \%signatures);
    my $imports = _collect_wasi_imports($program);
    my ($type_indices, $types) = _build_type_table($functions, $imports);
    my $data_offsets = _layout_data($data_decls);
    my $data_size = _data_size($data_decls);
    my $scratch_base = _needs_wasi_scratch($program) ? _align_up($data_size, 4) : undef;

    my $module = {
        types     => $types,
        imports   => [],
        functions => [],
        tables    => [],
        memories  => [],
        globals   => [],
        exports   => [],
        start     => undef,
        elements  => [],
        codes     => [],
        data      => [],
        custom    => [],
    };

    for my $import (@$imports) {
        push @{ $module->{imports} }, {
            module => _WASI_MODULE,
            mod    => _WASI_MODULE,
            name   => $import->{name},
            desc   => {
                kind => 'func',
                idx  => $type_indices->{ $import->{type_key} },
            },
        };
    }

    my $function_index_base = scalar @$imports;
    my %function_indices;
    for my $i (0 .. $#$functions) {
        $function_indices{ $functions->[$i]{label} } = $function_index_base + $i;
        push @{ $module->{functions} }, $type_indices->{ $functions->[$i]{label} };
    }

    my $total_bytes = $data_size;
    if (defined $scratch_base) {
        $total_bytes = _max($total_bytes, $scratch_base + _WASI_SCRATCH_SIZE);
    }
    if (_needs_memory($program) || defined $scratch_base) {
        my $page_count = $total_bytes ? int(($total_bytes + 65535) / 65536) : 1;
        $page_count = 1 if $page_count < 1;
        push @{ $module->{memories} }, { limits => { min => $page_count, max => undef } };
        push @{ $module->{exports} }, { name => 'memory', desc => { kind => 'mem', idx => 0 } };

        for my $decl (@$data_decls) {
            my $offset = $data_offsets->{ $decl->{label} };
            push @{ $module->{data} }, {
                memory_index => 0,
                offset_expr  => _const_expr($offset),
                data         => pack('C*', (($decl->{init} || 0) & 0xFF) x $decl->{size}),
            };
        }
    }

    my %wasi_indices = map { $imports->[$_]{syscall_number} => $_ } 0 .. $#$imports;
    my $wasi_context = {
        function_indices => \%wasi_indices,
        scratch_base     => $scratch_base,
    };

    for my $function (@$functions) {
        push @{ $module->{codes} }, _lower_function(
            function         => $function,
            signatures       => \%signatures,
            function_indices => \%function_indices,
            data_offsets     => $data_offsets,
            wasi_context     => $wasi_context,
        );

        if (defined $function->{signature}{export_name}) {
            push @{ $module->{exports} }, {
                name => $function->{signature}{export_name},
                desc => {
                    kind => 'func',
                    idx  => $function_indices{ $function->{label} },
                },
            };
        }
    }

    return $module;
}

sub infer_function_signatures_from_comments {
    my ($program) = @_;
    my %signatures;
    my $pending_comment;

    for my $instruction (@{ $program->{instructions} || [] }) {
        if ($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::COMMENT) {
            $pending_comment = _label_name_from_operand($instruction->{operands}[0]);
            next;
        }

        my $label_name = _function_label_name($instruction);
        if (defined $label_name) {
            if ($label_name eq '_start') {
                $signatures{$label_name} = new_function_signature($label_name, 0, '_start');
            }
            elsif ($label_name =~ /^_fn_(.+)$/ && defined $pending_comment) {
                my $export_name = $1;
                if ($pending_comment =~ /^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$/ && $1 eq $export_name) {
                    my $params_blob = $2;
                    my @params = grep { length($_) } map { s/^\s+|\s+$//gr } split /,/, $params_blob;
                    $signatures{$label_name} = new_function_signature($label_name, scalar @params, $export_name);
                }
            }
            $pending_comment = undef;
            next;
        }

        $pending_comment = undef if $instruction->{opcode} != CodingAdventures::CompilerIr::IrOp::COMMENT;
    }

    return \%signatures;
}

sub _build_type_table {
    my ($functions, $imports) = @_;
    my @types;
    my %indices;
    my %function_to_type_index;

    for my $import (@$imports) {
        my $key = _func_type_key($import->{func_type});
        if (!exists $indices{$key}) {
            $indices{$key} = scalar @types;
            push @types, $import->{func_type};
        }
        $function_to_type_index{ $import->{type_key} } = $indices{$key};
    }

    for my $function (@$functions) {
        my $func_type = {
            params  => [ (($CodingAdventures::WasmTypes::ValType{i32}) x $function->{signature}{param_count}) ],
            results => [ $CodingAdventures::WasmTypes::ValType{i32} ],
        };
        my $key = _func_type_key($func_type);
        if (!exists $indices{$key}) {
            $indices{$key} = scalar @types;
            push @types, $func_type;
        }
        $function_to_type_index{ $function->{label} } = $indices{$key};
    }

    return (\%function_to_type_index, \@types);
}

sub _layout_data {
    my ($decls) = @_;
    my %offsets;
    my $cursor = 0;
    for my $decl (@$decls) {
        $offsets{ $decl->{label} } = $cursor;
        $cursor += $decl->{size};
    }
    return \%offsets;
}

sub _data_size {
    my ($decls) = @_;
    my $sum = 0;
    for my $decl (@{ $decls || [] }) {
        $sum += $decl->{size};
    }
    return $sum;
}

sub _validate_function_signature {
    my ($signature) = @_;
    croak 'CodingAdventures::IrToWasmCompiler: function signature hashref required'
        unless _is_hash_like($signature);

    my $label = $signature->{label};
    croak 'CodingAdventures::IrToWasmCompiler: function signature requires label'
        unless defined $label && !ref($label) && length($label);

    my $param_count = $signature->{param_count};
    croak "CodingAdventures::IrToWasmCompiler: function '$label' param_count must be a non-negative integer"
        unless _is_bounded_nonnegative_integer($param_count, _MAX_FUNCTION_PARAMS);

    return {
        label       => $label,
        param_count => 0 + $param_count,
        export_name => $signature->{export_name},
    };
}

sub _validate_data_decls {
    my ($decls) = @_;
    croak 'CodingAdventures::IrToWasmCompiler: data declarations arrayref required'
        unless ref($decls || []) eq 'ARRAY';

    my @validated;
    my $total = 0;

    for my $decl (@{ $decls || [] }) {
        croak 'CodingAdventures::IrToWasmCompiler: data declaration hashref required'
            unless _is_hash_like($decl);

        my $label = $decl->{label};
        croak 'CodingAdventures::IrToWasmCompiler: data declaration requires label'
            unless defined $label && !ref($label) && length($label);

        my $size = $decl->{size};
        croak "CodingAdventures::IrToWasmCompiler: data declaration '$label' size must be between 0 and "
            . _MAX_DATA_DECL_BYTES . ' bytes'
            unless _is_bounded_nonnegative_integer($size, _MAX_DATA_DECL_BYTES);

        $total += 0 + $size;
        croak 'CodingAdventures::IrToWasmCompiler: total data size exceeds '
            . _MAX_TOTAL_DATA_BYTES . ' bytes'
            if $total > _MAX_TOTAL_DATA_BYTES;

        my $init = defined($decl->{init}) ? $decl->{init} : 0;
        croak "CodingAdventures::IrToWasmCompiler: data declaration '$label' init must be a byte"
            unless _is_bounded_nonnegative_integer($init, 255);

        push @validated, {
            label => $label,
            size  => 0 + $size,
            init  => 0 + $init,
        };
    }

    return \@validated;
}

sub _is_bounded_nonnegative_integer {
    my ($value, $max) = @_;
    return 0 unless defined $value && !ref($value);
    return 0 unless "$value" =~ /\A(?:0|[1-9][0-9]*)\z/;
    return 0 if $value > $max;
    return 1;
}

sub _is_hash_like {
    my ($value) = @_;
    return ref($value) && (reftype($value) || '') eq 'HASH' ? 1 : 0;
}

sub _needs_memory {
    my ($program) = @_;
    return 1 if @{ $program->{data} || [] };
    for my $instruction (@{ $program->{instructions} || [] }) {
        return 1 if $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::LOAD_ADDR
                 || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::LOAD_BYTE
                 || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::STORE_BYTE
                 || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::LOAD_WORD
                 || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::STORE_WORD;
    }
    return 0;
}

sub _needs_wasi_scratch {
    my ($program) = @_;
    for my $instruction (@{ $program->{instructions} || [] }) {
        next unless $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::SYSCALL;
        next unless @{ $instruction->{operands} || [] };
        my $syscall = _expect_immediate($instruction->{operands}[0], 'SYSCALL number')->{value};
        return 1 if $syscall == _SYSCALL_WRITE || $syscall == _SYSCALL_READ;
    }
    return 0;
}

sub _collect_wasi_imports {
    my ($program) = @_;
    my %required;
    for my $instruction (@{ $program->{instructions} || [] }) {
        next unless $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::SYSCALL;
        next unless @{ $instruction->{operands} || [] };
        $required{ _expect_immediate($instruction->{operands}[0], 'SYSCALL number')->{value} } = 1;
    }

    my @ordered = (
        {
            syscall_number => _SYSCALL_WRITE,
            name           => 'fd_write',
            type_key       => 'wasi::fd_write',
            func_type      => {
                params  => [ ($CodingAdventures::WasmTypes::ValType{i32}) x 4 ],
                results => [ $CodingAdventures::WasmTypes::ValType{i32} ],
            },
        },
        {
            syscall_number => _SYSCALL_READ,
            name           => 'fd_read',
            type_key       => 'wasi::fd_read',
            func_type      => {
                params  => [ ($CodingAdventures::WasmTypes::ValType{i32}) x 4 ],
                results => [ $CodingAdventures::WasmTypes::ValType{i32} ],
            },
        },
        {
            syscall_number => _SYSCALL_EXIT,
            name           => 'proc_exit',
            type_key       => 'wasi::proc_exit',
            func_type      => {
                params  => [ $CodingAdventures::WasmTypes::ValType{i32} ],
                results => [],
            },
        },
    );

    for my $syscall (sort { $a <=> $b } keys %required) {
        next if grep { $_->{syscall_number} == $syscall } @ordered;
        croak "CodingAdventures::IrToWasmCompiler: unsupported SYSCALL number $syscall";
    }

    return [ grep { $required{ $_->{syscall_number} } } @ordered ];
}

sub _split_functions {
    my ($program, $signatures) = @_;
    my @functions;
    my ($start_index, $start_label);

    for my $index (0 .. $#{ $program->{instructions} || [] }) {
        my $instruction = $program->{instructions}[$index];
        my $label_name = _function_label_name($instruction);
        next unless defined $label_name;

        if (defined $start_label && defined $start_index) {
            push @functions, _make_function_ir(
                label        => $start_label,
                instructions => [ @{$program->{instructions}}[$start_index .. $index - 1] ],
                signatures   => $signatures,
            );
        }
        $start_label = $label_name;
        $start_index = $index;
    }

    if (defined $start_label && defined $start_index) {
        push @functions, _make_function_ir(
            label        => $start_label,
            instructions => [ @{$program->{instructions}}[$start_index .. $#{ $program->{instructions} }] ],
            signatures   => $signatures,
        );
    }

    return \@functions;
}

sub _lower_function {
    my (%args) = @_;
    my $function = $args{function};
    my $signatures = $args{signatures};
    my $function_indices = $args{function_indices};
    my $data_offsets = $args{data_offsets};
    my $wasi_context = $args{wasi_context};
    my $param_count = $function->{signature}{param_count};
    my $bytes = '';
    my $instructions = $function->{instructions};
    my %label_to_index = map {
        my $label = _label_name($instructions->[$_]);
        defined $label ? ($label => $_) : ()
    } 0 .. $#$instructions;

    for my $param_index (0 .. $param_count - 1) {
        $bytes .= _emit_opcode('local.get');
        $bytes .= _u32($param_index);
        $bytes .= _emit_opcode('local.set');
        $bytes .= _u32($param_count + _REG_VAR_BASE + $param_index);
    }

    $bytes .= _emit_region(
        start            => 1,
        end              => scalar @$instructions,
        instructions     => $instructions,
        label_to_index   => \%label_to_index,
        function         => $function,
        signatures       => $signatures,
        function_indices => $function_indices,
        data_offsets     => $data_offsets,
        wasi_context     => $wasi_context,
        param_count      => $param_count,
    );
    $bytes .= _emit_opcode('end');

    return {
        locals => [ { count => $function->{max_reg} + 1, type => $CodingAdventures::WasmTypes::ValType{i32} } ],
        body   => $bytes,
    };
}

sub _emit_region {
    my (%args) = @_;
    my $instructions = $args{instructions};
    my $index = $args{start};
    my $bytes = '';

    while ($index < $args{end}) {
        my $instruction = $instructions->[$index];

        if ($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::COMMENT) {
            $index++;
            next;
        }

        my $label_name = _label_name($instruction);
        if (defined $label_name && $label_name =~ /^loop_\d+_start$/) {
            my ($fragment, $next_index) = _emit_loop(%args, label_index => $index);
            $bytes .= $fragment;
            $index = $next_index;
            next;
        }

        if (($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_Z
                || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_NZ)
            && @{ $instruction->{operands} || [] } >= 2
            && _operand_label_name($instruction->{operands}[1]) =~ /^if_\d+_else$/) {
            my ($fragment, $next_index) = _emit_if(%args, branch_index => $index);
            $bytes .= $fragment;
            $index = $next_index;
            next;
        }

        if ($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::LABEL) {
            $index++;
            next;
        }

        if ($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::JUMP
            || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_Z
            || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_NZ) {
            croak "CodingAdventures::IrToWasmCompiler: unexpected unstructured control flow in $args{function}{label}";
        }

        $bytes .= _emit_simple(%args, instruction => $instruction);
        $index++;
    }

    return $bytes;
}

sub _emit_if {
    my (%args) = @_;
    my $branch = $args{instructions}[ $args{branch_index} ];
    my $cond_reg = _expect_register($branch->{operands}[0], 'if condition');
    my $else_label = _expect_label($branch->{operands}[1], 'if else label')->{name};
    (my $end_label = $else_label) =~ s/_else$/_end/;

    my $else_index = _require_label_index($args{label_to_index}, $else_label, $args{function}{label});
    my $end_index = _require_label_index($args{label_to_index}, $end_label, $args{function}{label});
    my $jump_index = _find_last_jump_to_label($args{instructions}, $args{branch_index} + 1, $else_index, $end_label, $args{function}{label});

    my $bytes = '';
    $bytes .= _emit_local_get($args{param_count}, $cond_reg->{index});
    if ($branch->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_NZ) {
        $bytes .= _emit_opcode('i32.eqz');
    }
    $bytes .= _emit_opcode('if') . pack('C', CodingAdventures::WasmTypes::BLOCK_TYPE_EMPTY());
    $bytes .= _emit_region(%args, start => $args{branch_index} + 1, end => $jump_index);

    if ($else_index + 1 < $end_index) {
        $bytes .= _emit_opcode('else');
        $bytes .= _emit_region(%args, start => $else_index + 1, end => $end_index);
    }

    $bytes .= _emit_opcode('end');
    return ($bytes, $end_index + 1);
}

sub _emit_loop {
    my (%args) = @_;
    my $start_label = _label_name($args{instructions}[ $args{label_index} ]);
    croak 'CodingAdventures::IrToWasmCompiler: loop lowering expected a start label'
        unless defined $start_label;
    (my $end_label = $start_label) =~ s/_start$/_end/;

    my $end_index = _require_label_index($args{label_to_index}, $end_label, $args{function}{label});
    my $branch_index = _find_first_branch_to_label(
        $args{instructions},
        $args{label_index} + 1,
        $end_index,
        $end_label,
        $args{function}{label},
    );
    my $backedge_index = _find_last_jump_to_label(
        $args{instructions},
        $branch_index + 1,
        $end_index,
        $start_label,
        $args{function}{label},
    );

    my $branch = $args{instructions}[$branch_index];
    my $cond_reg = _expect_register($branch->{operands}[0], 'loop condition');
    my $bytes = '';
    $bytes .= _emit_opcode('block') . pack('C', CodingAdventures::WasmTypes::BLOCK_TYPE_EMPTY());
    $bytes .= _emit_opcode('loop')  . pack('C', CodingAdventures::WasmTypes::BLOCK_TYPE_EMPTY());
    $bytes .= _emit_region(%args, start => $args{label_index} + 1, end => $branch_index);
    $bytes .= _emit_local_get($args{param_count}, $cond_reg->{index});
    if ($branch->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_Z) {
        $bytes .= _emit_opcode('i32.eqz');
    }
    $bytes .= _emit_opcode('br_if') . _u32(1);
    $bytes .= _emit_region(%args, start => $branch_index + 1, end => $backedge_index);
    $bytes .= _emit_opcode('br') . _u32(0);
    $bytes .= _emit_opcode('end') . _emit_opcode('end');
    return ($bytes, $end_index + 1);
}

sub _emit_simple {
    my (%args) = @_;
    my $instruction = $args{instruction};
    my $opcode = $instruction->{opcode};

    if ($opcode == CodingAdventures::CompilerIr::IrOp::LOAD_IMM) {
        my $dst = _expect_register($instruction->{operands}[0], 'LOAD_IMM dst');
        my $imm = _expect_immediate($instruction->{operands}[1], 'LOAD_IMM imm');
        return _emit_i32_const($imm->{value}) . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::LOAD_ADDR) {
        my $dst = _expect_register($instruction->{operands}[0], 'LOAD_ADDR dst');
        my $label = _expect_label($instruction->{operands}[1], 'LOAD_ADDR label');
        croak "CodingAdventures::IrToWasmCompiler: unknown data label $label->{name}"
            unless exists $args{data_offsets}{ $label->{name} };
        return _emit_i32_const($args{data_offsets}{ $label->{name} })
            . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::LOAD_BYTE) {
        my ($dst, $base, $offset) = map {
            _expect_register($instruction->{operands}[$_], 'LOAD_BYTE operand')
        } 0..2;
        return _emit_address($args{param_count}, $base->{index}, $offset->{index})
            . _emit_opcode('i32.load8_u')
            . _emit_memarg(0, 0)
            . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::STORE_BYTE) {
        my ($src, $base, $offset) = map {
            _expect_register($instruction->{operands}[$_], 'STORE_BYTE operand')
        } 0..2;
        return _emit_address($args{param_count}, $base->{index}, $offset->{index})
            . _emit_local_get($args{param_count}, $src->{index})
            . _emit_opcode('i32.store8')
            . _emit_memarg(0, 0);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::LOAD_WORD) {
        my ($dst, $base, $offset) = map {
            _expect_register($instruction->{operands}[$_], 'LOAD_WORD operand')
        } 0..2;
        return _emit_address($args{param_count}, $base->{index}, $offset->{index})
            . _emit_opcode('i32.load')
            . _emit_memarg(2, 0)
            . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::STORE_WORD) {
        my ($src, $base, $offset) = map {
            _expect_register($instruction->{operands}[$_], 'STORE_WORD operand')
        } 0..2;
        return _emit_address($args{param_count}, $base->{index}, $offset->{index})
            . _emit_local_get($args{param_count}, $src->{index})
            . _emit_opcode('i32.store')
            . _emit_memarg(2, 0);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::ADD) {
        return _emit_binary_numeric($args{param_count}, 'i32.add', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::ADD_IMM) {
        my $dst = _expect_register($instruction->{operands}[0], 'ADD_IMM dst');
        my $src = _expect_register($instruction->{operands}[1], 'ADD_IMM src');
        my $imm = _expect_immediate($instruction->{operands}[2], 'ADD_IMM imm');
        return _emit_local_get($args{param_count}, $src->{index})
            . _emit_i32_const($imm->{value})
            . _emit_opcode('i32.add')
            . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::SUB) {
        return _emit_binary_numeric($args{param_count}, 'i32.sub', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::AND) {
        return _emit_binary_numeric($args{param_count}, 'i32.and', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::AND_IMM) {
        my $dst = _expect_register($instruction->{operands}[0], 'AND_IMM dst');
        my $src = _expect_register($instruction->{operands}[1], 'AND_IMM src');
        my $imm = _expect_immediate($instruction->{operands}[2], 'AND_IMM imm');
        return _emit_local_get($args{param_count}, $src->{index})
            . _emit_i32_const($imm->{value})
            . _emit_opcode('i32.and')
            . _emit_local_set($args{param_count}, $dst->{index});
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::CMP_EQ) {
        return _emit_binary_numeric($args{param_count}, 'i32.eq', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::CMP_NE) {
        return _emit_binary_numeric($args{param_count}, 'i32.ne', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::CMP_LT) {
        return _emit_binary_numeric($args{param_count}, 'i32.lt_s', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::CMP_GT) {
        return _emit_binary_numeric($args{param_count}, 'i32.gt_s', $instruction);
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::CALL) {
        my $label = _expect_label($instruction->{operands}[0], 'CALL target');
        my $signature = $args{signatures}{ $label->{name} }
            or croak "CodingAdventures::IrToWasmCompiler: missing function signature for $label->{name}";
        my $function_index = $args{function_indices}{ $label->{name} };
        croak "CodingAdventures::IrToWasmCompiler: unknown function label $label->{name}"
            unless defined $function_index;

        my $bytes = '';
        for my $param_index (0 .. $signature->{param_count} - 1) {
            $bytes .= _emit_local_get($args{param_count}, _REG_VAR_BASE + $param_index);
        }
        $bytes .= _emit_opcode('call') . _u32($function_index);
        $bytes .= _emit_local_set($args{param_count}, _REG_SCRATCH);
        return $bytes;
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::RET
        || $opcode == CodingAdventures::CompilerIr::IrOp::HALT) {
        return _emit_local_get($args{param_count}, _REG_SCRATCH) . _emit_opcode('return');
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::NOP) {
        return _emit_opcode('nop');
    }
    if ($opcode == CodingAdventures::CompilerIr::IrOp::SYSCALL) {
        return _emit_syscall(%args, instruction => $instruction);
    }

    croak 'CodingAdventures::IrToWasmCompiler: unsupported opcode '
        . CodingAdventures::CompilerIr::IrOp::op_name($opcode);
}

sub _emit_syscall {
    my (%args) = @_;
    my $instruction = $args{instruction};
    my $syscall = _expect_immediate($instruction->{operands}[0], 'SYSCALL number')->{value};

    return _emit_wasi_write(%args) if $syscall == _SYSCALL_WRITE;
    return _emit_wasi_read(%args)  if $syscall == _SYSCALL_READ;
    return _emit_wasi_exit(%args)  if $syscall == _SYSCALL_EXIT;

    croak "CodingAdventures::IrToWasmCompiler: unsupported SYSCALL number $syscall";
}

sub _emit_wasi_write {
    my (%args) = @_;
    my $scratch_base = _require_wasi_scratch($args{wasi_context});
    my $iovec_ptr = $scratch_base + _WASI_IOVEC_OFFSET;
    my $nwritten_ptr = $scratch_base + _WASI_COUNT_OFFSET;
    my $byte_ptr = $scratch_base + _WASI_BYTE_OFFSET;

    return _emit_i32_const($byte_ptr)
        . _emit_local_get($args{param_count}, _SYSCALL_ARG0)
        . _emit_opcode('i32.store8')
        . _emit_memarg(0, 0)
        . _emit_store_const_i32($iovec_ptr, $byte_ptr)
        . _emit_store_const_i32($iovec_ptr + 4, 1)
        . _emit_i32_const(1)
        . _emit_i32_const($iovec_ptr)
        . _emit_i32_const(1)
        . _emit_i32_const($nwritten_ptr)
        . _emit_wasi_call($args{wasi_context}, _SYSCALL_WRITE)
        . _emit_local_set($args{param_count}, _REG_SCRATCH);
}

sub _emit_wasi_read {
    my (%args) = @_;
    my $scratch_base = _require_wasi_scratch($args{wasi_context});
    my $iovec_ptr = $scratch_base + _WASI_IOVEC_OFFSET;
    my $nread_ptr = $scratch_base + _WASI_COUNT_OFFSET;
    my $byte_ptr = $scratch_base + _WASI_BYTE_OFFSET;

    return _emit_i32_const($byte_ptr)
        . _emit_i32_const(0)
        . _emit_opcode('i32.store8')
        . _emit_memarg(0, 0)
        . _emit_store_const_i32($iovec_ptr, $byte_ptr)
        . _emit_store_const_i32($iovec_ptr + 4, 1)
        . _emit_i32_const(0)
        . _emit_i32_const($iovec_ptr)
        . _emit_i32_const(1)
        . _emit_i32_const($nread_ptr)
        . _emit_wasi_call($args{wasi_context}, _SYSCALL_READ)
        . _emit_local_set($args{param_count}, _REG_SCRATCH)
        . _emit_i32_const($byte_ptr)
        . _emit_opcode('i32.load8_u')
        . _emit_memarg(0, 0)
        . _emit_local_set($args{param_count}, _SYSCALL_ARG0);
}

sub _emit_wasi_exit {
    my (%args) = @_;
    return _emit_local_get($args{param_count}, _SYSCALL_ARG0)
        . _emit_wasi_call($args{wasi_context}, _SYSCALL_EXIT)
        . _emit_i32_const(0)
        . _emit_opcode('return');
}

sub _emit_store_const_i32 {
    my ($address, $value) = @_;
    return _emit_i32_const($address)
        . _emit_i32_const($value)
        . _emit_opcode('i32.store')
        . _emit_memarg(2, 0);
}

sub _emit_wasi_call {
    my ($wasi_context, $syscall_number) = @_;
    my $function_index = $wasi_context->{function_indices}{$syscall_number};
    croak "CodingAdventures::IrToWasmCompiler: missing WASI import for SYSCALL $syscall_number"
        unless defined $function_index;
    return _emit_opcode('call') . _u32($function_index);
}

sub _require_wasi_scratch {
    my ($wasi_context) = @_;
    croak 'CodingAdventures::IrToWasmCompiler: SYSCALL lowering requires WASM scratch memory'
        unless defined $wasi_context->{scratch_base};
    return $wasi_context->{scratch_base};
}

sub _emit_binary_numeric {
    my ($param_count, $wasm_op, $instruction) = @_;
    my $dst = _expect_register($instruction->{operands}[0], 'binary dst');
    my $left = _expect_register($instruction->{operands}[1], 'binary lhs');
    my $right = _expect_register($instruction->{operands}[2], 'binary rhs');
    return _emit_local_get($param_count, $left->{index})
        . _emit_local_get($param_count, $right->{index})
        . _emit_opcode($wasm_op)
        . _emit_local_set($param_count, $dst->{index});
}

sub _emit_address {
    my ($param_count, $base_index, $offset_index) = @_;
    return _emit_local_get($param_count, $base_index)
        . _emit_local_get($param_count, $offset_index)
        . _emit_opcode('i32.add');
}

sub _emit_local_get {
    my ($param_count, $reg_index) = @_;
    return _emit_opcode('local.get') . _u32($param_count + $reg_index);
}

sub _emit_local_set {
    my ($param_count, $reg_index) = @_;
    return _emit_opcode('local.set') . _u32($param_count + $reg_index);
}

sub _emit_i32_const {
    my ($value) = @_;
    return _emit_opcode('i32.const') . pack('C*', encode_signed($value));
}

sub _u32 {
    my ($value) = @_;
    return pack('C*', encode_unsigned($value));
}

sub _emit_memarg {
    my ($align, $offset) = @_;
    return _u32($align) . _u32($offset);
}

sub _emit_opcode {
    my ($name) = @_;
    return pack('C', $_OPCODE_BY_NAME{$name});
}

sub _require_label_index {
    my ($label_to_index, $label, $function_label) = @_;
    croak "CodingAdventures::IrToWasmCompiler: missing label $label in $function_label"
        unless exists $label_to_index->{$label};
    return $label_to_index->{$label};
}

sub _find_first_branch_to_label {
    my ($instructions, $start, $end, $label, $function_label) = @_;
    for my $index ($start .. $end - 1) {
        my $instruction = $instructions->[$index];
        next unless $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_Z
                 || $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::BRANCH_NZ;
        return $index if _operand_label_name($instruction->{operands}[1]) eq $label;
    }
    croak "CodingAdventures::IrToWasmCompiler: expected branch to $label in $function_label";
}

sub _find_last_jump_to_label {
    my ($instructions, $start, $end, $label, $function_label) = @_;
    for (my $index = $end - 1; $index >= $start; $index--) {
        my $instruction = $instructions->[$index];
        next unless $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::JUMP;
        return $index if _operand_label_name($instruction->{operands}[0]) eq $label;
    }
    croak "CodingAdventures::IrToWasmCompiler: expected jump to $label in $function_label";
}

sub _make_function_ir {
    my (%args) = @_;
    my $label = $args{label};
    my $instructions = $args{instructions};
    my $signatures = $args{signatures};

    my $signature = $label eq '_start'
        ? ($signatures->{$label} || new_function_signature($label, 0, '_start'))
        : $signatures->{$label};
    croak "CodingAdventures::IrToWasmCompiler: missing function signature for $label"
        unless $signature;

    my $max_reg = _max(1, _REG_VAR_BASE + ($signature->{param_count} > 0 ? $signature->{param_count} - 1 : 0));
    for my $instruction (@$instructions) {
        for my $operand (@{ $instruction->{operands} || [] }) {
            next unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrRegister');
            $max_reg = _max($max_reg, $operand->{index});
        }
        if ($instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::SYSCALL) {
            $max_reg = _max($max_reg, _SYSCALL_ARG0);
        }
    }

    return {
        label        => $label,
        instructions => $instructions,
        signature    => $signature,
        max_reg      => $max_reg,
    };
}

sub _const_expr {
    my ($value) = @_;
    return _emit_i32_const($value) . _emit_opcode('end');
}

sub _function_label_name {
    my ($instruction) = @_;
    my $label = _label_name($instruction);
    return undef unless defined $label;
    return $label if $label eq '_start';
    return $label if $label =~ /^_fn_/;
    return undef;
}

sub _label_name {
    my ($instruction) = @_;
    return undef unless $instruction->{opcode} == CodingAdventures::CompilerIr::IrOp::LABEL;
    return undef unless @{ $instruction->{operands} || [] };
    return undef unless blessed($instruction->{operands}[0])
                    && $instruction->{operands}[0]->isa('CodingAdventures::CompilerIr::IrLabel');
    return $instruction->{operands}[0]{name};
}

sub _operand_label_name {
    my ($operand) = @_;
    return '' unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrLabel');
    return $operand->{name};
}

sub _label_name_from_operand {
    my ($operand) = @_;
    return undef unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrLabel');
    return $operand->{name};
}

sub _expect_register {
    my ($operand, $context) = @_;
    croak "CodingAdventures::IrToWasmCompiler: $context expected register"
        unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrRegister');
    return $operand;
}

sub _expect_immediate {
    my ($operand, $context) = @_;
    croak "CodingAdventures::IrToWasmCompiler: $context expected immediate"
        unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrImmediate');
    return $operand;
}

sub _expect_label {
    my ($operand, $context) = @_;
    croak "CodingAdventures::IrToWasmCompiler: $context expected label"
        unless blessed($operand) && $operand->isa('CodingAdventures::CompilerIr::IrLabel');
    return $operand;
}

sub _align_up {
    my ($value, $alignment) = @_;
    return int(($value + $alignment - 1) / $alignment) * $alignment;
}

sub _max {
    my ($a, $b) = @_;
    return $a > $b ? $a : $b;
}

sub _func_type_key {
    my ($func_type) = @_;
    return join(',', @{ $func_type->{params} || [] }) . '->' . join(',', @{ $func_type->{results} || [] });
}

1;

__END__

=head1 NAME

CodingAdventures::IrToWasmCompiler - lower generic IR into WebAssembly 1.0 modules

=head1 SYNOPSIS

    use CodingAdventures::IrToWasmCompiler qw(compile new_function_signature);

    my $module = compile(
        $ir_program,
        [ new_function_signature('_start', 0, '_start') ],
    );

=head1 DESCRIPTION

Lowers the repo's generic compiler IR into a plain Perl Wasm module structure.

=cut
