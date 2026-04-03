# CodingAdventures::CpuPipeline (Perl)

Configurable N-stage CPU instruction pipeline simulator.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```perl
use CodingAdventures::CpuPipeline;

my $config = CodingAdventures::CpuPipeline->classic_5_stage();
my $result = CodingAdventures::CpuPipeline::Pipeline->new(
    $config,
    sub { 0 },        # fetch:     (pc) → raw instruction
    sub { $_[1] },    # decode:    (raw, token) → decoded token
    sub { $_[0] },    # execute:   (token) → token with alu_result
    sub { $_[0] },    # memory:    (token) → token with mem_data
    sub { },          # writeback: (token) → void
);
my $p = $result->{pipeline};
$p->run(10);
printf "IPC: %.3f\n", $p->stats()->ipc();
```

## Description

A CPU pipeline overlaps the execution of multiple instructions. The classic
5-stage RISC pipeline (IF→ID→EX→MEM→WB) achieves up to 5x throughput
improvement over a single-cycle design.

See `code/specs/D04-pipeline.md` for the full specification.
