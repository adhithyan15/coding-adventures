# CodingAdventures::CpuPipeline (Perl)

Configurable N-stage CPU instruction pipeline.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::CpuPipeline;

my @mem = (0xFF, (0x01) x 255);  # HALT at address 0

my $result = CodingAdventures::CpuPipeline::Pipeline->new(
    CodingAdventures::CpuPipeline::PipelineConfig->classic_5_stage(),
    sub { $mem[$_[0]] // 0 },           # fetch
    sub { my ($raw, $tok) = @_;         # decode
          $tok->{opcode}  = $raw == 0xFF ? 'HALT' : 'NOP';
          $tok->{is_halt} = $raw == 0xFF ? 1 : 0;
          $tok },
    sub { $_[0] },   # execute
    sub { $_[0] },   # memory
    sub { },         # writeback
);
my $p = $result->{pipeline};
$p->run(100);
printf "Halted: %d\n", $p->is_halted();
printf "IPC: %.3f\n",  $p->get_stats()->ipc();
```

## Dependencies

None (no external Perl dependencies).
