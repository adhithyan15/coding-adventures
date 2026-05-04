package CodingAdventures::CodegenCore;
use strict;
use warnings;

our $VERSION = '0.01';

sub _format_value {
    my ($value) = @_;
    return 'nil' unless defined $value;
    return $value =~ /^-?\d+(?:\.\d+)?$/ ? $value : '"' . $value . '"';
}

sub _format_instr {
    my ($instr) = @_;
    my $dest = defined $instr->{dest} ? "$instr->{dest} = " : '';
    my $args = join(', ', map { _format_value($_) } @{ $instr->{srcs} });
    my $type = defined $instr->{type_hint} ? " : $instr->{type_hint}" : '';
    return "$dest$instr->{op}($args)$type";
}

package CodingAdventures::CodegenCore::TextBackend;
use strict;
use warnings;
sub new { bless { target => $_[1] }, $_[0] }
sub compile {
    my ($self, $mod) = @_;
    my @lines = ("; LANG target=$self->{target} module=$mod->{name} language=$mod->{language}", ".entry $mod->{entry_point}");
    for my $fn (@{ $mod->{functions} }) {
        my $params = join(' ', map { "$_->{name}:$_->{type}" } @{ $fn->{params} });
        push @lines, '', '.function ' . $fn->{name} . ($params ? " $params" : '') . " -> $fn->{return_type}";
        for my $i (0..$#{ $fn->{instructions} }) {
            push @lines, sprintf('  %04d  %s', $i, CodingAdventures::CodegenCore::_format_instr($fn->{instructions}[$i]));
        }
        push @lines, '.end';
    }
    return { target => $self->{target}, format => "$self->{target}-lang-ir-text", body => join("\n", @lines) . "\n", metadata => { functions => $mod->function_names, entry_point => $mod->{entry_point} } };
}

package CodingAdventures::CodegenCore::BackendRegistry;
use strict;
use warnings;
sub new { bless { backends => {}, order => [] }, $_[0] }
sub default {
    my $class = shift;
    my $self = $class->new;
    $self->register(CodingAdventures::CodegenCore::TextBackend->new($_)) for qw(pure_vm jvm clr wasm);
    return $self;
}
sub register {
    my ($self, $backend) = @_;
    push @{ $self->{order} }, $backend->{target} unless exists $self->{backends}{ $backend->{target} };
    $self->{backends}{ $backend->{target} } = $backend;
}
sub fetch {
    my ($self, $target) = @_;
    die "unknown backend target: $target" unless exists $self->{backends}{$target};
    return $self->{backends}{$target};
}
sub compile { $_[0]->fetch($_[2])->compile($_[1]) }
sub targets { [ @{ $_[0]->{order} } ] }

package CodingAdventures::CodegenCore;
1;
