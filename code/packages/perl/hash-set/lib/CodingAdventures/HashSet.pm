package CodingAdventures::HashSet;

use strict;
use warnings;
use Exporter qw(import);
use Scalar::Util qw(blessed);
use CodingAdventures::HashMap;

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(
    new_set with_options from_list from_list_with_options
    add remove discard contains has size is_empty to_list
    union intersection difference symmetric_difference
    is_subset is_superset is_disjoint equals
);

my $PRESENT = 1;

sub _from_map {
    my ($invocant, $map) = @_;
    my $class = ref($invocant) || $invocant;
    return bless { map => $map }, $class;
}

sub _require_set {
    my ($value, $name) = @_;
    die "$name must be a CodingAdventures::HashSet\n"
        unless blessed($value) && $value->isa(__PACKAGE__);
    return $value;
}

sub new {
    my ($class, %args) = @_;
    my %map_args;
    $map_args{capacity} = $args{capacity} if exists $args{capacity};
    $map_args{strategy} = $args{strategy} if exists $args{strategy};
    $map_args{hash_fn} = $args{hash_fn} if exists $args{hash_fn};
    return $class->_from_map(CodingAdventures::HashMap->new(%map_args));
}

sub new_set {
    return __PACKAGE__->new(@_);
}

sub with_options {
    my ($first, @rest) = @_;
    my ($class, $capacity, $strategy, $hash_fn);
    if (!ref($first) && defined($first) && $first eq __PACKAGE__) {
        $class = $first;
        ($capacity, $strategy, $hash_fn) = @rest;
    } else {
        $class = __PACKAGE__;
        ($capacity, $strategy, $hash_fn) = ($first, @rest);
    }
    return $class->new(
        capacity => $capacity,
        strategy => $strategy,
        hash_fn  => $hash_fn,
    );
}

sub from_list {
    my ($first, @rest) = @_;
    my ($class, $elements);
    if (!ref($first) && defined($first) && $first eq __PACKAGE__) {
        $class = $first;
        $elements = shift @rest;
    } else {
        $class = __PACKAGE__;
        $elements = $first;
    }
    die "elements must be an array reference\n" unless ref($elements) eq 'ARRAY';
    die "options must be key-value pairs\n" if @rest % 2;
    my $set = $class->new(@rest);
    for my $element (@{$elements}) {
        die "element must be defined\n" unless defined $element;
        $set->{map}->set($element, $PRESENT);
    }
    return $set;
}

sub from_list_with_options {
    my ($first, @rest) = @_;
    my ($class, $elements);
    if (!ref($first) && defined($first) && $first eq __PACKAGE__) {
        $class = $first;
        $elements = shift @rest;
    } else {
        $class = __PACKAGE__;
        $elements = $first;
    }
    my ($capacity, $strategy, $hash_fn) = @rest;
    return $class->from_list(
        $elements,
        capacity => $capacity,
        strategy => $strategy,
        hash_fn  => $hash_fn,
    );
}

sub clone {
    my ($self) = @_;
    return $self->_from_map($self->{map}->clone);
}

sub add {
    my ($self, $element) = @_;
    die "element must be defined\n" unless defined $element;
    return $self->_from_map($self->{map}->with_set($element, $PRESENT));
}

sub remove {
    my ($self, $element) = @_;
    return $self->_from_map($self->{map}->without($element));
}

sub discard {
    my ($self, $element) = @_;
    return $self->remove($element);
}

sub contains {
    my ($self, $element) = @_;
    return $self->{map}->has($element);
}

sub has { return shift->contains(@_); }
sub size { return $_[0]->{map}->size; }
sub len { return $_[0]->size; }
sub is_empty { return $_[0]->size == 0; }
sub to_list { return $_[0]->{map}->keys; }
sub capacity { return $_[0]->{map}->capacity; }
sub strategy { return $_[0]->{map}->strategy; }
sub hash_fn { return $_[0]->{map}->hash_fn; }

sub _empty_like {
    my ($self) = @_;
    return $self->_from_map(CodingAdventures::HashMap->new(
        capacity => $self->capacity,
        strategy => $self->strategy,
        hash_fn  => $self->hash_fn,
    ));
}

sub union {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    my $result = $self->clone;
    $result->{map}->set($_, $PRESENT) for @{$other->to_list};
    return $result;
}

sub intersection {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    my ($smaller, $larger) = $self->size <= $other->size
        ? ($self, $other)
        : ($other, $self);
    my $result = $self->_empty_like;
    for my $element (@{$smaller->to_list}) {
        $result->{map}->set($element, $PRESENT) if $larger->contains($element);
    }
    return $result;
}

sub difference {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    my $result = $self->_empty_like;
    for my $element (@{$self->to_list}) {
        $result->{map}->set($element, $PRESENT) unless $other->contains($element);
    }
    return $result;
}

sub symmetric_difference {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    my $result = $self->_empty_like;
    for my $element (@{$self->to_list}) {
        $result->{map}->set($element, $PRESENT) unless $other->contains($element);
    }
    for my $element (@{$other->to_list}) {
        $result->{map}->set($element, $PRESENT) unless $self->contains($element);
    }
    return $result;
}

sub is_subset {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    return 0 if $self->size > $other->size;
    for my $element (@{$self->to_list}) {
        return 0 unless $other->contains($element);
    }
    return 1;
}

sub is_superset {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    return $other->is_subset($self);
}

sub is_disjoint {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    my ($smaller, $larger) = $self->size <= $other->size
        ? ($self, $other)
        : ($other, $self);
    for my $element (@{$smaller->to_list}) {
        return 0 if $larger->contains($element);
    }
    return 1;
}

sub equals {
    my ($self, $other) = @_;
    _require_set($other, 'other');
    return $self->size == $other->size && $self->is_subset($other);
}

1;

__END__

=head1 NAME

CodingAdventures::HashSet - persistent DT19 hash set

=head1 SYNOPSIS

  use CodingAdventures::HashSet qw(from_list);
  my $base = from_list([qw(Ada Grace Ada)]);
  my $next = $base->add('Linus');
  die unless $base->size == 2 && $next->contains('Linus');

=head1 DESCRIPTION

Wraps the sibling DT18 hash map with a keys-only persistent set abstraction.
Includes full set algebra, relation predicates, option preservation, and
reference-identity semantics.

=cut
