package CodingAdventures::FPGA::Fabric;
use strict;
use warnings;
use CodingAdventures::FPGA::CLB;
use CodingAdventures::FPGA::SwitchMatrix;
use CodingAdventures::FPGA::IOBlock;
our $VERSION = '0.01';

# Fabric — the full FPGA: rows×cols CLBs + switch matrices + perimeter I/O.
#
# I/O naming:
#   top_{c}    (c=0..cols-1): input blocks
#   bottom_{c} (c=0..cols-1): output blocks
#   left_{r}   (r=0..rows-1): input blocks
#   right_{r}  (r=0..rows-1): output blocks

sub new {
    my ($class, $rows, $cols, %opts) = @_;
    my $n = $opts{lut_inputs} // 4;

    # CLB grid: $clbs[$r][$c]
    my @clbs;
    for my $r (0 .. $rows-1) {
        for my $c (0 .. $cols-1) {
            $clbs[$r][$c] = CodingAdventures::FPGA::CLB->new($r, $c, lut_inputs => $n);
        }
    }

    # Switch matrix per CLB (8 inputs, 4 outputs)
    my @switch_matrices;
    for my $r (0 .. $rows-1) {
        for my $c (0 .. $cols-1) {
            $switch_matrices[$r][$c] = CodingAdventures::FPGA::SwitchMatrix->new(8, 4);
        }
    }

    # Perimeter I/O blocks
    my %io_blocks;
    for my $c (0 .. $cols-1) { $io_blocks{"top_$c"}    = CodingAdventures::FPGA::IOBlock->new("top_$c",    'input');  }
    for my $c (0 .. $cols-1) { $io_blocks{"bottom_$c"} = CodingAdventures::FPGA::IOBlock->new("bottom_$c", 'output'); }
    for my $r (0 .. $rows-1) { $io_blocks{"left_$r"}   = CodingAdventures::FPGA::IOBlock->new("left_$r",   'input');  }
    for my $r (0 .. $rows-1) { $io_blocks{"right_$r"}  = CodingAdventures::FPGA::IOBlock->new("right_$r",  'output'); }

    return bless {
        rows            => $rows,
        cols            => $cols,
        lut_inputs      => $n,
        clbs            => \@clbs,
        switch_matrices => \@switch_matrices,
        io_blocks       => \%io_blocks,
        clb_outputs     => {},
    }, $class;
}

sub load_bitstream {
    my ($self, $bs) = @_;
    for my $r (0 .. $self->{rows}-1) {
        for my $c (0 .. $self->{cols}-1) {
            my $key = "$r,$c";
            my $clb_cfg = $bs->clb_config($key);
            $self->{clbs}[$r][$c]->configure($clb_cfg) if $clb_cfg;
            my $r_cfg = $bs->routing_config($key);
            $self->{switch_matrices}[$r][$c]->configure($r_cfg) if $r_cfg;
        }
    }
}

sub set_input {
    my ($self, $pin_name, $value) = @_;
    die "unknown I/O block: $pin_name" unless exists $self->{io_blocks}{$pin_name};
    $self->{io_blocks}{$pin_name}->set_pin($value);
}

sub read_output {
    my ($self, $pin_name) = @_;
    die "unknown I/O block: $pin_name" unless exists $self->{io_blocks}{$pin_name};
    return $self->{io_blocks}{$pin_name}->read_pin();
}

sub evaluate {
    my ($self, $clock) = @_;

    for my $r (0 .. $self->{rows}-1) {
        for my $c (0 .. $self->{cols}-1) {
            # Build 8 input signals for switch matrix
            my %sigs;
            $sigs{"in_0"} = ($self->{io_blocks}{"top_$c"}  ? $self->{io_blocks}{"top_$c"}->read_fabric()  : 0) // 0;
            $sigs{"in_1"} = ($self->{io_blocks}{"left_$r"} ? $self->{io_blocks}{"left_$r"}->read_fabric() : 0) // 0;
            for my $i (2..7) { $sigs{"in_$i"} = 0; }

            my $routed = $self->{switch_matrices}[$r][$c]->route(\%sigs);

            my $make_inputs = sub {
                my $v = $_[0] // 0;
                return [($v) x $self->{lut_inputs}];
            };

            my $clb_inputs = {
                s0_a => $make_inputs->($routed->{"out_0"}),
                s0_b => $make_inputs->($routed->{"out_1"}),
                s1_a => $make_inputs->($routed->{"out_2"}),
                s1_b => $make_inputs->($routed->{"out_3"}),
            };

            my ($outputs, $_carry) = $self->{clbs}[$r][$c]->evaluate($clb_inputs, $clock, 0);
            $self->{clb_outputs}{"$r,$c"} = $outputs;
        }
    }

    # Drive output I/O blocks
    for my $c (0 .. $self->{cols}-1) {
        my $key = ($self->{rows}-1) . ",$c";
        my $outs = $self->{clb_outputs}{$key};
        if ($outs && exists $self->{io_blocks}{"bottom_$c"}) {
            $self->{io_blocks}{"bottom_$c"}->set_fabric($outs->[0]);
        }
    }
    for my $r (0 .. $self->{rows}-1) {
        my $key = "$r," . ($self->{cols}-1);
        my $outs = $self->{clb_outputs}{$key};
        if ($outs && exists $self->{io_blocks}{"right_$r"}) {
            $self->{io_blocks}{"right_$r"}->set_fabric($outs->[0]);
        }
    }
}

sub summary {
    my ($self) = @_;
    my $io_count = 2 * ($self->{rows} + $self->{cols});
    return sprintf("FPGA Fabric %dx%d\n  CLBs: %d\n  I/O Blocks: %d",
        $self->{rows}, $self->{cols}, $self->{rows} * $self->{cols}, $io_count);
}

1;
