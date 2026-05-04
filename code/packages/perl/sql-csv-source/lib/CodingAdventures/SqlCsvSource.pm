package CodingAdventures::SqlCsvSource;

use strict;
use warnings;
use Exporter 'import';

use CodingAdventures::SqlCsvSource::CsvDataSource;
use CodingAdventures::SqlExecutionEngine;

our $VERSION = '0.01';
our @EXPORT_OK = qw(coerce execute_csv);

sub coerce {
    return CodingAdventures::SqlCsvSource::CsvDataSource::coerce(@_);
}

sub execute_csv {
    my ($sql, $directory) = @_;
    my $source = CodingAdventures::SqlCsvSource::CsvDataSource->new($directory);
    return CodingAdventures::SqlExecutionEngine->execute($sql, $source);
}

1;

__END__

=head1 NAME

CodingAdventures::SqlCsvSource - CSV-backed data source for the SQL execution engine

=head1 SYNOPSIS

  use CodingAdventures::SqlCsvSource qw(execute_csv);

  my ($ok, $result) = execute_csv(
      'SELECT name FROM employees WHERE dept_id IS NULL',
      'data',
  );

=head1 DESCRIPTION

Provides a CSV data-source adapter for CodingAdventures::SqlExecutionEngine.
Each table maps to a C<tablename.csv> file in the configured directory.

=cut
