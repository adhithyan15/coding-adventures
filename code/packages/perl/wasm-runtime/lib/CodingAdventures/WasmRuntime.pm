package CodingAdventures::WasmRuntime;

# ============================================================================
# CodingAdventures::WasmRuntime — WebAssembly 1.0 wasm-runtime
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
##
# Usage:
#
#   use CodingAdventures::WasmRuntime;
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::WasmLeb128;
use CodingAdventures::WasmTypes;
use CodingAdventures::WasmOpcodes;
use CodingAdventures::WasmModuleParser;
use CodingAdventures::VirtualMachine;

# TODO: Implement WasmRuntime

1;

__END__

=head1 NAME

CodingAdventures::WasmRuntime - WebAssembly 1.0 wasm-runtime

=head1 SYNOPSIS

    use CodingAdventures::WasmRuntime;

=head1 DESCRIPTION

WebAssembly 1.0 wasm-runtime

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
