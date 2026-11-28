use strict;
use warnings;
use Config;
use File::Spec;
use File::Temp qw(tempdir);
use FindBin;

my $package = File::Spec->rel2abs(File::Spec->catdir($FindBin::Bin, '..'));
chdir $package or die "cannot enter package directory $package: $!";
my $temporary = tempdir('coding-adventures-perl-rank-XXXXXX', TMPDIR => 1, CLEANUP => 1);
my $install_base = File::Spec->catdir($temporary, 'install');
my @make = grep { length } split /\s+/, $Config{make};
die 'Perl Config has no make command' unless @make;

sub run {
    my (@command) = @_;
    print '+ ', join(' ', @command), "\n";
    system @command;
    die "command failed ($?): @command" if $? != 0;
}

my $completed = eval {
    run($^X, 'Makefile.PL', "INSTALL_BASE=$install_base");
    run(@make, 'install');
    my $path_separator = $Config{path_sep};
    $ENV{PERL5LIB} = join $path_separator,
        File::Spec->catdir($install_base, 'lib', 'perl5'),
        File::Spec->catdir($install_base, 'lib', 'perl5', $Config{archname}),
        grep { defined && length } ($ENV{PERL5LIB});
    run($^X, 'tests/installed-consumer/Consumer.pl');
    1;
};
my $error = $@;
if (-f 'Makefile') {
    system @make, 'realclean';
    die "make realclean failed ($?)" if $? != 0 && $completed;
}
die $error unless $completed;
print "installed Perl rank-statistics consumer passed\n";
