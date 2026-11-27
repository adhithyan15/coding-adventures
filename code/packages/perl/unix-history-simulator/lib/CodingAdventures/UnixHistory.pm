package CodingAdventures::UnixHistory::FileSystem;
use strict;
use warnings;
use Encode qw(encode_utf8);

sub demo {
    my ($class) = @_;
    return bless {
        1 => { entries => [[hello => 2], [empty => 3], [etc => 4]] },
        2 => { data => "Hello, Unix!\n" },
        3 => { data => "" },
        4 => { entries => [[notes => 5]] },
        5 => { data => "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n" },
    }, $class;
}

sub resolve {
    my ($self, $path) = @_;
    return (undef, 'EINVAL') if $path eq '' || index($path, "\0") != -1;
    my $inode = 1;
    my @parents;
    for my $name (grep { length } split m{/}, $path) {
        my $entries = $self->{$inode}{entries};
        return (undef, 'ENOTDIR') unless $entries;
        if ($name eq '.') {
            next;
        } elsif ($name eq '..') {
            $inode = @parents ? pop @parents : 1;
        } else {
            my ($entry) = grep { $_->[0] eq $name } @$entries;
            return (undef, 'ENOENT') unless $entry;
            push @parents, $inode;
            $inode = $entry->[1];
        }
    }
    return (undef, 'ENOTDIR') if $path =~ m{/$} && !$self->{$inode}{entries};
    return ($inode, undef);
}

sub entries {
    my ($self, $inode) = @_;
    return $self->{$inode}{entries};
}

sub read {
    my ($self, $inode, $offset, $count) = @_;
    my $data = $self->{$inode}{data};
    return (undef, 'EINVAL')
        unless defined($offset) && $offset =~ /\A\d+\z/
            && defined($count) && $count =~ /\A\d+\z/;
    my $bytes = encode_utf8($data);
    return (substr($bytes, $offset, $count), undef);
}

package CodingAdventures::UnixHistory::Descriptors;
use strict;
use warnings;

sub new {
    my ($class, $capacity) = @_;
    return bless [(undef) x $capacity], $class;
}

sub get {
    my ($self, $fd) = @_;
    return ($fd >= 0 && $fd < @$self && $self->[$fd]) ? { %{$self->[$fd]} } : undef;
}

sub allocate {
    my ($self, $inode) = @_;
    for my $fd (0 .. $#$self) {
        next if defined $self->[$fd];
        $self->[$fd] = { inode => $inode, offset => 0 };
        return $fd;
    }
    return undef;
}

sub advance {
    my ($self, $fd, $bytes) = @_;
    return 0 unless $self->get($fd) && $bytes >= 0;
    $self->[$fd]{offset} += $bytes;
    return 1;
}

sub close {
    my ($self, $fd) = @_;
    return 0 unless $self->get($fd);
    $self->[$fd] = undef;
    return 1;
}

package CodingAdventures::UnixHistory::Kernel;
use strict;
use warnings;

sub new {
    my ($class, $files, $descriptors) = @_;
    return bless { files => $files, descriptors => $descriptors }, $class;
}

sub open {
    my ($self, $path) = @_;
    my ($inode, $error) = $self->{files}->resolve($path);
    my $event = { call => 'open', path => $path };
    return { %$event, error => $error } if defined $error;
    return { %$event, error => 'EISDIR' } if $self->{files}->entries($inode);
    my $fd = $self->{descriptors}->allocate($inode);
    return { %$event, error => 'EMFILE' } unless defined $fd;
    return { %$event, fd => $fd, inode => $inode, offset => 0 };
}

sub read {
    my ($self, $fd, $count) = @_;
    my $slot = $self->{descriptors}->get($fd);
    my $event = { call => 'read', fd => $fd };
    return ({ %$event, error => 'EBADF' }, '') unless $slot;
    my ($data, $error) = $self->{files}->read($slot->{inode}, $slot->{offset}, $count);
    return ({ %$event, error => $error }, '') if defined $error;
    my $bytes = length($data);
    $self->{descriptors}->advance($fd, $bytes);
    return ({ %$event, %$slot, bytes => $bytes, nextOffset => $slot->{offset} + $bytes }, $data);
}

sub close {
    my ($self, $fd) = @_;
    my $event = { call => 'close', fd => $fd };
    return $self->{descriptors}->close($fd) ? $event : { %$event, error => 'EBADF' };
}

sub list {
    my ($self, $path) = @_;
    my ($inode, $error) = $self->{files}->resolve($path);
    my $event = { call => 'list', path => $path };
    return { %$event, error => $error } if defined $error;
    my $entries = $self->{files}->entries($inode);
    return { %$event, error => 'ENOTDIR' } unless $entries;
    return { %$event, inode => $inode, names => [map { $_->[0] } @$entries] };
}

package CodingAdventures::UnixHistory;
use strict;
use warnings;

sub run_command {
    my ($command) = @_;
    my $kernel = CodingAdventures::UnixHistory::Kernel->new(
        CodingAdventures::UnixHistory::FileSystem->demo,
        CodingAdventures::UnixHistory::Descriptors->new(4)
    );
    my @words = grep { length } split /[ \t\n\r\f\x0b]+/, $command;
    my @events;
    my $output = '';
    if (@words == 2 && $words[0] eq 'cat') {
        my $opened = $kernel->open($words[1]);
        push @events, $opened;
        if (defined(my $fd = $opened->{fd})) {
            while (1) {
                my ($read, $data) = $kernel->read($fd, 8);
                push @events, $read;
                last if $data eq '';
                $output .= $data;
            }
            push @events, $kernel->close($fd);
        }
    } elsif (@words && @words <= 2 && $words[0] eq 'ls') {
        my $listed = $kernel->list($words[1] // '/');
        push @events, $listed;
        $output .= "$_\n" for @{$listed->{names} // []};
    } elsif (@words) {
        $output = "usage: cat PATH or ls [PATH]\n";
    }
    for my $event (@events) {
        $output .= "error: $event->{error}\n" if defined $event->{error};
    }
    return { output => $output, events => \@events };
}

1;
