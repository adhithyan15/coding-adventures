package CodingAdventures::Microbenchmark;

# Paired microbenchmark collection and strict report replay.
# Each pair runs both variants adjacently at one common calibrated size. The
# clock is injectable, so every scheduling and failure branch is deterministic
# in tests; the native CLI uses Time::HiRes' monotonic wall clock.

use strict;
use warnings;
use Exporter qw(import);
use Config;
use B qw(svref_2object SVf_IOK SVf_NOK);
use Encode qw(decode encode FB_CROAK);
use JSON::PP ();
use Math::BigInt;
use Math::BigFloat;
use POSIX qw(isfinite floor);
use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
use Scalar::Util qw(looks_like_number);
use CodingAdventures::RankStatistics qw(median median_interval);
use CodingAdventures::JsonParser;

our $VERSION = '0.1.0';
our @EXPORT_OK = qw(defaults analyze collect report_from_json report_to_json indexed_batch iterated_batch array_cli monotonic_clock);

use constant MAX_PAIRS => 10_000;
use constant MAX_JSON_BYTES => 4 * 1024 * 1024;
use constant MAX_JSON_DEPTH => 256;
use constant MAX_JSON_NODES => 200_000;
use constant MAX_JSON_TOKENS => 200_000;
use constant I64_MAX => '9223372036854775807';
use constant I64_MIN => '-9223372036854775808';

sub _i64 {
    my ($value, $name, $min, $max) = @_;
    die "$name must be a signed 64-bit integer" if !defined($value) || ref($value);
    die "$name must be an integer value" unless svref_2object(\$value)->FLAGS & SVf_IOK();
    my $text = "$value";
    die "$name must be an integer token" unless $text =~ /\A(?:0|-?[1-9][0-9]*)\z/;
    my $big = Math::BigInt->new($text);
    die "$name is outside signed 64-bit range" if $big->bcmp(I64_MIN) < 0 || $big->bcmp(I64_MAX) > 0;
    die "$name is below its minimum" if defined($min) && $big->bcmp($min) < 0;
    die "$name is above its maximum" if defined($max) && $big->bcmp($max) > 0;
    return 0 + $text;
}

sub _factor {
    my ($value) = @_;
    my $number = _numeric_value($value, 'minimum_factor');
    die "minimum_factor must be a finite number greater than 1" if !isfinite($number) || $number <= 1;
    return $number;
}

sub _numeric_value {
    my ($value, $label) = @_;
    if (ref($value) eq 'Math::BigFloat' || ref($value) eq 'Math::BigInt') {
        return $value->numify;
    }
    die "$label must be a finite JSON number" if !defined($value) || ref($value) || !looks_like_number($value);
    return 0 + $value;
}

sub defaults { return { pairs => 40, confidence_percent => 95, minimum_factor => 1.05, seed => 0, target_ns => 100_000, max_batch => 65_536 }; }

sub _config {
    my ($value) = @_;
    die "config must be an object" unless ref($value) eq 'HASH';
    my %keys = map { $_ => 1 } qw(pairs confidence_percent minimum_factor seed target_ns max_batch);
    die "config fields do not match schema" if grep { !$keys{$_} } keys %$value;
    my $d = defaults();
    my $c = { pairs => _i64($value->{pairs} // $d->{pairs}, 'pairs', 1, MAX_PAIRS),
        confidence_percent => _i64($value->{confidence_percent} // $d->{confidence_percent}, 'confidence_percent', 1, 99),
        minimum_factor => _factor(defined $value->{minimum_factor} ? $value->{minimum_factor} : $d->{minimum_factor}),
        seed => _i64(defined $value->{seed} ? $value->{seed} : $d->{seed}, 'seed'),
        target_ns => _i64(defined $value->{target_ns} ? $value->{target_ns} : $d->{target_ns}, 'target_ns', 1),
        max_batch => _i64(defined $value->{max_batch} ? $value->{max_batch} : $d->{max_batch}, 'max_batch', 1) };
    return $c;
}

sub analyze {
    my ($pairs, $confidence, $factor) = @_;
    die "pairs must be a non-empty array reference" unless ref($pairs) eq 'ARRAY' && @$pairs;
    die "at most 10000 pairs are supported" if @$pairs > MAX_PAIRS;
    $confidence = _i64($confidence, 'confidence_percent', 1, 99);
    $factor = _factor($factor);
    my @ratios;
    my %valid_order = map { $_ => 1 } ('baseline-first', 'candidate-first');
    for my $i (0 .. $#$pairs) {
        my $p = $pairs->[$i];
        die "pair $i must be an object" unless ref($p) eq 'HASH';
        die "pair $i has invalid order" unless defined($p->{order}) && !ref($p->{order}) && $valid_order{$p->{order}};
        my $base = _i64($p->{baseline_ns}, "pair $i baseline_ns", 1);
        my $candidate = _i64($p->{candidate_ns}, "pair $i candidate_ns", 1);
        push @ratios, $candidate / $base;
    }
    my $ratio = median(\@ratios);
    my $interval = median_interval(\@ratios, $confidence);
    my $verdict = 'inconclusive';
    if ($interval) {
        my ($low, $high) = @$interval;
        $verdict = 'faster' if $high < 1 / $factor;
        $verdict = 'slower' if $low > $factor;
        $verdict = 'equivalent' if $low >= 1 / $factor && $high <= $factor;
    }
    return { ratio => $ratio, interval => $interval, verdict => $verdict };
}

sub _checked_product {
    my ($a, $b, $label) = @_;
    my $product = Math::BigInt->new("$a")->bmul("$b");
    die "$label overflows signed 64-bit integer" if $product->bcmp(I64_MIN) < 0 || $product->bcmp(I64_MAX) > 0;
    return 0 + $product->bstr;
}

sub _checked_sum {
    my ($a, $b, $label) = @_;
    die "$label overflows signed 64-bit integer" if ($b > 0 && $a > I64_MAX - $b) || ($b < 0 && $a < I64_MIN - $b);
    return $a + $b;
}

sub _now_ns {
    my $now = clock_gettime(CLOCK_MONOTONIC);
    die "monotonic clock unavailable" unless defined($now) && isfinite($now) && $now >= 0;
    return int(floor($now * 1_000_000_000));
}

sub monotonic_clock {
    my $origin = _now_ns();
    return sub {
        my $elapsed = _now_ns() - $origin;
        die "monotonic clock elapsed value is outside signed 64-bit range" if $elapsed < 0 || $elapsed > I64_MAX;
        return $elapsed;
    };
}

sub collect {
    my ($baseline, $candidate, $expected, $config, $clock) = @_;
    $config = _config(defined($config) ? $config : defaults());
    die "baseline and candidate callbacks are required" unless ref($baseline) eq 'CODE' && ref($candidate) eq 'CODE';
    die "clock callback is required" unless ref($clock) eq 'CODE';
    $expected = _i64($expected, 'expected_checksum');
    my $want = sub { _checked_product($expected, $_[0], 'checksum') };
    for my $fn ($baseline, $candidate) {
        my $got = $fn->(1);
        die "checksum mismatch" unless _i64($got, 'checksum') == $want->(1);
    }
    my $last;
    my $read = sub {
        my $tick = _i64($clock->(), 'clock tick', 0);
        die "clock regressed" if defined($last) && $tick < $last;
        $last = $tick;
        return $tick;
    };
    my $measure = sub {
        my ($fn, $size, $calibration) = @_;
        my $expected_batch = $want->($size);
        my $start = $read->();
        my $got = $fn->($size);
        my $end = $read->();
        die "checksum mismatch" unless _i64($got, 'checksum') == $expected_batch;
        die "measured duration must be positive" if $end == $start && !$calibration;
        return $end - $start;
    };
    my $size = 1;
    while (1) {
        my $left = $measure->($baseline, $size, 1);
        my $right = $measure->($candidate, $size, 1);
        last if $left >= $config->{target_ns} && $right >= $config->{target_ns};
        die "calibration limit exhausted" if $size >= $config->{max_batch};
        $size = $size > int($config->{max_batch} / 2) ? $config->{max_batch} : $size * 2;
    }
    my @schedule = (('baseline-first') x int(($config->{pairs} + 1) / 2), ('candidate-first') x int($config->{pairs} / 2));
    my $rng = $config->{seed} % 4_294_967_296;
    $rng += 4_294_967_296 if $rng < 0;
    my $random = sub { $rng = (1_664_525 * $rng + 1_013_904_223) % 4_294_967_296; return $rng; };
    for (my $i = $#schedule; $i > 0; $i--) { my $j = $random->() % ($i + 1); @schedule[$i, $j] = @schedule[$j, $i]; }
    my @pairs;
    for my $order (@schedule) {
        my ($bns, $cns);
        if ($order eq 'baseline-first') { $bns = $measure->($baseline, $size, 0); $cns = $measure->($candidate, $size, 0); }
        else { $cns = $measure->($candidate, $size, 0); $bns = $measure->($baseline, $size, 0); }
        push @pairs, { baseline_ns => $bns, candidate_ns => $cns, order => $order };
    }
    return { config => $config, batch_size => $size, pairs => \@pairs, analysis => analyze(\@pairs, $config->{confidence_percent}, $config->{minimum_factor}) };
}

sub _walk_ast {
    my ($node, $state) = @_;
    return unless ref($node) && $node->can('rule_name');
    $state->{nodes}++;
    die "report exceeds JSON node limit" if $state->{nodes} > MAX_JSON_NODES;
    my $children = $node->children;
    if ($node->rule_name eq 'object') {
        my %seen;
        for my $child (@$children) {
            next unless ref($child) && $child->can('rule_name') && $child->rule_name eq 'pair';
            $state->{nodes}++;
            die "report exceeds JSON node limit" if $state->{nodes} > MAX_JSON_NODES;
            my $key_leaf = $child->children->[0];
            my $raw = $key_leaf->token->{value};
            my $key = JSON::PP->new->decode($raw);
            die "duplicate JSON object member" if exists $seen{$key};
            $seen{$key} = 1;
            _walk_ast($child->children->[2], $state);
        }
        return;
    }
    _walk_ast($_, $state) for @$children;
}

sub _preflight_json {
    my ($text) = @_;
    my ($depth, $tokens, $index) = (0, 0, 0);
    my $length = length($text);
    my $add_token = sub {
        $tokens++;
        die "report exceeds JSON token limit" if $tokens > MAX_JSON_TOKENS;
    };
    while ($index < $length) {
        my $char = substr($text, $index++, 1);
        next if $char =~ /\s/;
        if ($char eq '"') {
            $add_token->();
            while ($index < $length) {
                my $inside = substr($text, $index++, 1);
                if ($inside eq '\\') { $index++ if $index < $length; next; }
                last if $inside eq '"';
            }
            next;
        }
        if ($char eq '{' || $char eq '[') {
            $add_token->();
            $depth++;
            die "report exceeds JSON depth limit" if $depth > MAX_JSON_DEPTH;
            next;
        }
        if ($char eq '}' || $char eq ']') {
            $add_token->();
            $depth--;
            next;
        }
        if ($char eq ',' || $char eq ':') { $add_token->(); next; }
        $add_token->();
        while ($index < $length) {
            my $next = substr($text, $index, 1);
            last if $next =~ /[\s{}\[\],:"]/;
            $index++;
        }
    }
}

sub _json_text {
    my ($input) = @_;
    die "report JSON must be a scalar" if !defined($input) || ref($input);
    my $raw = $input;
    my $text = utf8::is_utf8($raw) ? $raw : decode('UTF-8', $raw, FB_CROAK);
    my $text_copy = $text;
    die "report exceeds JSON byte limit" if length(encode('UTF-8', $text_copy, FB_CROAK)) > MAX_JSON_BYTES;
    _preflight_json($text);
    return $text;
}

sub _read_bounded_report {
    my ($fh) = @_;
    my @chunks;
    my $total = 0;
    while ($total <= MAX_JSON_BYTES) {
        my $remaining = MAX_JSON_BYTES + 1 - $total;
        my $chunk = '';
        my $read = sysread($fh, $chunk, $remaining);
        die "cannot read report: $!" unless defined $read;
        last if $read == 0;
        push @chunks, $chunk;
        $total += $read;
    }
    die "report exceeds JSON byte limit" if $total > MAX_JSON_BYTES;
    return join('', @chunks);
}

sub _fields {
    my ($object, $expected, $label) = @_;
    die "$label must be an object" unless ref($object) eq 'HASH';
    my %allowed = map { $_ => 1 } @$expected;
    die "$label has unknown fields" if grep { !$allowed{$_} } keys %$object;
    die "$label is missing fields" if grep { !exists $object->{$_} } @$expected;
}

sub _json_number {
    my ($value, $label) = @_;
    my $number = _numeric_value($value, $label);
    die "$label must be a finite JSON number" unless isfinite($number);
    return $number;
}

sub _metadata_text {
    my ($value, $label, $allow_empty) = @_;
    die "$label must be a string" if !defined($value) || ref($value);
    my $flags = svref_2object(\$value)->FLAGS;
    die "$label must be a string" if $flags & (SVf_IOK() | SVf_NOK());
    die "$label must not be empty" if !$allow_empty && !length($value);
    die "$label must not contain NUL" if index($value, "\0") >= 0;
    return $value;
}

sub report_from_json {
    my ($input) = @_;
    my $text = _json_text($input);
    my $ast = CodingAdventures::JsonParser->parse($text);
    _walk_ast($ast, { nodes => 0 });
    my $doc = JSON::PP->new->allow_bignum->decode($text);
    die "report must be an object" unless ref($doc) eq 'HASH';
    my @core = qw(config batch_size pairs analysis);
    _fields($doc, [@core, grep { exists $doc->{$_} } qw(workload invocation revision runtime os arch profile)], 'report');
    my @metadata = qw(workload invocation revision runtime os arch);
    my $present = grep { exists $doc->{$_} } @metadata;
    die "standard metadata fields must be complete or absent" if $present && $present != @metadata;
    die "profile requires complete standard metadata" if exists($doc->{profile}) && !$present;
    if ($present) {
        for my $field (@metadata) {
            if ($field eq 'invocation') {
                die "invocation must be a string array" unless ref($doc->{invocation}) eq 'ARRAY';
                _metadata_text($_, "invocation entry", 1) for @{$doc->{invocation}};
            } else {
                _metadata_text($doc->{$field}, $field, 0);
                if ($field eq 'workload') {
                    my $workload = $doc->{$field};
                    my $flags = svref_2object(\$workload)->FLAGS;
                    die "workload must be a string" if $flags & (SVf_IOK() | SVf_NOK());
                    my $has_nonblank = $workload =~ /[^ \x09-\x0D]/;
                    die "workload must be nonblank and contain no NUL" if !$has_nonblank || index($workload, "\0") >= 0;
                }
            }
        }
    }
    _metadata_text($doc->{profile}, "profile", 0) if exists($doc->{profile});
    _fields($doc->{config}, [qw(pairs confidence_percent minimum_factor seed target_ns max_batch)], 'config');
    my $config = _config($doc->{config});
    my $batch_size = _i64($doc->{batch_size}, 'batch_size', 1, $config->{max_batch});
    die "pairs must be an array" unless ref($doc->{pairs}) eq 'ARRAY' && @{$doc->{pairs}} == $config->{pairs};
    my @pairs;
    my $baseline_first = 0;
    for my $i (0 .. $#{$doc->{pairs}}) {
        my $p = $doc->{pairs}[$i];
        _fields($p, [qw(baseline_ns candidate_ns order)], "pair $i");
        my $b = _i64($p->{baseline_ns}, "pair $i baseline_ns", 1);
        my $c = _i64($p->{candidate_ns}, "pair $i candidate_ns", 1);
        die "pair $i has invalid order" unless !ref($p->{order}) && ($p->{order} eq 'baseline-first' || $p->{order} eq 'candidate-first');
        $baseline_first++ if $p->{order} eq 'baseline-first';
        push @pairs, { baseline_ns => $b, candidate_ns => $c, order => $p->{order} };
    }
    die "pair order schedule is unbalanced" if $baseline_first != int(($config->{pairs} + 1) / 2);
    _fields($doc->{analysis}, [qw(ratio interval verdict)], 'analysis');
    my $analysis = analyze(\@pairs, $config->{confidence_percent}, $config->{minimum_factor});
    my $ratio = _json_number($doc->{analysis}{ratio}, 'analysis.ratio');
    die "analysis ratio differs from raw pairs" unless $ratio == $analysis->{ratio};
    die "analysis verdict differs from raw pairs" unless defined($doc->{analysis}{verdict}) && !ref($doc->{analysis}{verdict}) && $doc->{analysis}{verdict} eq $analysis->{verdict};
    if (defined $analysis->{interval}) {
        die "analysis interval must contain two finite numbers" unless ref($doc->{analysis}{interval}) eq 'ARRAY' && @{$doc->{analysis}{interval}} == 2;
        my @given = map { _json_number($_, 'analysis interval bound') } @{$doc->{analysis}{interval}};
        die "analysis interval differs from raw pairs" unless $given[0] == $analysis->{interval}[0] && $given[1] == $analysis->{interval}[1];
    } else {
        die "analysis interval must be null" if defined $doc->{analysis}{interval};
    }
    return { config => $config, batch_size => $batch_size, pairs => \@pairs, analysis => $analysis,
        (map { exists($doc->{$_}) ? ($_ => $doc->{$_}) : () } @metadata, 'profile') };
}

sub report_to_json {
    my ($report) = @_;
    # JSON::PP emits ordinary Perl NVs with 15 significant digits by default.
    # Promote contract float fields to BigFloat decimal spellings generated at
    # 17 digits so decoding back to binary64 preserves the analysis exactly.
    my %copy = %$report;
    $copy{config} = { %{$report->{config}} };
    $copy{config}{minimum_factor} = Math::BigFloat->new(sprintf('%.17g', $report->{config}{minimum_factor}));
    $copy{analysis} = { %{$report->{analysis}} };
    $copy{analysis}{ratio} = Math::BigFloat->new(sprintf('%.17g', $report->{analysis}{ratio}));
    if (ref($report->{analysis}{interval}) eq 'ARRAY') {
        $copy{analysis}{interval} = [ map { Math::BigFloat->new(sprintf('%.17g', $_)) } @{$report->{analysis}{interval}} ];
    }
    my $json = JSON::PP->new->utf8->canonical->pretty->allow_bignum;
    return $json->encode(\%copy);
}

sub _array_values {
    my @values = map { (17 * $_ + 3) % 251 } 0 .. 4095;
    return \@values;
}

sub _checked_array_input {
    my ($values, $operations) = @_;
    die "values must be an array reference" unless ref($values) eq 'ARRAY';
    die "operations must be a non-negative integer" unless defined($operations) && !ref($operations) && "$operations" =~ /\A(?:0|[1-9][0-9]*)\z/;
    die "operations must be a non-negative integer" if $operations < 0;
    for my $index (0 .. $#$values) {
        my $value = $values->[$index];
        my $label = "array element $index";
        die "$label must be a signed 64-bit integer" if !defined($value) || ref($value);
        die "$label must be an integer value" unless svref_2object(\$value)->FLAGS & SVf_IOK();
        my $text = "$value";
        die "$label must be an integer token" unless $text =~ /\A(?:0|-?[1-9][0-9]*)\z/;
        my $negative = substr($text, 0, 1) eq '-';
        (my $digits = $text) =~ s/\A-//;
        my $limit = $negative ? '9223372036854775808' : '9223372036854775807';
        die "$label is outside signed 64-bit range" if length($digits) > 19 || (length($digits) == 19 && $digits gt $limit);
    }
    return 0 + $operations;
}

sub indexed_batch {
    my ($values, $operations) = @_;
    $operations = _checked_array_input($values, $operations);
    my $total = 0;
    for (1 .. $operations) { for my $i (0 .. $#$values) { $total = _checked_sum($total, $values->[$i], 'array checksum'); } }
    return $total;
}

sub iterated_batch {
    my ($values, $operations) = @_;
    $operations = _checked_array_input($values, $operations);
    my $total = 0;
    for (1 .. $operations) { for my $value (@$values) { $total = _checked_sum($total, $value, 'array checksum'); } }
    return $total;
}

sub _revision {
    my ($explicit) = @_;
    my $trimmed = sub { my $text = defined($_[0]) ? $_[0] : ''; $text =~ s/\A\s+|\s+\z//g; return $text; };
    die "--revision must not be blank" if defined($explicit) && !length($trimmed->($explicit));
    return $trimmed->($explicit) if defined $explicit;
    return $trimmed->($ENV{GIT_REVISION}) if defined($ENV{GIT_REVISION}) && length($trimmed->($ENV{GIT_REVISION}));
    return $trimmed->($ENV{GITHUB_SHA}) if defined($ENV{GITHUB_SHA}) && length($trimmed->($ENV{GITHUB_SHA}));
    return 'unknown';
}

sub array_cli {
    my ($argv, $stdout, $stderr, $clock) = @_;
    $argv ||= \@ARGV; $stdout ||= *STDOUT; $stderr ||= *STDERR; $clock ||= monotonic_clock();
    my $config = defaults(); my ($revision, $explicit_revision, $replay);
    my @args = @$argv;
    while (@args) {
        my $option = shift @args;
        if ($option eq '--help') { print {$stderr} "Usage: array-microbenchmark [--pairs N] [--confidence N] [--factor F] [--seed I64] [--target-ns N] [--max-batch N] [--revision REV] | --replay FILE\n"; return 0; }
        die "missing value for $option" unless @args;
        my $value = shift @args;
        if ($option eq '--pairs') { $config->{pairs} = _cli_integer($value, $option, 1, MAX_PAIRS); }
        elsif ($option eq '--confidence') { $config->{confidence_percent} = _cli_integer($value, $option, 1, 99); }
        elsif ($option eq '--factor') { die "invalid factor" unless $value =~ /\A(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\z/; $config->{minimum_factor} = _factor(0 + $value); }
        elsif ($option eq '--seed') { $config->{seed} = _cli_integer($value, $option); }
        elsif ($option eq '--target-ns') { $config->{target_ns} = _cli_integer($value, $option, 1); }
        elsif ($option eq '--max-batch') { $config->{max_batch} = _cli_integer($value, $option, 1); }
        elsif ($option eq '--revision') { $revision = $value; $explicit_revision = 1; }
        elsif ($option eq '--replay') { $replay = $value; }
        else { die "unknown option $option"; }
    }
    $revision = _revision($explicit_revision ? $revision : undef);
    if (defined $replay) {
        open my $fh, '<:raw', $replay or die "cannot read report: $!";
        my $raw = _read_bounded_report($fh); close $fh or die "cannot close report: $!";
        my $report = report_from_json($raw);
        _write_checked($stdout, "replay valid: " . scalar(@{$report->{pairs}}) . " pairs, $report->{analysis}{verdict}\n");
        return 0;
    }
    my $values = _array_values();
    my $expected = 0; $expected += $_ for @$values;
    my $report = collect(sub { indexed_batch($values, $_[0]) }, sub { iterated_batch($values, $_[0]) }, $expected, $config, $clock);
    my @invocation = ('array-microbenchmark', @$argv);
    @{$report}{qw(workload invocation revision runtime os arch)} = ('indexed-array-sum-4096', \@invocation,
        $revision, "$^V", $^O, $Config{archname});
    my $text = report_to_json($report);
    report_from_json($text);
    _write_checked($stdout, $text);
    return 0;
}

sub _cli_integer {
    my ($text, $label, $min, $max) = @_;
    die "$label must be a decimal integer" unless defined($text) && $text =~ /\A(?:0|-?[1-9][0-9]*)\z/;
    my $v = 0 + _i64(0 + $text, $label, $min, $max);
    return $v;
}

sub _write_checked {
    my ($fh, $text) = @_;
    my $is_stdout = defined(fileno($fh)) && defined(fileno(STDOUT)) && fileno($fh) == fileno(STDOUT);
    binmode($fh, ':raw') if $is_stdout;
    print {$fh} $text or die "stdout write failed: $!";
    if ($is_stdout) { require IO::Handle; IO::Handle::flush($fh) or die "stdout flush failed: $!"; }
}

1;
