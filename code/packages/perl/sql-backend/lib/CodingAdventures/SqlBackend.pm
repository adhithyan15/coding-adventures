package CodingAdventures::SqlBackend;

use strict;
use warnings;
use Exporter 'import';
use Scalar::Util qw(blessed looks_like_number);

our $VERSION = '0.01';
our @EXPORT_OK = qw(
    blob is_sql_value sql_type_name compare_sql_values copy_row
    column_def index_def trigger_def backend_as_schema_provider
);

sub blob {
    return CodingAdventures::SqlBackend::Blob->new(@_);
}

sub column_def {
    return CodingAdventures::SqlBackend::ColumnDef->new(@_);
}

sub index_def {
    return CodingAdventures::SqlBackend::IndexDef->new(@_);
}

sub trigger_def {
    return CodingAdventures::SqlBackend::TriggerDef->new(@_);
}

sub _is_blob {
    my ($value) = @_;
    return blessed($value) && $value->isa('CodingAdventures::SqlBackend::Blob');
}

sub is_sql_value {
    my ($value) = @_;
    return 1 if !defined $value;
    return 1 if !ref $value;
    return 1 if _is_blob($value);
    return 0;
}

sub sql_type_name {
    my ($value) = @_;
    return 'NULL' if !defined $value;
    return 'BLOB' if _is_blob($value);
    if (!ref $value && looks_like_number($value)) {
        return "$value" =~ /\A-?\d+\z/ ? 'INTEGER' : 'REAL';
    }
    return 'TEXT' if !ref $value;
    _raise(_internal('not a SqlValue'));
}

sub _type_rank {
    my ($value) = @_;
    return 0 if !defined $value;
    return 4 if _is_blob($value);
    return looks_like_number($value) ? 2 : 3 if !ref $value;
    _raise(_internal('not a SqlValue'));
}

sub compare_sql_values {
    my ($left, $right) = @_;
    my $left_rank  = _type_rank($left);
    my $right_rank = _type_rank($right);
    return $left_rank <=> $right_rank if $left_rank != $right_rank;
    return 0 if !defined $left && !defined $right;
    my $left_value  = _is_blob($left)  ? $left->{bytes}  : $left;
    my $right_value = _is_blob($right) ? $right->{bytes} : $right;
    return 0 if (!defined $left_value && !defined $right_value) || (defined $left_value && defined $right_value && $left_value eq $right_value);
    return looks_like_number($left_value) && looks_like_number($right_value)
        ? ($left_value <=> $right_value)
        : ($left_value cmp $right_value);
}

sub copy_value {
    my ($value) = @_;
    return blob($value->{bytes}) if _is_blob($value);
    return $value;
}

sub copy_row {
    my ($row) = @_;
    my %copy;
    for my $key (keys %{ $row || {} }) {
        $copy{$key} = copy_value($row->{$key});
    }
    return \%copy;
}

sub backend_as_schema_provider {
    my ($backend) = @_;
    return CodingAdventures::SqlBackend::SchemaProvider->new($backend);
}

sub _raise {
    my ($err) = @_;
    die $err;
}

sub _table_not_found {
    my ($table) = @_;
    return CodingAdventures::SqlBackend::Error::TableNotFound->new(
        kind => 'TableNotFound',
        table => $table,
        message => "table not found: $table",
    );
}

sub _table_already_exists {
    my ($table) = @_;
    return CodingAdventures::SqlBackend::Error::TableAlreadyExists->new(
        kind => 'TableAlreadyExists',
        table => $table,
        message => "table already exists: $table",
    );
}

sub _column_not_found {
    my ($table, $column) = @_;
    return CodingAdventures::SqlBackend::Error::ColumnNotFound->new(
        kind => 'ColumnNotFound',
        table => $table,
        column => $column,
        message => "column not found: $table.$column",
    );
}

sub _column_already_exists {
    my ($table, $column) = @_;
    return CodingAdventures::SqlBackend::Error::ColumnAlreadyExists->new(
        kind => 'ColumnAlreadyExists',
        table => $table,
        column => $column,
        message => "column already exists: $table.$column",
    );
}

sub _constraint_violation {
    my ($table, $column, $message) = @_;
    return CodingAdventures::SqlBackend::Error::ConstraintViolation->new(
        kind => 'ConstraintViolation',
        table => $table,
        column => $column,
        message => $message,
    );
}

sub _unsupported {
    my ($operation) = @_;
    return CodingAdventures::SqlBackend::Error::Unsupported->new(
        kind => 'Unsupported',
        operation => $operation,
        message => "operation not supported: $operation",
    );
}

sub _internal {
    my ($message) = @_;
    return CodingAdventures::SqlBackend::Error::Internal->new(
        kind => 'Internal',
        message => $message,
    );
}

sub _index_already_exists {
    my ($index) = @_;
    return CodingAdventures::SqlBackend::Error::IndexAlreadyExists->new(
        kind => 'IndexAlreadyExists',
        index => $index,
        message => "index already exists: $index",
    );
}

sub _index_not_found {
    my ($index) = @_;
    return CodingAdventures::SqlBackend::Error::IndexNotFound->new(
        kind => 'IndexNotFound',
        index => $index,
        message => "index not found: $index",
    );
}

sub _trigger_already_exists {
    my ($name) = @_;
    return CodingAdventures::SqlBackend::Error::TriggerAlreadyExists->new(
        kind => 'TriggerAlreadyExists',
        name => $name,
        message => "trigger already exists: $name",
    );
}

sub _trigger_not_found {
    my ($name) = @_;
    return CodingAdventures::SqlBackend::Error::TriggerNotFound->new(
        kind => 'TriggerNotFound',
        name => $name,
        message => "trigger not found: $name",
    );
}

sub _normalize_name {
    my ($name) = @_;
    return lc "$name";
}

package CodingAdventures::SqlBackend::Error;

use strict;
use warnings;
use overload '""' => sub { $_[0]->{message} }, fallback => 1;

sub new {
    my ($class, %fields) = @_;
    return bless \%fields, $class;
}

package CodingAdventures::SqlBackend::Error::TableNotFound;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::TableAlreadyExists;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::ColumnNotFound;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::ColumnAlreadyExists;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::ConstraintViolation;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::Unsupported;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::Internal;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::IndexAlreadyExists;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::IndexNotFound;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::TriggerAlreadyExists;
our @ISA = ('CodingAdventures::SqlBackend::Error');
package CodingAdventures::SqlBackend::Error::TriggerNotFound;
our @ISA = ('CodingAdventures::SqlBackend::Error');

package CodingAdventures::SqlBackend::Blob;

use strict;
use warnings;
use overload '""' => sub { $_[0]->{bytes} }, fallback => 1;

sub new {
    my ($class, $bytes) = @_;
    return bless { bytes => defined $bytes ? "$bytes" : '' }, $class;
}

package CodingAdventures::SqlBackend::ColumnDef;

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name => "$opts{name}",
        type_name => defined $opts{type_name} ? "$opts{type_name}" : '',
        not_null => $opts{not_null} ? 1 : 0,
        primary_key => $opts{primary_key} ? 1 : 0,
        unique => $opts{unique} ? 1 : 0,
        autoincrement => $opts{autoincrement} ? 1 : 0,
        has_default => $opts{has_default} || (exists $opts{default} && defined $opts{default}) ? 1 : 0,
        default => $opts{default},
        check_expression => $opts{check_expression},
        foreign_key => $opts{foreign_key},
    }, $class;
}

sub effective_not_null {
    my ($self) = @_;
    return $self->{not_null} || $self->{primary_key};
}

sub effective_unique {
    my ($self) = @_;
    return $self->{unique} || $self->{primary_key};
}

sub clone {
    my ($self) = @_;
    return __PACKAGE__->new(%$self, default => CodingAdventures::SqlBackend::copy_value($self->{default}));
}

package CodingAdventures::SqlBackend::IndexDef;

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name => "$opts{name}",
        table => "$opts{table}",
        columns => [ @{ $opts{columns} || [] } ],
        unique => $opts{unique} ? 1 : 0,
        auto => $opts{auto} ? 1 : 0,
    }, $class;
}

sub clone {
    my ($self) = @_;
    return __PACKAGE__->new(%$self, columns => [ @{ $self->{columns} } ]);
}

package CodingAdventures::SqlBackend::TriggerDef;

use strict;
use warnings;

sub new {
    my ($class, %opts) = @_;
    return bless {
        name => "$opts{name}",
        table => "$opts{table}",
        timing => uc "$opts{timing}",
        event => uc "$opts{event}",
        body => defined $opts{body} ? "$opts{body}" : '',
    }, $class;
}

sub clone {
    my ($self) = @_;
    return __PACKAGE__->new(%$self);
}

package CodingAdventures::SqlBackend::ListRowIterator;

use strict;
use warnings;

sub new {
    my ($class, $rows) = @_;
    return bless {
        rows => [ map { CodingAdventures::SqlBackend::copy_row($_) } @{ $rows || [] } ],
        index => 0,
        closed => 0,
    }, $class;
}

sub next {
    my ($self) = @_;
    return undef if $self->{closed} || $self->{index} >= @{ $self->{rows} };
    my $row = CodingAdventures::SqlBackend::copy_row($self->{rows}[ $self->{index} ]);
    $self->{index}++;
    return $row;
}

sub close {
    my ($self) = @_;
    $self->{closed} = 1;
    return 1;
}

sub to_array {
    my ($self) = @_;
    my @rows;
    while (my $row = $self->next) {
        push @rows, $row;
    }
    $self->close;
    return \@rows;
}

package CodingAdventures::SqlBackend::ListCursor;

use strict;
use warnings;

sub new {
    my ($class, $rows, $table_key) = @_;
    return bless {
        rows => [ map { CodingAdventures::SqlBackend::copy_row($_) } @{ $rows || [] } ],
        table_key => $table_key,
        index => -1,
    }, $class;
}

sub next {
    my ($self) = @_;
    $self->{index}++;
    return $self->current_row;
}

sub current_row {
    my ($self) = @_;
    return undef if $self->{index} < 0 || $self->{index} >= @{ $self->{rows} };
    return CodingAdventures::SqlBackend::copy_row($self->{rows}[ $self->{index} ]);
}

sub current_index {
    my ($self) = @_;
    return $self->{index};
}

sub adjust_after_delete {
    my ($self) = @_;
    $self->{index}-- if $self->{index} >= 0;
}

package CodingAdventures::SqlBackend::TableCursor;

use strict;
use warnings;

sub new {
    my ($class, $table_key, $state) = @_;
    return bless { table_key => $table_key, state => $state, index => -1 }, $class;
}

sub next {
    my ($self) = @_;
    $self->{index}++;
    return $self->current_row;
}

sub current_record {
    my ($self) = @_;
    return undef if $self->{index} < 0 || $self->{index} >= @{ $self->{state}{rows} };
    return $self->{state}{rows}[ $self->{index} ];
}

sub current_row {
    my ($self) = @_;
    my $record = $self->current_record;
    return $record ? CodingAdventures::SqlBackend::copy_row($record->{row}) : undef;
}

sub current_index {
    my ($self) = @_;
    return $self->{index};
}

sub adjust_after_delete {
    my ($self) = @_;
    $self->{index}-- if $self->{index} >= 0;
}

package CodingAdventures::SqlBackend::SchemaProvider;

use strict;
use warnings;

sub new {
    my ($class, $backend) = @_;
    return bless { backend => $backend }, $class;
}

sub columns {
    my ($self, $table) = @_;
    return [ map { $_->{name} } @{ $self->{backend}->columns($table) } ];
}

sub list_indexes {
    my ($self, $table) = @_;
    return $self->{backend}->list_indexes($table);
}

package CodingAdventures::SqlBackend::InMemoryBackend;

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {
        tables_by_key => {},
        indexes_by_key => {},
        triggers_by_key => {},
        triggers_by_table => {},
        user_version => 0,
        schema_version => 0,
        transaction_snapshot => undef,
        current_transaction => undef,
        next_transaction => 1,
        savepoints => [],
    }, $class;
}

sub _bump_schema_version {
    my ($self) = @_;
    $self->{schema_version}++;
}

sub _table_state {
    my ($self, $table) = @_;
    my $state = $self->{tables_by_key}{ CodingAdventures::SqlBackend::_normalize_name($table) };
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_table_not_found($table)) unless $state;
    return $state;
}

sub _find_column {
    my ($self, $state, $name) = @_;
    my $wanted = CodingAdventures::SqlBackend::_normalize_name($name);
    for my $column (@{ $state->{columns} }) {
        return $column if CodingAdventures::SqlBackend::_normalize_name($column->{name}) eq $wanted;
    }
    return undef;
}

sub _real_column_name {
    my ($self, $state, $name) = @_;
    my $column = $self->_find_column($state, $name);
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_not_found($state->{name}, $name)) unless $column;
    return $column->{name};
}

sub tables {
    my ($self) = @_;
    return [ sort map { $_->{name} } values %{ $self->{tables_by_key} } ];
}

sub columns {
    my ($self, $table) = @_;
    my $state = $self->_table_state($table);
    return [ map { $_->clone } @{ $state->{columns} } ];
}

sub scan {
    my ($self, $table) = @_;
    my $state = $self->_table_state($table);
    return CodingAdventures::SqlBackend::ListRowIterator->new([ map { $_->{row} } @{ $state->{rows} } ]);
}

sub open_cursor {
    my ($self, $table) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($table);
    return CodingAdventures::SqlBackend::TableCursor->new($key, $self->_table_state($table));
}

sub _next_autoincrement_value {
    my ($self, $state, $column) = @_;
    my $max = 0;
    for my $record (@{ $state->{rows} }) {
        my $value = $record->{row}{ $column->{name} };
        $max = $value if defined $value && Scalar::Util::looks_like_number($value) && $value > $max;
    }
    return $max + 1;
}

sub _materialize_row {
    my ($self, $state, $row) = @_;
    my %candidate;
    for my $column (@{ $state->{columns} }) {
        my ($present, $value) = (0, undef);
        for my $key (keys %{ $row || {} }) {
            if (CodingAdventures::SqlBackend::_normalize_name($key) eq CodingAdventures::SqlBackend::_normalize_name($column->{name})) {
                ($present, $value) = (1, $row->{$key});
                last;
            }
        }
        if (!$present) {
            if ($column->{autoincrement} && $column->{primary_key}) {
                $value = $self->_next_autoincrement_value($state, $column);
            } elsif ($column->{has_default}) {
                $value = $column->{default};
            }
        }
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal('not a SqlValue')) unless CodingAdventures::SqlBackend::is_sql_value($value);
        $candidate{ $column->{name} } = CodingAdventures::SqlBackend::copy_value($value);
    }
    for my $key (keys %{ $row || {} }) {
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_not_found($state->{name}, $key)) unless $self->_find_column($state, $key);
    }
    return \%candidate;
}

sub _key_for_index {
    my ($self, $state, $index, $row) = @_;
    return [ map { $row->{ $self->_real_column_name($state, $_) } } @{ $index->{columns} } ];
}

sub _key_has_null {
    my ($key) = @_;
    for my $value (@$key) {
        return 1 if !defined $value;
    }
    return 0;
}

sub _compare_keys {
    my ($left, $right) = @_;
    my $length = @$left > @$right ? @$left : @$right;
    for my $i (0 .. $length - 1) {
        my $comparison = CodingAdventures::SqlBackend::compare_sql_values($left->[$i], $right->[$i]);
        return $comparison if $comparison != 0;
    }
    return 0;
}

sub _serialize_key {
    my ($key) = @_;
    return join "\0", map {
        (CodingAdventures::SqlBackend::_is_blob($_) ? 'BLOB:' . $_->{bytes} : CodingAdventures::SqlBackend::sql_type_name($_) . ':' . (defined $_ ? $_ : ''))
    } @$key;
}

sub _validate_unique_index {
    my ($self, $state, $index, $candidate, $skip_rowid) = @_;
    if ($candidate) {
        my $candidate_key = $self->_key_for_index($state, $index, $candidate);
        return if _key_has_null($candidate_key);
        for my $record (@{ $state->{rows} }) {
            next if defined $skip_rowid && $record->{rowid} == $skip_rowid;
            if (_compare_keys($self->_key_for_index($state, $index, $record->{row}), $candidate_key) == 0) {
                CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_constraint_violation(
                    $state->{name}, join(',', @{ $index->{columns} }),
                    'UNIQUE constraint failed: ' . $state->{name} . '.' . join(',', @{ $index->{columns} })
                ));
            }
        }
        return;
    }
    my %seen;
    for my $record (@{ $state->{rows} }) {
        my $key = $self->_key_for_index($state, $index, $record->{row});
        next if _key_has_null($key);
        my $serialized = _serialize_key($key);
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_constraint_violation(
            $state->{name}, join(',', @{ $index->{columns} }),
            'UNIQUE constraint failed: ' . $state->{name} . '.' . join(',', @{ $index->{columns} })
        )) if $seen{$serialized};
        $seen{$serialized} = 1;
    }
}

sub _validate_row {
    my ($self, $state, $candidate, $skip_rowid) = @_;
    for my $column (@{ $state->{columns} }) {
        my $value = $candidate->{ $column->{name} };
        if ($column->effective_not_null && !defined $value) {
            CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_constraint_violation(
                $state->{name}, $column->{name}, 'NOT NULL constraint failed: ' . $state->{name} . '.' . $column->{name}
            ));
        }
        if ($column->effective_unique && defined $value) {
            for my $record (@{ $state->{rows} }) {
                next if defined $skip_rowid && $record->{rowid} == $skip_rowid;
                next unless CodingAdventures::SqlBackend::compare_sql_values($record->{row}{ $column->{name} }, $value) == 0;
                my $constraint = $column->{primary_key} ? 'PRIMARY KEY' : 'UNIQUE';
                CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_constraint_violation(
                    $state->{name}, $column->{name}, "$constraint constraint failed: " . $state->{name} . '.' . $column->{name}
                ));
            }
        }
    }
    for my $index (values %{ $self->{indexes_by_key} }) {
        next unless $index->{unique};
        next unless CodingAdventures::SqlBackend::_normalize_name($index->{table}) eq CodingAdventures::SqlBackend::_normalize_name($state->{name});
        $self->_validate_unique_index($state, $index, $candidate, $skip_rowid);
    }
}

sub insert {
    my ($self, $table, $row) = @_;
    my $state = $self->_table_state($table);
    my $candidate = $self->_materialize_row($state, $row);
    $self->_validate_row($state, $candidate, undef);
    push @{ $state->{rows} }, { rowid => $state->{next_rowid}++, row => $candidate };
    return 1;
}

sub _current_record_for {
    my ($self, $state, $cursor) = @_;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal('cursor does not belong to table'))
        unless Scalar::Util::blessed($cursor) && $cursor->isa('CodingAdventures::SqlBackend::TableCursor')
            && $cursor->{table_key} eq CodingAdventures::SqlBackend::_normalize_name($state->{name});
    my $record = $cursor->current_record;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal('cursor is not positioned on a row')) unless $record;
    return $record;
}

sub update {
    my ($self, $table, $cursor, $assignments) = @_;
    my $state = $self->_table_state($table);
    my $record = $self->_current_record_for($state, $cursor);
    my $candidate = CodingAdventures::SqlBackend::copy_row($record->{row});
    for my $name (keys %{ $assignments || {} }) {
        my $column = $self->_find_column($state, $name);
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_not_found($state->{name}, $name)) unless $column;
        my $value = $assignments->{$name};
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal('not a SqlValue')) unless CodingAdventures::SqlBackend::is_sql_value($value);
        $candidate->{ $column->{name} } = CodingAdventures::SqlBackend::copy_value($value);
    }
    $self->_validate_row($state, $candidate, $record->{rowid});
    $record->{row} = $candidate;
    return 1;
}

sub delete {
    my ($self, $table, $cursor) = @_;
    my $state = $self->_table_state($table);
    my $record = $self->_current_record_for($state, $cursor);
    for my $i (0 .. $#{ $state->{rows} }) {
        if ($state->{rows}[$i] == $record) {
            splice @{ $state->{rows} }, $i, 1;
            $cursor->adjust_after_delete;
            last;
        }
    }
    return 1;
}

sub create_table {
    my ($self, $table, $columns, %opts) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($table);
    if ($self->{tables_by_key}{$key}) {
        return 1 if $opts{if_not_exists};
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_table_already_exists($table));
    }
    my (%seen, @copied);
    for my $column (@{ $columns || [] }) {
        my $column_key = CodingAdventures::SqlBackend::_normalize_name($column->{name});
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_already_exists($table, $column->{name})) if $seen{$column_key};
        $seen{$column_key} = 1;
        push @copied, $column->clone;
    }
    $self->{tables_by_key}{$key} = { name => "$table", columns => \@copied, rows => [], next_rowid => 0 };
    $self->_bump_schema_version;
    return 1;
}

sub drop_table {
    my ($self, $table, %opts) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($table);
    if (!$self->{tables_by_key}{$key}) {
        return 1 if $opts{if_exists};
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_table_not_found($table));
    }
    delete $self->{tables_by_key}{$key};
    for my $index_key (keys %{ $self->{indexes_by_key} }) {
        delete $self->{indexes_by_key}{$index_key}
            if CodingAdventures::SqlBackend::_normalize_name($self->{indexes_by_key}{$index_key}{table}) eq $key;
    }
    delete $self->{triggers_by_table}{$key};
    for my $trigger_key (keys %{ $self->{triggers_by_key} }) {
        delete $self->{triggers_by_key}{$trigger_key}
            if CodingAdventures::SqlBackend::_normalize_name($self->{triggers_by_key}{$trigger_key}{table}) eq $key;
    }
    $self->_bump_schema_version;
    return 1;
}

sub add_column {
    my ($self, $table, $column) = @_;
    my $state = $self->_table_state($table);
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_already_exists($state->{name}, $column->{name}))
        if $self->_find_column($state, $column->{name});
    if ($column->effective_not_null && !$column->{has_default} && @{ $state->{rows} }) {
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_constraint_violation(
            $state->{name}, $column->{name}, 'NOT NULL constraint failed: ' . $state->{name} . '.' . $column->{name}
        ));
    }
    my $copied = $column->clone;
    push @{ $state->{columns} }, $copied;
    for my $record (@{ $state->{rows} }) {
        $record->{row}{ $copied->{name} } = CodingAdventures::SqlBackend::copy_value($copied->{default});
    }
    $self->_bump_schema_version;
    return 1;
}

sub create_index {
    my ($self, $index) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($index->{name});
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_index_already_exists($index->{name})) if $self->{indexes_by_key}{$key};
    my $state = $self->_table_state($index->{table});
    for my $column (@{ $index->{columns} }) {
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_column_not_found($state->{name}, $column))
            unless $self->_find_column($state, $column);
    }
    my $copied = $index->clone;
    $self->_validate_unique_index($state, $copied, undef, undef) if $copied->{unique};
    $self->{indexes_by_key}{$key} = $copied;
    $self->_bump_schema_version;
    return 1;
}

sub drop_index {
    my ($self, $name, %opts) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($name);
    if (!$self->{indexes_by_key}{$key}) {
        return 1 if $opts{if_exists};
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_index_not_found($name));
    }
    delete $self->{indexes_by_key}{$key};
    $self->_bump_schema_version;
    return 1;
}

sub list_indexes {
    my ($self, $table) = @_;
    my $table_key = defined $table ? CodingAdventures::SqlBackend::_normalize_name($table) : undef;
    return [
        sort { $a->{name} cmp $b->{name} }
        map { $_->clone }
        grep { !defined $table_key || CodingAdventures::SqlBackend::_normalize_name($_->{table}) eq $table_key }
        values %{ $self->{indexes_by_key} }
    ];
}

sub scan_index {
    my ($self, $index_name, $lo, $hi, %opts) = @_;
    my $index = $self->{indexes_by_key}{ CodingAdventures::SqlBackend::_normalize_name($index_name) };
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_index_not_found($index_name)) unless $index;
    my $state = $self->_table_state($index->{table});
    my @entries = map {
        { key => $self->_key_for_index($state, $index, $_->{row}), rowid => $_->{rowid} }
    } @{ $state->{rows} };
    @entries = sort {
        my $cmp = _compare_keys($a->{key}, $b->{key});
        $cmp || ($a->{rowid} <=> $b->{rowid});
    } @entries;
    my @rowids;
    for my $entry (@entries) {
        my $after_lo = 1;
        my $before_hi = 1;
        if ($lo) {
            my $cmp = _compare_keys($entry->{key}, $lo);
            $after_lo = $cmp > 0 || (($opts{lo_inclusive} // 1) && $cmp == 0);
        }
        if ($hi) {
            my $cmp = _compare_keys($entry->{key}, $hi);
            $before_hi = $cmp < 0 || (($opts{hi_inclusive} // 1) && $cmp == 0);
        }
        push @rowids, $entry->{rowid} if $after_lo && $before_hi;
    }
    return \@rowids;
}

sub scan_by_rowids {
    my ($self, $table, $rowids) = @_;
    my $state = $self->_table_state($table);
    my %by_rowid = map { $_->{rowid} => $_->{row} } @{ $state->{rows} };
    return CodingAdventures::SqlBackend::ListRowIterator->new([ map { $by_rowid{$_} } grep { exists $by_rowid{$_} } @{ $rowids || [] } ]);
}

sub _copy_table_state {
    my ($state) = @_;
    return {
        name => $state->{name},
        columns => [ map { $_->clone } @{ $state->{columns} } ],
        rows => [ map { { rowid => $_->{rowid}, row => CodingAdventures::SqlBackend::copy_row($_->{row}) } } @{ $state->{rows} } ],
        next_rowid => $state->{next_rowid},
    };
}

sub _snapshot_state {
    my ($self) = @_;
    return {
        tables_by_key => { map { $_ => _copy_table_state($self->{tables_by_key}{$_}) } keys %{ $self->{tables_by_key} } },
        indexes_by_key => { map { $_ => $self->{indexes_by_key}{$_}->clone } keys %{ $self->{indexes_by_key} } },
        triggers_by_key => { map { $_ => $self->{triggers_by_key}{$_}->clone } keys %{ $self->{triggers_by_key} } },
        triggers_by_table => { map { $_ => [ @{ $self->{triggers_by_table}{$_} } ] } keys %{ $self->{triggers_by_table} } },
        user_version => $self->{user_version},
        schema_version => $self->{schema_version},
    };
}

sub _restore_state {
    my ($self, $snapshot) = @_;
    $self->{tables_by_key} = { map { $_ => _copy_table_state($snapshot->{tables_by_key}{$_}) } keys %{ $snapshot->{tables_by_key} } };
    $self->{indexes_by_key} = { map { $_ => $snapshot->{indexes_by_key}{$_}->clone } keys %{ $snapshot->{indexes_by_key} } };
    $self->{triggers_by_key} = { map { $_ => $snapshot->{triggers_by_key}{$_}->clone } keys %{ $snapshot->{triggers_by_key} } };
    $self->{triggers_by_table} = { map { $_ => [ @{ $snapshot->{triggers_by_table}{$_} } ] } keys %{ $snapshot->{triggers_by_table} } };
    $self->{user_version} = $snapshot->{user_version};
    $self->{schema_version} = $snapshot->{schema_version};
}

sub begin_transaction {
    my ($self) = @_;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_unsupported('nested transactions')) if defined $self->{current_transaction};
    $self->{transaction_snapshot} = $self->_snapshot_state;
    $self->{current_transaction} = $self->{next_transaction}++;
    return $self->{current_transaction};
}

sub current_transaction {
    my ($self) = @_;
    return $self->{current_transaction};
}

sub _validate_transaction {
    my ($self, $handle) = @_;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal('invalid transaction handle'))
        unless defined $self->{current_transaction} && defined $handle && $self->{current_transaction} == $handle;
}

sub commit {
    my ($self, $handle) = @_;
    $self->_validate_transaction($handle);
    $self->{transaction_snapshot} = undef;
    $self->{current_transaction} = undef;
    $self->{savepoints} = [];
    return 1;
}

sub rollback {
    my ($self, $handle) = @_;
    $self->_validate_transaction($handle);
    $self->_restore_state($self->{transaction_snapshot});
    $self->{transaction_snapshot} = undef;
    $self->{current_transaction} = undef;
    $self->{savepoints} = [];
    return 1;
}

sub create_savepoint {
    my ($self, $name) = @_;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_unsupported('savepoints outside transaction'))
        unless defined $self->{current_transaction};
    push @{ $self->{savepoints} }, { name => "$name", snapshot => $self->_snapshot_state };
    return 1;
}

sub _savepoint_index {
    my ($self, $name) = @_;
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_unsupported('savepoints outside transaction'))
        unless defined $self->{current_transaction};
    for (my $i = $#{ $self->{savepoints} }; $i >= 0; $i--) {
        return $i if $self->{savepoints}[$i]{name} eq "$name";
    }
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_internal("savepoint not found: $name"));
}

sub release_savepoint {
    my ($self, $name) = @_;
    my $index = $self->_savepoint_index($name);
    splice @{ $self->{savepoints} }, $index;
    return 1;
}

sub rollback_to_savepoint {
    my ($self, $name) = @_;
    my $index = $self->_savepoint_index($name);
    $self->_restore_state($self->{savepoints}[$index]{snapshot});
    splice @{ $self->{savepoints} }, $index + 1;
    return 1;
}

sub create_trigger {
    my ($self, $trigger) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($trigger->{name});
    CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_trigger_already_exists($trigger->{name}))
        if $self->{triggers_by_key}{$key};
    my $state = $self->_table_state($trigger->{table});
    $self->{triggers_by_key}{$key} = $trigger->clone;
    my $table_key = CodingAdventures::SqlBackend::_normalize_name($state->{name});
    push @{ $self->{triggers_by_table}{$table_key} ||= [] }, $key;
    $self->_bump_schema_version;
    return 1;
}

sub drop_trigger {
    my ($self, $name, %opts) = @_;
    my $key = CodingAdventures::SqlBackend::_normalize_name($name);
    my $trigger = $self->{triggers_by_key}{$key};
    if (!$trigger) {
        return 1 if $opts{if_exists};
        CodingAdventures::SqlBackend::_raise(CodingAdventures::SqlBackend::_trigger_not_found($name));
    }
    delete $self->{triggers_by_key}{$key};
    my $table_key = CodingAdventures::SqlBackend::_normalize_name($trigger->{table});
    my $keys = $self->{triggers_by_table}{$table_key} ||= [];
    @$keys = grep { $_ ne $key } @$keys;
    $self->_bump_schema_version;
    return 1;
}

sub list_triggers {
    my ($self, $table) = @_;
    my $table_key = CodingAdventures::SqlBackend::_normalize_name($table);
    return [ map { $self->{triggers_by_key}{$_}->clone } grep { $self->{triggers_by_key}{$_} } @{ $self->{triggers_by_table}{$table_key} || [] } ];
}

1;
