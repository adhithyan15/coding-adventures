use strict;
use warnings;
use Config;
use File::Path qw(make_path);
use File::Spec;
use File::Temp qw(tempdir);
use FindBin;

my $package = File::Spec->rel2abs(File::Spec->catdir($FindBin::Bin, '..'));
chdir $package or die "cannot enter package directory $package: $!";
my $temporary = tempdir('coding-adventures-perl-microbenchmark-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $install_base = File::Spec->catdir($temporary, 'install');
my $report_path = File::Spec->catfile($temporary, 'native-report.json');
my @local_dependencies = map {
    File::Spec->rel2abs(File::Spec->catdir($package, $_, 'lib'))
} qw(../rank-statistics ../grammar-tools ../json-lexer ../json-parser);
my $path_separator = $Config{path_sep};
$ENV{PERL5LIB} = join $path_separator,
    File::Spec->catdir($install_base, 'lib', 'perl5'),
    File::Spec->catdir($install_base, 'lib', 'perl5', $Config{archname}),
    @local_dependencies,
    grep { defined && length } ($ENV{PERL5LIB});

sub run {
    my (@command) = @_;
    print '+ ', join(' ', @command), "\n";
    system @command;
    die "command failed ($?): @command" if $? != 0;
}

my @make = grep { length } split /\s+/, $Config{make};
die 'Perl Config has no make command' unless @make;
my $completed = eval {
    local $ENV{GIT_REVISION} = '';
    local $ENV{GITHUB_SHA} = '';
    run($^X, 'Makefile.PL', "INSTALL_BASE=$install_base");
    run(@make, 'test');
    run(@make, 'install');

    my $installed_script = File::Spec->catfile($install_base, 'bin', 'array-microbenchmark');
    open my $native, '-|', $^X, $installed_script, '--pairs', '40'
        or die "cannot launch installed native CLI: $!";
    open my $report, '>:raw', $report_path or die "cannot create report: $!";
    while (1) {
        my $chunk = '';
        my $read = read($native, $chunk, 64 * 1024);
        die "cannot read native CLI output: $!" unless defined $read;
        last if $read == 0;
        print {$report} $chunk or die "cannot write report: $!";
    }
    close $report or die "cannot close report: $!";
    close $native or die "installed CLI failed ($?)";
    run($^X, 'tests/installed-consumer/Consumer.pl', $report_path);
    1;
};
my $error = $@;
if (-f 'Makefile') {
    system @make, 'realclean';
    die "make realclean failed ($?)" if $? != 0 && $completed;
}
die $error unless $completed;
print "installed Perl consumer and native 40-pair replay passed\n";
