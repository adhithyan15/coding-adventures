use ExtUtils::MakeMaker;
WriteMakefile(
    NAME         => 'CodingAdventures::JsonValue',
    VERSION_FROM => 'lib/CodingAdventures/JsonValue.pm',
    PREREQ_PM    => {
        'CodingAdventures::JsonParser' => '0.01',
    },
    TEST_REQUIRES => { 'Test2::V0' => 0 },
    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/example/coding-adventures',
            },
        },
    },
);
