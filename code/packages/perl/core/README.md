# CodingAdventures::Core (Perl)

CPU core that integrates pipeline, register file, memory controller, and ISA decoder.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::Core;
use CodingAdventures::CpuPipeline;
use CodingAdventures::CpuSimulator;

# Implement a minimal ISA decoder
package MyDecoder;
sub new { bless {}, shift }
sub decode {
    my ($self, $raw, $token) = @_;
    $token->{opcode} = $raw == 0xFF ? 'HALT' : 'NOP';
    $token->{is_halt} = 1 if $raw == 0xFF;
    return $token;
}
sub execute         { return $_[1] }
sub instruction_size { 4 }

package main;

my $result = CodingAdventures::Core::Core->new(
    CodingAdventures::Core::CoreConfig->simple(),
    MyDecoder->new()
);
my $core = $result->{core};
$core->load_program([0xFF, 0, 0, 0], 0);  # HALT program
$core->run(100);
printf "Halted: %d\n",  $core->is_halted();  # 1
printf "IPC: %.3f\n",   $core->get_stats()->ipc();
```

## Dependencies

- `CodingAdventures::CpuPipeline` — provides the pipeline engine
- `CodingAdventures::CpuSimulator` — provides Memory and RegisterFile
