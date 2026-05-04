package CodingAdventures::MiniSqlite;

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use Storable qw(dclone);
use CodingAdventures::SqlExecutionEngine;

our $VERSION = '0.01';
our $apilevel = '2.0';
our $threadsafety = 1;
our $paramstyle = 'qmark';

use constant ROW_ID_COLUMN => '__mini_sqlite_rowid';

sub apilevel { return $apilevel }
sub threadsafety { return $threadsafety }
sub paramstyle { return $paramstyle }

sub _err {
    my ($kind, $message) = @_;
    return { kind => $kind, message => $message };
}

sub _return {
    my ($value, $err) = @_;
    return wantarray ? ($value, $err) : $value;
}

sub _fail {
    my ($kind, $message) = @_;
    return _return(undef, _err($kind, $message));
}

sub _trim {
    my ($s) = @_;
    $s = '' unless defined $s;
    $s =~ s/^\s+//;
    $s =~ s/\s+$//;
    return $s;
}

sub _normalize_name {
    my ($name) = @_;
    return lc $name;
}

sub _strip_trailing_semicolon {
    my ($sql) = @_;
    $sql = _trim($sql);
    $sql =~ s/;\s*\z//;
    return $sql;
}

sub _is_boundary_char {
    my ($ch) = @_;
    return 1 if !defined($ch) || $ch eq '';
    return $ch !~ /[A-Za-z0-9_]/;
}

sub _first_keyword {
    my ($sql) = @_;
    $sql = _trim($sql);
    return uc($1) if $sql =~ /\A([A-Za-z_]+)/;
    return '';
}

sub _quote_sql_string {
    my ($value) = @_;
    $value = "$value";
    $value =~ s/'/''/g;
    return "'$value'";
}

sub _to_sql_literal {
    my ($value) = @_;
    return (defined $value ? undef : 'NULL', undef) unless defined $value;
    return ($value, undef) if !ref($value) && looks_like_number($value);
    return (_quote_sql_string($value), undef) unless ref($value);
    return (undef, _err('ProgrammingError', 'unsupported parameter type: ' . ref($value)));
}

sub _read_quoted {
    my ($sql, $i, $quote) = @_;
    $i++;
    while ($i < length($sql)) {
        my $ch = substr($sql, $i, 1);
        if ($ch eq $quote) {
            if (substr($sql, $i + 1, 1) eq $quote) {
                $i += 2;
            } else {
                return $i + 1;
            }
        } else {
            $i++;
        }
    }
    return length($sql);
}

sub _bind_parameters {
    my ($sql, $params) = @_;
    $params ||= [];
    return (undef, _err('ProgrammingError', 'parameters must be an array reference'))
        if ref($params) ne 'ARRAY';

    my @out;
    my $index = 0;
    my $i = 0;
    while ($i < length($sql)) {
        my $ch = substr($sql, $i, 1);
        if ($ch eq q{'} || $ch eq q{"}) {
            my $next_i = _read_quoted($sql, $i, $ch);
            push @out, substr($sql, $i, $next_i - $i);
            $i = $next_i;
        } elsif ($ch eq '-' && substr($sql, $i, 2) eq '--') {
            my $next_i = $i + 2;
            $next_i++ while $next_i < length($sql) && substr($sql, $next_i, 1) ne "\n";
            push @out, substr($sql, $i, $next_i - $i);
            $i = $next_i;
        } elsif ($ch eq '/' && substr($sql, $i, 2) eq '/*') {
            my $next_i = $i + 2;
            $next_i++ while $next_i + 1 < length($sql) && substr($sql, $next_i, 2) ne '*/';
            $next_i = $next_i + 2 <= length($sql) ? $next_i + 2 : length($sql);
            push @out, substr($sql, $i, $next_i - $i);
            $i = $next_i;
        } elsif ($ch eq '?') {
            return (undef, _err('ProgrammingError', 'not enough parameters for SQL statement'))
                if $index >= @$params;
            my ($literal, $literal_err) = _to_sql_literal($params->[$index]);
            return (undef, $literal_err) unless defined $literal;
            push @out, $literal;
            $index++;
            $i++;
        } else {
            push @out, $ch;
            $i++;
        }
    }

    return (undef, _err('ProgrammingError', 'too many parameters for SQL statement'))
        if $index < @$params;
    return (join('', @out), undef);
}

sub _split_top_level {
    my ($text, $delimiter) = @_;
    my (@parts, @current);
    my ($depth, $quote, $i) = (0, undef, 0);
    while ($i < length($text)) {
        my $ch = substr($text, $i, 1);
        if (defined $quote) {
            push @current, $ch;
            if ($ch eq $quote) {
                if (substr($text, $i + 1, 1) eq $quote) {
                    $i++;
                    push @current, substr($text, $i, 1);
                } else {
                    $quote = undef;
                }
            }
        } elsif ($ch eq q{'} || $ch eq q{"}) {
            $quote = $ch;
            push @current, $ch;
        } elsif ($ch eq '(') {
            $depth++;
            push @current, $ch;
        } elsif ($ch eq ')') {
            $depth-- if $depth > 0;
            push @current, $ch;
        } elsif ($depth == 0 && $ch eq $delimiter) {
            my $part = _trim(join('', @current));
            push @parts, $part if $part ne '';
            @current = ();
        } else {
            push @current, $ch;
        }
        $i++;
    }
    my $part = _trim(join('', @current));
    push @parts, $part if $part ne '';
    return \@parts;
}

sub _split_top_level_keyword {
    my ($text, $keyword) = @_;
    my $upper = uc($text);
    my $key_len = length($keyword);
    my ($depth, $quote, $i) = (0, undef, 0);
    while ($i < length($text)) {
        my $ch = substr($text, $i, 1);
        if (defined $quote) {
            if ($ch eq $quote) {
                if (substr($text, $i + 1, 1) eq $quote) { $i++ } else { $quote = undef }
            }
        } elsif ($ch eq q{'} || $ch eq q{"}) {
            $quote = $ch;
        } elsif ($ch eq '(') {
            $depth++;
        } elsif ($ch eq ')') {
            $depth-- if $depth > 0;
        } elsif (
            $depth == 0
            && substr($upper, $i, $key_len) eq $keyword
            && _is_boundary_char($i == 0 ? '' : substr($text, $i - 1, 1))
            && _is_boundary_char(substr($text, $i + $key_len, 1))
        ) {
            return (_trim(substr($text, 0, $i)), _trim(substr($text, $i + $key_len)));
        }
        $i++;
    }
    return (_trim($text), '');
}

sub _find_matching_paren {
    my ($text, $open_index) = @_;
    my ($depth, $quote, $i) = (0, undef, $open_index);
    while ($i < length($text)) {
        my $ch = substr($text, $i, 1);
        if (defined $quote) {
            if ($ch eq $quote) {
                if (substr($text, $i + 1, 1) eq $quote) { $i++ } else { $quote = undef }
            }
        } elsif ($ch eq q{'} || $ch eq q{"}) {
            $quote = $ch;
        } elsif ($ch eq '(') {
            $depth++;
        } elsif ($ch eq ')') {
            $depth--;
            return $i if $depth == 0;
        }
        $i++;
    }
    return undef;
}

sub _parse_literal {
    my ($text) = @_;
    my $value = _trim($text);
    my $upper = uc($value);
    return undef if $upper eq 'NULL';
    return 1 if $upper eq 'TRUE';
    return 0 if $upper eq 'FALSE';
    if ($value =~ /\A'(.*)'\z/s) {
        my $s = $1;
        $s =~ s/''/'/g;
        return $s;
    }
    return 0 + $value if $value =~ /\A[-+]?(?:\d+(?:\.\d*)?|\.\d+)\z/;
    die "expected literal value, got: $text\n";
}

sub _identifier_at_start {
    my ($text) = @_;
    $text = _trim($text);
    return $1 if $text =~ /\A([A-Za-z_][A-Za-z0-9_]*)/;
    return undef;
}

sub _parse_create {
    my ($sql) = @_;
    my $s = _strip_trailing_semicolon($sql);
    my ($table_name, $defs, $if_not_exists);
    if ($s =~ /\A\s*CREATE\s+TABLE\s+IF\s+NOT\s+EXISTS\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*\z/is) {
        ($table_name, $defs, $if_not_exists) = ($1, $2, 1);
    } elsif ($s =~ /\A\s*CREATE\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*\z/is) {
        ($table_name, $defs, $if_not_exists) = ($1, $2, 0);
    } else {
        die "invalid CREATE TABLE statement\n";
    }

    my @columns;
    for my $part (@{ _split_top_level($defs, ',') }) {
        my $name = _identifier_at_start($part);
        push @columns, $name if defined $name;
    }
    die "CREATE TABLE requires at least one column\n" unless @columns;
    return { table => $table_name, columns => \@columns, if_not_exists => $if_not_exists };
}

sub _parse_drop {
    my ($sql) = @_;
    my $s = _strip_trailing_semicolon($sql);
    return { table => $1, if_exists => 1 }
        if $s =~ /\A\s*DROP\s+TABLE\s+IF\s+EXISTS\s+([A-Za-z_][A-Za-z0-9_]*)\s*\z/is;
    return { table => $1, if_exists => 0 }
        if $s =~ /\A\s*DROP\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*\z/is;
    die "invalid DROP TABLE statement\n";
}

sub _parse_value_rows {
    my ($sql) = @_;
    my $rest = _trim($sql);
    my @rows;
    while ($rest ne '') {
        die "INSERT VALUES rows must be parenthesized\n" unless substr($rest, 0, 1) eq '(';
        my $close = _find_matching_paren($rest, 0);
        die "unterminated INSERT VALUES row\n" unless defined $close;
        my $inside = substr($rest, 1, $close - 1);
        my @row = map { _parse_literal($_) } @{ _split_top_level($inside, ',') };
        die "INSERT row requires at least one value\n" unless @row;
        push @rows, \@row;
        $rest = _trim(substr($rest, $close + 1));
        if (substr($rest, 0, 1) eq ',') {
            $rest = _trim(substr($rest, 1));
        } elsif ($rest ne '') {
            die "invalid text after INSERT row\n";
        }
    }
    die "INSERT requires at least one row\n" unless @rows;
    return \@rows;
}

sub _parse_insert {
    my ($sql) = @_;
    my $s = _strip_trailing_semicolon($sql);
    my ($table_name, $columns_sql, $rows_sql);
    my @columns;
    if ($s =~ /\A\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s+VALUES\s+(.+)\s*\z/is) {
        ($table_name, $columns_sql, $rows_sql) = ($1, $2, $3);
        @columns = map { _trim($_) } @{ _split_top_level($columns_sql, ',') };
    } elsif ($s =~ /\A\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)\s+VALUES\s+(.+)\s*\z/is) {
        ($table_name, $rows_sql) = ($1, $2);
    } else {
        die "invalid INSERT statement\n";
    }
    return { table => $table_name, columns => \@columns, rows => _parse_value_rows($rows_sql) };
}

sub _parse_update {
    my ($sql) = @_;
    my $s = _strip_trailing_semicolon($sql);
    die "invalid UPDATE statement\n"
        unless $s =~ /\A\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.+)\s*\z/is;
    my ($table_name, $rest) = ($1, $2);
    my ($assign_sql, $where_sql) = _split_top_level_keyword($rest, 'WHERE');
    my @assignments;
    for my $assignment (@{ _split_top_level($assign_sql, ',') }) {
        my $parts = _split_top_level($assignment, '=');
        die "invalid assignment: $assignment\n" unless @$parts == 2;
        my $col = _trim($parts->[0]);
        die "invalid identifier: $col\n" unless $col =~ /\A[A-Za-z_][A-Za-z0-9_]*\z/;
        push @assignments, { column => $col, value => _parse_literal($parts->[1]) };
    }
    die "UPDATE requires at least one assignment\n" unless @assignments;
    return { table => $table_name, assignments => \@assignments, where => $where_sql };
}

sub _parse_delete {
    my ($sql) = @_;
    my $s = _strip_trailing_semicolon($sql);
    die "invalid DELETE statement\n"
        unless $s =~ /\A\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(.*)\z/is;
    my ($table_name, $where_sql) = ($1, _trim($2 // ''));
    if ($where_sql ne '') {
        die "invalid DELETE statement\n" unless $where_sql =~ /\AWHERE\s+(.+)\z/is;
        $where_sql = _trim($1);
    }
    return { table => $table_name, where => $where_sql };
}

sub connect {
    my ($class, $database, @rest) = @_;
    my $options = ref($rest[0]) eq 'HASH' ? $rest[0] : { @rest };
    return _fail('NotSupportedError', 'Perl mini-sqlite supports only :memory: in Level 0')
        unless defined $database && $database eq ':memory:';
    return _return(CodingAdventures::MiniSqlite::Connection->new($options), undef);
}

package CodingAdventures::MiniSqlite::Database;

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless { tables => {} }, $class;
}

sub snapshot {
    my ($self) = @_;
    return { tables => Storable::dclone($self->{tables}) };
}

sub restore {
    my ($self, $snapshot) = @_;
    $self->{tables} = Storable::dclone($snapshot->{tables});
}

sub schema {
    my ($self, $table_name) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($table_name)};
    die "no such table: $table_name\n" unless $table_data;
    return [ @{ $table_data->{columns} } ];
}

sub scan {
    my ($self, $table_name) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($table_name)};
    die "no such table: $table_name\n" unless $table_data;
    return Storable::dclone($table_data->{rows});
}

sub _canonical_column {
    my ($table_data, $column) = @_;
    my $wanted = CodingAdventures::MiniSqlite::_normalize_name($column);
    for my $candidate (@{ $table_data->{columns} }) {
        return $candidate if CodingAdventures::MiniSqlite::_normalize_name($candidate) eq $wanted;
    }
    die "no such column: $column\n";
}

sub create {
    my ($self, $stmt) = @_;
    my $key = CodingAdventures::MiniSqlite::_normalize_name($stmt->{table});
    if ($self->{tables}{$key}) {
        return { columns => [], rows => [], rows_affected => 0 } if $stmt->{if_not_exists};
        die "table already exists: $stmt->{table}\n";
    }
    my %seen;
    for my $column (@{ $stmt->{columns} }) {
        my $normalized = CodingAdventures::MiniSqlite::_normalize_name($column);
        die "duplicate column: $column\n" if $seen{$normalized};
        $seen{$normalized} = 1;
    }
    $self->{tables}{$key} = { columns => [ @{ $stmt->{columns} } ], rows => [] };
    return { columns => [], rows => [], rows_affected => 0 };
}

sub drop {
    my ($self, $stmt) = @_;
    my $key = CodingAdventures::MiniSqlite::_normalize_name($stmt->{table});
    if (!$self->{tables}{$key}) {
        return { columns => [], rows => [], rows_affected => 0 } if $stmt->{if_exists};
        die "no such table: $stmt->{table}\n";
    }
    delete $self->{tables}{$key};
    return { columns => [], rows => [], rows_affected => 0 };
}

sub insert {
    my ($self, $stmt) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($stmt->{table})};
    die "no such table: $stmt->{table}\n" unless $table_data;
    my @columns = @{ $stmt->{columns} }
        ? map { _canonical_column($table_data, $_) } @{ $stmt->{columns} }
        : @{ $table_data->{columns} };

    for my $values (@{ $stmt->{rows} }) {
        die 'INSERT expected ' . scalar(@columns) . ' values, got ' . scalar(@$values) . "\n"
            unless @$values == @columns;
        my %row;
        for my $i (0 .. $#columns) {
            $row{$columns[$i]} = $values->[$i];
        }
        push @{ $table_data->{rows} }, \%row;
    }
    return { columns => [], rows => [], rows_affected => scalar @{ $stmt->{rows} } };
}

sub matching_row_ids {
    my ($self, $table_name, $where_sql) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($table_name)};
    die "no such table: $table_name\n" unless $table_data;
    if (CodingAdventures::MiniSqlite::_trim($where_sql // '') eq '') {
        return [ 1 .. scalar @{ $table_data->{rows} } ];
    }

    my $source = CodingAdventures::MiniSqlite::RowIdSource->new($self, $table_name);
    my ($ok, $result) = CodingAdventures::SqlExecutionEngine->execute(
        'SELECT ' . CodingAdventures::MiniSqlite::ROW_ID_COLUMN . " FROM $table_name WHERE $where_sql",
        $source,
    );
    die "$result\n" unless $ok;
    return [ map { $_->[0] } @{ $result->{rows} } ];
}

sub update {
    my ($self, $stmt) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($stmt->{table})};
    die "no such table: $stmt->{table}\n" unless $table_data;
    my @assignments = map {
        { column => _canonical_column($table_data, $_->{column}), value => $_->{value} }
    } @{ $stmt->{assignments} };
    my $ids = $self->matching_row_ids($stmt->{table}, $stmt->{where});
    my %idset = map { $_ => 1 } @$ids;
    my $id = 1;
    for my $row (@{ $table_data->{rows} }) {
        if ($idset{$id}) {
            for my $assignment (@assignments) {
                $row->{ $assignment->{column} } = $assignment->{value};
            }
        }
        $id++;
    }
    return { columns => [], rows => [], rows_affected => scalar @$ids };
}

sub delete {
    my ($self, $stmt) = @_;
    my $table_data = $self->{tables}{CodingAdventures::MiniSqlite::_normalize_name($stmt->{table})};
    die "no such table: $stmt->{table}\n" unless $table_data;
    my $ids = $self->matching_row_ids($stmt->{table}, $stmt->{where});
    my %idset = map { $_ => 1 } @$ids;
    my @rows;
    my $id = 1;
    for my $row (@{ $table_data->{rows} }) {
        push @rows, $row unless $idset{$id};
        $id++;
    }
    $table_data->{rows} = \@rows;
    return { columns => [], rows => [], rows_affected => scalar @$ids };
}

sub select_sql {
    my ($self, $sql) = @_;
    my ($ok, $result) = CodingAdventures::SqlExecutionEngine->execute($sql, $self);
    die "$result\n" unless $ok;
    $result->{rows_affected} = -1;
    return $result;
}

package CodingAdventures::MiniSqlite::RowIdSource;

use strict;
use warnings;

sub new {
    my ($class, $db, $table_name) = @_;
    return bless { db => $db, table_name => $table_name }, $class;
}

sub schema {
    my ($self, $table_name) = @_;
    die "no such table: $table_name\n"
        unless CodingAdventures::MiniSqlite::_normalize_name($table_name)
            eq CodingAdventures::MiniSqlite::_normalize_name($self->{table_name});
    my $cols = $self->{db}->schema($table_name);
    push @$cols, CodingAdventures::MiniSqlite::ROW_ID_COLUMN;
    return $cols;
}

sub scan {
    my ($self, $table_name) = @_;
    die "no such table: $table_name\n"
        unless CodingAdventures::MiniSqlite::_normalize_name($table_name)
            eq CodingAdventures::MiniSqlite::_normalize_name($self->{table_name});
    my $rows = $self->{db}->scan($table_name);
    my $id = 1;
    for my $row (@$rows) {
        $row->{CodingAdventures::MiniSqlite::ROW_ID_COLUMN()} = $id++;
    }
    return $rows;
}

package CodingAdventures::MiniSqlite::Cursor;

use strict;
use warnings;

sub new {
    my ($class, $conn) = @_;
    return bless {
        conn        => $conn,
        description => [],
        rowcount    => -1,
        lastrowid   => undef,
        arraysize   => 1,
        rows        => [],
        offset      => 0,
        closed      => 0,
    }, $class;
}

sub execute {
    my ($self, $sql, $params) = @_;
    return CodingAdventures::MiniSqlite::_fail('ProgrammingError', 'cursor is closed')
        if $self->{closed};
    my ($result, $err) = $self->{conn}->_execute_bound($sql, $params // []);
    return CodingAdventures::MiniSqlite::_return(undef, $err) unless $result;
    $self->{rows} = $result->{rows} // [];
    $self->{offset} = 0;
    $self->{rowcount} = $result->{rows_affected} // -1;
    $self->{description} = [ map { { name => $_ } } @{ $result->{columns} // [] } ];
    return CodingAdventures::MiniSqlite::_return($self, undef);
}

sub executemany {
    my ($self, $sql, $params_seq) = @_;
    $params_seq ||= [];
    my $total = 0;
    for my $params (@$params_seq) {
        my ($cursor, $err) = $self->execute($sql, $params);
        return CodingAdventures::MiniSqlite::_return(undef, $err) unless $cursor;
        $total += $self->{rowcount} if $self->{rowcount} > 0;
    }
    $self->{rowcount} = $total if @$params_seq;
    return CodingAdventures::MiniSqlite::_return($self, undef);
}

sub fetchone {
    my ($self) = @_;
    return undef if $self->{closed} || $self->{offset} >= @{ $self->{rows} };
    return $self->{rows}[ $self->{offset}++ ];
}

sub fetchmany {
    my ($self, $size) = @_;
    return [] if $self->{closed};
    $size //= $self->{arraysize};
    my @rows;
    for (1 .. $size) {
        my $row = $self->fetchone;
        last unless defined $row;
        push @rows, $row;
    }
    return \@rows;
}

sub fetchall {
    my ($self) = @_;
    return [] if $self->{closed};
    my @rows;
    while (1) {
        my $row = $self->fetchone;
        last unless defined $row;
        push @rows, $row;
    }
    return \@rows;
}

sub close {
    my ($self) = @_;
    $self->{closed} = 1;
    $self->{rows} = [];
    $self->{description} = [];
    return 1;
}

package CodingAdventures::MiniSqlite::Connection;

use strict;
use warnings;

sub new {
    my ($class, $options) = @_;
    return bless {
        db         => CodingAdventures::MiniSqlite::Database->new,
        autocommit => $options && $options->{autocommit} ? 1 : 0,
        snapshot   => undef,
        closed     => 0,
    }, $class;
}

sub _ensure_snapshot {
    my ($self) = @_;
    $self->{snapshot} = $self->{db}->snapshot
        if !$self->{autocommit} && !defined $self->{snapshot};
}

sub cursor {
    my ($self) = @_;
    return CodingAdventures::MiniSqlite::_fail('ProgrammingError', 'connection is closed')
        if $self->{closed};
    return CodingAdventures::MiniSqlite::_return(CodingAdventures::MiniSqlite::Cursor->new($self), undef);
}

sub execute {
    my ($self, $sql, $params) = @_;
    my $cursor = CodingAdventures::MiniSqlite::Cursor->new($self);
    return $cursor->execute($sql, $params // []);
}

sub executemany {
    my ($self, $sql, $params_seq) = @_;
    my $cursor = CodingAdventures::MiniSqlite::Cursor->new($self);
    return $cursor->executemany($sql, $params_seq // []);
}

sub commit {
    my ($self) = @_;
    return CodingAdventures::MiniSqlite::_fail('ProgrammingError', 'connection is closed')
        if $self->{closed};
    $self->{snapshot} = undef;
    return CodingAdventures::MiniSqlite::_return(1, undef);
}

sub rollback {
    my ($self) = @_;
    return CodingAdventures::MiniSqlite::_fail('ProgrammingError', 'connection is closed')
        if $self->{closed};
    if (defined $self->{snapshot}) {
        $self->{db}->restore($self->{snapshot});
        $self->{snapshot} = undef;
    }
    return CodingAdventures::MiniSqlite::_return(1, undef);
}

sub close {
    my ($self) = @_;
    return 1 if $self->{closed};
    $self->{db}->restore($self->{snapshot}) if defined $self->{snapshot};
    $self->{snapshot} = undef;
    $self->{closed} = 1;
    return 1;
}

sub _execute_bound {
    my ($self, $sql, $params) = @_;
    return (undef, CodingAdventures::MiniSqlite::_err('ProgrammingError', 'connection is closed'))
        if $self->{closed};
    my ($bound, $bind_err) = CodingAdventures::MiniSqlite::_bind_parameters($sql, $params // []);
    return (undef, $bind_err) unless defined $bound;
    my $keyword = CodingAdventures::MiniSqlite::_first_keyword($bound);
    my $result = eval {
        if ($keyword eq 'BEGIN') {
            $self->_ensure_snapshot;
            return { columns => [], rows => [], rows_affected => 0 };
        }
        if ($keyword eq 'COMMIT') {
            $self->{snapshot} = undef;
            return { columns => [], rows => [], rows_affected => 0 };
        }
        if ($keyword eq 'ROLLBACK') {
            if (defined $self->{snapshot}) {
                $self->{db}->restore($self->{snapshot});
                $self->{snapshot} = undef;
            }
            return { columns => [], rows => [], rows_affected => 0 };
        }
        if ($keyword eq 'SELECT') {
            return $self->{db}->select_sql($bound);
        }
        if ($keyword eq 'CREATE') {
            $self->_ensure_snapshot;
            return $self->{db}->create(CodingAdventures::MiniSqlite::_parse_create($bound));
        }
        if ($keyword eq 'DROP') {
            $self->_ensure_snapshot;
            return $self->{db}->drop(CodingAdventures::MiniSqlite::_parse_drop($bound));
        }
        if ($keyword eq 'INSERT') {
            $self->_ensure_snapshot;
            return $self->{db}->insert(CodingAdventures::MiniSqlite::_parse_insert($bound));
        }
        if ($keyword eq 'UPDATE') {
            $self->_ensure_snapshot;
            return $self->{db}->update(CodingAdventures::MiniSqlite::_parse_update($bound));
        }
        if ($keyword eq 'DELETE') {
            $self->_ensure_snapshot;
            return $self->{db}->delete(CodingAdventures::MiniSqlite::_parse_delete($bound));
        }
        die "unsupported SQL statement\n";
    };
    if ($@) {
        my $message = $@;
        $message =~ s/\s+at\s+.+\s+line\s+\d+.*\z//s;
        chomp $message;
        return (undef, CodingAdventures::MiniSqlite::_err('OperationalError', $message));
    }
    return ($result, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::MiniSqlite - Level 0 in-memory mini-sqlite facade

=head1 SYNOPSIS

  use CodingAdventures::MiniSqlite;

  my $conn = CodingAdventures::MiniSqlite->connect(':memory:');
  $conn->execute('CREATE TABLE users (id INTEGER, name TEXT)');
  $conn->execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);

  my $cursor = $conn->execute('SELECT name FROM users');
  my $rows = $cursor->fetchall;

=head1 DESCRIPTION

Provides a DB-API-inspired in-memory mini-sqlite facade for Perl. Level 0
supports simple table DDL/DML and delegates SELECT execution to
CodingAdventures::SqlExecutionEngine.

=cut
