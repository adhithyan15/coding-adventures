package CodingAdventures::SqlCsvSource::CsvDataSource;

use strict;
use warnings;

use File::Spec;
use CodingAdventures::CsvParser;

sub new {
    my ($class, $directory) = @_;
    return bless { directory => "$directory" }, $class;
}

sub schema {
    my ($self, $table_name) = @_;
    my $rows = $self->_parse_table($table_name);
    return [] unless @$rows;
    return [ map { _trim($_) } @{$rows->[0]} ];
}

sub scan {
    my ($self, $table_name) = @_;
    my $rows = $self->_parse_table($table_name);
    return [] unless @$rows;

    my @columns = map { _trim($_) } @{shift @$rows};
    my @records;
    for my $parsed_row (@$rows) {
        my %record;
        for my $index (0 .. $#columns) {
            my $value = $parsed_row->[$index] // '';
            $record{$columns[$index]} = coerce($value);
        }
        push @records, \%record;
    }

    return \@records;
}

sub coerce {
    my ($value) = @_;
    return undef if !defined($value) || $value eq '';

    my $lower = lc $value;
    return 1 if $lower eq 'true';
    return 0 if $lower eq 'false';
    return 0 + $value if $value =~ /\A-?\d+\z/;
    return 0.0 + $value if $value =~ /\A-?\d+\.\d+\z/;
    return $value;
}

sub _parse_table {
    my ($self, $table_name) = @_;
    my $path = $self->_resolve($table_name);
    my $content = _read_file($path);
    my $parser = CodingAdventures::CsvParser->new;
    return $parser->parse($content);
}

sub _resolve {
    my ($self, $table_name) = @_;
    my $path = File::Spec->catfile($self->{directory}, "$table_name.csv");
    die "table not found: $table_name\n" unless -e $path;
    return $path;
}

sub _read_file {
    my ($path) = @_;
    open my $fh, '<:encoding(UTF-8)', $path or die "failed to read $path: $!\n";
    local $/;
    return <$fh>;
}

sub _trim {
    my ($value) = @_;
    $value //= '';
    $value =~ s/\A\s+//;
    $value =~ s/\s+\z//;
    return $value;
}

1;

__END__

=head1 NAME

CodingAdventures::SqlCsvSource::CsvDataSource - CSV data-source adapter

=head1 DESCRIPTION

Implements the C<schema> and C<scan> methods expected by
CodingAdventures::SqlExecutionEngine. CSV headers define table columns and
data rows are coerced to SQL-friendly Perl scalars.

=cut
