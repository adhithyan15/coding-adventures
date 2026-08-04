package CodingAdventures::Trie;

use strict;
use warnings;
use utf8;
use overload '""' => 'as_string', fallback => 1;

our $VERSION = '0.1.0';

sub _new_node {
    return {
        children => {},
        terminal => 0,
        value    => undef,
    };
}

sub _assert_string {
    my ($value, $name) = @_;
    die "$name must be a string\n" if !defined($value) || ref($value);
}

sub new {
    my ($class, $entries) = @_;
    $entries = [] if !defined $entries;
    die "entries must be an array reference\n" if ref($entries) ne 'ARRAY';

    my $self = bless {
        root => _new_node(),
        size => 0,
    }, $class;

    for my $index (0 .. $#$entries) {
        my $entry = $entries->[$index];
        die "entry at index $index must contain a key\n"
            if ref($entry) ne 'ARRAY' || !@$entry;
        my $value = @$entry >= 2 ? $entry->[1] : 1;
        $self->insert($entry->[0], $value);
    }
    return $self;
}

sub insert {
    my ($self, $key) = @_;
    my $value = @_ >= 3 ? $_[2] : 1;
    _assert_string($key, 'key');

    my $node = $self->{root};
    for my $character (split //u, $key) {
        $node->{children}{$character} //= _new_node();
        $node = $node->{children}{$character};
    }

    $self->{size}++ if !$node->{terminal};
    $node->{terminal} = 1;
    $node->{value} = $value;
    return $self;
}

sub _find_node {
    my ($self, $key) = @_;
    my $node = $self->{root};
    for my $character (split //u, $key) {
        return undef if !exists $node->{children}{$character};
        $node = $node->{children}{$character};
    }
    return $node;
}

sub search {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    my $node = $self->_find_node($key);
    return defined($node) && $node->{terminal} ? $node->{value} : undef;
}

sub contains_key {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    my $node = $self->_find_node($key);
    return defined($node) && $node->{terminal} ? 1 : 0;
}

sub key_exists { return shift->contains_key(@_); }
sub contains   { return shift->contains_key(@_); }

sub _delete_recursive {
    my ($node, $characters, $depth) = @_;
    if ($depth >= @$characters) {
        $node->{terminal} = 0;
        $node->{value} = undef;
        return !keys %{$node->{children}};
    }

    my $character = $characters->[$depth];
    my $child = $node->{children}{$character};
    if (defined($child) && _delete_recursive($child, $characters, $depth + 1)) {
        delete $node->{children}{$character};
    }
    return !keys(%{$node->{children}}) && !$node->{terminal};
}

sub delete {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    return 0 if !$self->contains_key($key);

    my @characters = split //u, $key;
    _delete_recursive($self->{root}, \@characters, 0);
    $self->{size}--;
    return 1;
}

sub starts_with {
    my ($self, $prefix) = @_;
    _assert_string($prefix, 'prefix');
    return $self->{size} > 0 ? 1 : 0 if $prefix eq '';
    return defined($self->_find_node($prefix)) ? 1 : 0;
}

sub _collect {
    my ($node, $current, $results) = @_;
    push @$results, [$current, $node->{value}] if $node->{terminal};
    for my $character (sort keys %{$node->{children}}) {
        _collect($node->{children}{$character}, $current . $character, $results);
    }
}

sub words_with_prefix {
    my ($self, $prefix) = @_;
    _assert_string($prefix, 'prefix');
    my $node = $self->_find_node($prefix);
    return [] if !defined $node;

    my $results = [];
    _collect($node, $prefix, $results);
    return $results;
}

sub all_words {
    my ($self) = @_;
    my $results = [];
    _collect($self->{root}, '', $results);
    return $results;
}

sub entries  { return shift->all_words(@_); }
sub to_array { return shift->all_words(@_); }

sub keys {
    my ($self) = @_;
    return [map { $_->[0] } @{$self->all_words}];
}

sub longest_prefix_match {
    my ($self, $input) = @_;
    _assert_string($input, 'input');
    my $node = $self->{root};
    my $current = '';
    my $best = $node->{terminal} ? ['', $node->{value}] : undef;

    for my $character (split //u, $input) {
        last if !exists $node->{children}{$character};
        $current .= $character;
        $node = $node->{children}{$character};
        $best = [$current, $node->{value}] if $node->{terminal};
    }
    return $best;
}

sub each {
    my ($self, $callback) = @_;
    die "callback must be a code reference\n" if ref($callback) ne 'CODE';
    $callback->(@$_) for @{$self->all_words};
    return $self;
}

sub size   { return $_[0]->{size}; }
sub length { return $_[0]->{size}; }

sub is_empty {
    my ($self) = @_;
    return $self->{size} == 0 ? 1 : 0;
}

sub _count_endpoints {
    my ($node) = @_;
    my $count = $node->{terminal} ? 1 : 0;
    $count += _count_endpoints($_) for values %{$node->{children}};
    return $count;
}

sub is_valid {
    my ($self) = @_;
    return _count_endpoints($self->{root}) == $self->{size} ? 1 : 0;
}

sub as_string {
    my ($self) = @_;
    return sprintf('Trie(%d keys)', $self->{size});
}

1;

__END__

=head1 NAME

CodingAdventures::Trie - prefix trie with sorted enumeration

=head1 SYNOPSIS

  my $trie = CodingAdventures::Trie->new;
  $trie->insert('app', 1)->insert('apple', 2);
  my $match = $trie->longest_prefix_match('apples');

=cut
