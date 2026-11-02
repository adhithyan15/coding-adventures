package CodingAdventures::BuildTool::Resolver;

# Resolver.pm -- Dependency Resolution from Package Metadata
# ===========================================================
#
# This module reads package metadata files (cpanfile, pyproject.toml, Gemfile,
# go.mod, package.json, Cargo.toml, mix.exs, .rockspec) and extracts internal
# dependencies. It builds a directed graph where edges represent "A must be
# built before B" (i.e., B depends on A).
#
# Dependency naming conventions
# -----------------------------
#
# Each language ecosystem uses a different naming convention for packages in
# this monorepo:
#
#   Perl:       coding-adventures-<kebab>    in cpanfile
#   Python:     coding-adventures-<kebab>    in pyproject.toml [project.dependencies]
#   Ruby:       coding_adventures_<snake>    in Gemfile / .gemspec
#   Go:         github.com/adhithyan15/...   in go.mod
#   TypeScript: @coding-adventures/<kebab>   in package.json dependencies
#   Rust:       <kebab> path = "../.."       in Cargo.toml link tables
#   Elixir:     :coding_adventures_<snake>   in mix.exs
#   Lua:        coding-adventures-<kebab>    in .rockspec
#
# External dependencies (those not matching the monorepo naming pattern) are
# silently skipped — we only care about internal build ordering.
#
# Why inline DirectedGraph?
# -------------------------
#
# The build tool specification calls for using the coding-adventures-directed-graph
# Perl package as an external dependency. That package is defined in the
# perl-starter-packages spec, which is implemented after this tool. To avoid
# a chicken-and-egg situation, we include a complete DirectedGraph implementation
# here. Once coding-adventures-directed-graph is published to the monorepo, this
# inline copy can be replaced with:
#
#   use CodingAdventures::DirectedGraph;
#
# The public API is identical, so the swap is a one-line change.
#
# Perl advantages demonstrated here:
#   - Native regex with /g for global matching — "find all" in one line.
#   - while ($text =~ /PATTERN/g) — the "scan loop" idiom.
#   - Hashes as first-class data structures for known-name lookups.
#   - qw() for concise string lists.

use strict;
use warnings;
use Encode qw(decode FB_CROAK);
use File::Spec ();
use File::Basename ();
use File::Find ();
use Scalar::Util qw(blessed);

our $VERSION = '0.01';

# ===========================================================================
# MetadataEncodingError -- Stable failure for malformed package metadata
# ===========================================================================

package CodingAdventures::BuildTool::MetadataEncodingError;

use overload '""' => 'as_string', fallback => 1;

sub new {
    my ($class, %args) = @_;
    return bless {
        code     => 'METADATA_INVALID_UTF8',
        package  => $args{package},
        manifest => $args{manifest},
        encoding => 'UTF-8',
    }, $class;
}

sub as_string {
    my ($self) = @_;
    return join ' ',
        $self->{code} . ':',
        "package=$self->{package}",
        "manifest=$self->{manifest}",
        "encoding=$self->{encoding}";
}

# ===========================================================================
# DependencySelfEdgeError -- A package whose manifest resolves itself
# ===========================================================================
#
# The conformance contract rejects this as DEPENDENCY_SELF_EDGE: a self-edge
# is neither dropped nor drawn (a drawn self-loop would surface later as a
# cycle, far from its cause). A Cargo manifest can do it with nothing more
# than a path entry named after its own directory or [package] name. The
# manifest is repository-relative, so the string never discloses the
# checkout root.

package CodingAdventures::BuildTool::DependencySelfEdgeError;

use overload '""' => 'as_string', fallback => 1;

sub new {
    my ($class, %args) = @_;
    return bless {
        code       => 'DEPENDENCY_SELF_EDGE',
        package    => $args{package},
        manifest   => $args{manifest},
        dependency => $args{dependency},
    }, $class;
}

sub as_string {
    my ($self) = @_;
    return join ' ',
        $self->{code} . ':',
        "package=$self->{package}",
        "manifest=$self->{manifest}",
        "dependency=$self->{dependency}";
}

# ===========================================================================
# DirectedGraph -- Inline graph implementation
# ===========================================================================
#
# A directed graph with Kahn's algorithm for topological level ordering.
# Edges point FROM dependency TO dependent: "A -> B" means "A must come
# before B" or equivalently "B depends on A".
#
# Operations:
#   add_node($n)         -- Ensure a node exists.
#   add_edge($a, $b)     -- Add edge A -> B.
#   has_node($n)         -- True if node exists.
#   nodes()              -- All node identifiers.
#   successors($n)       -- Nodes that $n points TO.
#   predecessors($n)     -- Nodes that point TO $n.
#   independent_groups() -- Kahn's topological levels (list of listrefs).
#   transitive_closure($n) -- All reachable nodes from $n.
#   transitive_dependents($n) -- All nodes that transitively depend on $n.
#   affected_nodes(\@roots) -- All nodes reachable from any root.

package CodingAdventures::BuildTool::Graph;

sub new {
    my ($class) = @_;
    return bless {
        # $forward{$node} = { $succ => 1, ... }  — nodes $node points TO.
        # $reverse{$node} = { $pred => 1, ... }  — nodes that point TO $node.
        forward => {},
        reverse => {},
    }, $class;
}

sub add_node {
    my ($self, $node) = @_;
    $self->{forward}{$node} //= {};
    $self->{reverse}{$node} //= {};
}

sub add_edge {
    my ($self, $from, $to) = @_;
    $self->add_node($from);
    $self->add_node($to);
    $self->{forward}{$from}{$to} = 1;
    $self->{reverse}{$to}{$from} = 1;
}

sub has_node {
    my ($self, $node) = @_;
    return exists $self->{forward}{$node};
}

sub nodes {
    my ($self) = @_;
    return keys %{ $self->{forward} };
}

sub successors {
    my ($self, $node) = @_;
    return () unless exists $self->{forward}{$node};
    return keys %{ $self->{forward}{$node} };
}

sub predecessors {
    my ($self, $node) = @_;
    return () unless exists $self->{reverse}{$node};
    return keys %{ $self->{reverse}{$node} };
}

# independent_groups -- Return nodes in topological levels (Kahn's algorithm).
#
# Each level contains nodes whose dependencies are all in earlier levels.
# Packages in the same level can be built in parallel.
#
# Kahn's algorithm:
#   1. Compute in-degree for all nodes (number of predecessors).
#   2. Start with all nodes with in-degree 0 (no dependencies).
#   3. Yield that set as one level.
#   4. For each node in the level, decrement in-degree of its successors.
#   5. Any successor that reaches in-degree 0 joins the next level.
#   6. Repeat until no nodes remain.
#
# If a cycle exists, some nodes will never reach in-degree 0. We skip them
# with a warning (cycles in a build graph are a configuration error).
#
# Returns: list of array references, each containing node names for that level.
sub independent_groups {
    my ($self) = @_;

    # Step 1: Compute initial in-degrees.
    my %in_degree;
    for my $node (keys %{ $self->{forward} }) {
        $in_degree{$node} //= 0;
        for my $succ (keys %{ $self->{forward}{$node} }) {
            $in_degree{$succ} = ($in_degree{$succ} // 0) + 1;
        }
    }

    # Step 2: Initial frontier — all nodes with no predecessors.
    my @current_level = sort grep { ($in_degree{$_} // 0) == 0 } keys %in_degree;

    my @groups;
    while (@current_level) {
        push @groups, [@current_level];

        my @next_level;
        for my $node (@current_level) {
            for my $succ (sort keys %{ $self->{forward}{$node} }) {
                $in_degree{$succ}--;
                push @next_level, $succ if $in_degree{$succ} == 0;
            }
        }
        @current_level = sort @next_level;
    }

    return @groups;
}

# transitive_closure -- All nodes reachable FROM $start (following forward edges).
#
# Uses breadth-first search. Returns a list of node names (not including $start).
sub transitive_closure {
    my ($self, $start) = @_;
    my %visited;
    my @queue = ($start);
    while (@queue) {
        my $node = shift @queue;
        next if $visited{$node}++;
        push @queue, keys %{ $self->{forward}{$node} // {} };
    }
    delete $visited{$start};
    return keys %visited;
}

# transitive_dependents -- All nodes that transitively depend on $start.
#
# Follows reverse edges (predecessors). Returns list of node names.
sub transitive_dependents {
    my ($self, $start) = @_;
    my %visited;
    my @queue = ($start);
    while (@queue) {
        my $node = shift @queue;
        next if $visited{$node}++;
        push @queue, keys %{ $self->{reverse}{$node} // {} };
    }
    delete $visited{$start};
    return keys %visited;
}

# affected_nodes -- All nodes reachable from any of the root nodes.
#
# This is a multi-source BFS following forward edges (successors). Used by
# GitDiff to find all packages that transitively depend on changed packages.
#
# @param \@roots -- List of root node names.
# @return List of all affected node names (including the roots themselves).
sub affected_nodes {
    my ($self, $roots_ref) = @_;
    my %visited;
    my @queue = @{$roots_ref};
    while (@queue) {
        my $node = shift @queue;
        next if $visited{$node}++;
        push @queue, keys %{ $self->{forward}{$node} // {} };
    }
    return keys %visited;
}

# ===========================================================================
# Back to the Resolver package
# ===========================================================================

package CodingAdventures::BuildTool::Resolver;

# new -- Constructor.
#
# @return blessed hashref.
sub new {
    my ($class, %args) = @_;
    return bless {}, $class;
}

sub _dependency_scope {
    my ($language) = @_;
    return 'dotnet' if $language eq 'csharp' || $language eq 'fsharp' || $language eq 'dotnet';
    return 'rust' if $language eq 'wasm';
    return $language;
}

sub _in_dependency_scope {
    my ($package_language, $scope) = @_;
    return $package_language eq 'csharp' || $package_language eq 'fsharp' || $package_language eq 'dotnet'
        if $scope eq 'dotnet';
    return $package_language eq 'rust' || $package_language eq 'wasm'
        if $scope eq 'rust';
    return $package_language eq $scope;
}

sub _normalize_path {
    my ($path) = @_;
    return lc File::Spec->canonpath($path);
}

sub _read_cargo_package_name {
    my ($pkg) = @_;
    my $cargo = File::Spec->catfile($pkg->{path}, 'Cargo.toml');
    my $text = _slurp($cargo) // return undef;

    for my $line (split /\n/, $text) {
        if ($line =~ /^\s*name\s*=\s*"([^"]+)"/) {
            return lc $1;
        }
    }

    return undef;
}

sub _set_known {
    my ($known_ref, $key, $pkg) = @_;
    if (!exists $known_ref->{$key}) {
        $known_ref->{$key} = $pkg->{name};
        return;
    }

    my $normalized = $pkg->{path};
    $normalized =~ s{\\}{/}g;
    if ($normalized !~ m{/programs/}i) {
        $known_ref->{$key} = $pkg->{name};
    }
}

# resolve -- Build a dependency graph from a list of packages.
#
# For each package, reads its metadata file and extracts internal dependencies.
# Returns a Graph object where edges go FROM dependency TO dependent.
#
# @param \@packages -- arrayref of package hashrefs (from Discovery).
# @return CodingAdventures::BuildTool::Graph instance.
sub resolve {
    my ($self, $packages_ref) = @_;

    my $graph = CodingAdventures::BuildTool::Graph->new();
    my %known_names_by_scope;

    # Add all packages as nodes first — even isolated packages with no deps.
    for my $pkg (@{$packages_ref}) {
        $graph->add_node($pkg->{name});
        my $language = $pkg->{language};
        if (!exists $known_names_by_scope{$language}) {
            $known_names_by_scope{$language} = { $self->build_known_names($packages_ref, $language) };
        }
    }

    # Parse dependencies for each package and add edges.
    for my $pkg (@{$packages_ref}) {
        my @deps = $self->resolve_dependencies($pkg, $known_names_by_scope{$pkg->{language}});
        # Step 5 of the Rust contract: a Cargo candidate resolving to the
        # crate itself (by directory alias or [package] name) is rejected,
        # neither dropped nor drawn.
        if (($pkg->{language} // '') =~ /\A(?:rust|wasm)\z/
            && grep { $_ eq $pkg->{name} } @deps) {
            die CodingAdventures::BuildTool::DependencySelfEdgeError->new(
                package    => $pkg->{name},
                manifest   => _repository_relative_manifest(
                    File::Spec->catfile($pkg->{path}, 'Cargo.toml')),
                dependency => $pkg->{name},
            );
        }
        for my $dep (@deps) {
            # Edge: dep -> pkg (dep must be built before pkg).
            $graph->add_edge($dep, $pkg->{name});
        }
    }

    return $graph;
}

# build_known_names -- Build a mapping from ecosystem package names to graph node names.
#
# Each language uses a different naming convention for its packages. We precompute
# a hash from the "ecosystem name" to the "graph node name" so that dependency
# parsing can do O(1) lookups.
#
# Examples:
#   Python:     "coding-adventures-logic-gates" => "python/logic-gates"
#   Ruby:       "coding_adventures_logic_gates"  => "ruby/logic-gates"
#   TypeScript: "@coding-adventures/logic-gates" => "typescript/logic-gates"
#   Perl:       "coding-adventures-logic-gates"  => "perl/logic-gates"
#
# @param \@packages -- arrayref of package hashrefs.
# @return hash mapping ecosystem name -> graph node name.
sub build_known_names {
    my ($self, $packages_ref, $language) = @_;
    my %known;
    $language //= '';
    my $scope = $language eq '' ? '' : _dependency_scope($language);

    for my $pkg (@{$packages_ref}) {
        next if $scope ne '' && !_in_dependency_scope($pkg->{language}, $scope);
        my $dir_name  = File::Basename::basename($pkg->{path});
        my $lang      = $pkg->{language};

        if ($lang eq 'python') {
            # Python: coding-adventures-<kebab> (hyphens, lowercase).
            my $cpan_name = "coding-adventures-$dir_name";
            $cpan_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $cpan_name, $pkg);

        } elsif ($lang eq 'ruby') {
            # Ruby: coding_adventures_<snake> (underscores, lowercase).
            my $gem_name = "coding_adventures_$dir_name";
            $gem_name =~ s/-/_/g;
            $gem_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $gem_name, $pkg);

        } elsif ($lang eq 'go') {
            # Go: full module path github.com/adhithyan15/.../go/<name>.
            # We register both the bare name and the full path prefix.
            $known{"github.com/adhithyan15/coding-adventures/code/packages/go/$dir_name"} = $pkg->{name};
            $known{"github.com/adhithyan15/coding-adventures/code/programs/go/$dir_name"} = $pkg->{name};

        } elsif ($lang eq 'typescript') {
            # TypeScript: @coding-adventures/<kebab> (npm scoped package).
            my $npm_name = "\@coding-adventures/$dir_name";
            $npm_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $npm_name, $pkg);
            _set_known(\%known, lc($dir_name), $pkg);

        } elsif ($lang eq 'rust') {
            # Rust: bare crate name (hyphens, lowercase).
            my $crate_name = $dir_name;
            $crate_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $crate_name, $pkg);
            my $cargo_name = _read_cargo_package_name($pkg);
            _set_known(\%known, $cargo_name, $pkg) if defined $cargo_name;

        } elsif ($lang eq 'wasm') {
            # WASM wrappers should resolve through their explicit Cargo package
            # names (for example "graph-wasm"), not bare Rust crate names.
            my $cargo_name = _read_cargo_package_name($pkg);
            _set_known(\%known, $cargo_name, $pkg) if defined $cargo_name;

        } elsif ($lang eq 'elixir') {
            # Elixir: :coding_adventures_<snake> atom (underscores).
            my $mix_name = "coding_adventures_$dir_name";
            $mix_name =~ s/-/_/g;
            $mix_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $mix_name, $pkg);
            _set_known(\%known, ":$mix_name", $pkg);

        } elsif ($lang eq 'lua') {
            # Lua: coding-adventures-<kebab> (same as Python).
            my $rock_name = "coding-adventures-$dir_name";
            $rock_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $rock_name, $pkg);

        } elsif ($lang eq 'perl') {
            # Perl: coding-adventures-<kebab> (same convention as Python/Lua).
            my $dist_name = "coding-adventures-$dir_name";
            $dist_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $dist_name, $pkg);

        } elsif ($lang eq 'swift') {
            _set_known(\%known, lc($dir_name), $pkg);

        } elsif ($lang eq 'haskell') {
            # Haskell: coding-adventures-<kebab>
            my $cabal_name = "coding-adventures-$dir_name";
            $cabal_name =~ tr/A-Z/a-z/;
            _set_known(\%known, $cabal_name, $pkg);

        } elsif ($lang eq 'csharp' || $lang eq 'fsharp' || $lang eq 'dotnet') {
            # .NET packages refer to sibling package directories via
            # <ProjectReference Include="../graph/Graph.csproj" />.
            _set_known(\%known, lc($dir_name), $pkg);
            my @project_files;
            File::Find::find(
                {
                    wanted => sub {
                        return if -d $File::Find::name;
                        return unless $_ =~ /\.(?:csproj|fsproj)\z/;
                        push @project_files, $File::Find::name;
                    },
                    no_chdir => 1,
                },
                $pkg->{path},
            );
            for my $project_file (@project_files) {
                $known{ _normalize_path($project_file) } = $pkg->{name};
            }
        }
    }

    return %known;
}

# resolve_dependencies -- Parse the metadata file for one package.
#
# Dispatches to the language-specific parser. Returns a list of graph node
# names that this package depends on.
#
# @param $pkg        -- Package hashref.
# @param \%known     -- Mapping from ecosystem name -> graph node name.
# @return list of graph node names (strings).
sub resolve_dependencies {
    my ($self, $pkg, $known_ref) = @_;
    my $lang = $pkg->{language} // 'unknown';

    if ($lang eq 'python') {
        return $self->_parse_python_deps($pkg, $known_ref);
    } elsif ($lang eq 'ruby') {
        return $self->_parse_ruby_deps($pkg, $known_ref);
    } elsif ($lang eq 'go') {
        return $self->_parse_go_deps($pkg, $known_ref);
    } elsif ($lang eq 'typescript') {
        return $self->_parse_typescript_deps($pkg, $known_ref);
    } elsif ($lang eq 'rust' || $lang eq 'wasm') {
        return $self->_parse_rust_deps($pkg, $known_ref);
    } elsif ($lang eq 'elixir') {
        return $self->_parse_elixir_deps($pkg, $known_ref);
    } elsif ($lang eq 'lua') {
        return $self->_parse_lua_deps($pkg, $known_ref);
    } elsif ($lang eq 'perl') {
        return $self->_parse_perl_deps($pkg, $known_ref);
    } elsif ($lang eq 'swift') {
        return $self->_parse_swift_deps($pkg, $known_ref);
    } elsif ($lang eq 'haskell') {
        return $self->_parse_haskell_deps($pkg, $known_ref);
    } elsif ($lang eq 'csharp' || $lang eq 'fsharp' || $lang eq 'dotnet') {
        return $self->_parse_dotnet_deps($pkg, $known_ref);
    }
    return ();
}

# ---------------------------------------------------------------------------
# Language-specific parsers
# ---------------------------------------------------------------------------

# _parse_perl_deps -- Extract dependencies from a Perl cpanfile.
#
# The cpanfile format uses `requires 'DIST-NAME'` declarations. We look for
# lines matching:
#
#   requires 'coding-adventures-<name>'
#   requires "coding-adventures-<name>"
#
# Perl's /g modifier with while() is the "scan loop" idiom. The regex
# engine advances through the string, yielding each match in turn. This
# is more concise than Go's FindAllStringSubmatch or Python's findall().
#
# We skip comment lines (# ...) and only match our own monorepo prefix.
sub _parse_perl_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $cpanfile = File::Spec->catfile($pkg->{path}, 'cpanfile');
    my $text = _slurp($cpanfile) // return ();

    my @deps;
    # The scan loop: iterate over all occurrences of the pattern.
    while ($text =~ /requires\s+['"]coding-adventures-([^'"]+)['"]/g) {
        my $dep_name = "coding-adventures-$1";
        $dep_name =~ tr/A-Z/a-z/;
        push @deps, $known_ref->{$dep_name} if exists $known_ref->{$dep_name};
    }
    return @deps;
}

# _parse_python_deps -- Extract dependencies from pyproject.toml.
#
# We look for the [project] dependencies array and match strings that start
# with "coding-adventures-". Python allows version specifiers like
# "coding-adventures-logic-gates>=0.1", so we match up to the first
# non-name character.
sub _parse_python_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $pyproject = File::Spec->catfile($pkg->{path}, 'pyproject.toml');
    my $text = _slurp($pyproject) // return ();

    my @deps;
    while ($text =~ /"(coding-adventures-[^">=,\s]+)/g) {
        my $dep = lc $1;
        push @deps, $known_ref->{$dep} if exists $known_ref->{$dep};
    }
    return @deps;
}

# _parse_ruby_deps -- Extract dependencies from a Gemfile.
#
# Ruby gems in this monorepo use the coding_adventures_ prefix with
# underscores. Gemfile lines look like: gem "coding_adventures_foo"
sub _parse_ruby_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $gemfile = File::Spec->catfile($pkg->{path}, 'Gemfile');
    my $text = _slurp($gemfile) // return ();

    my @deps;
    while ($text =~ /gem\s+["'](coding_adventures_[^"']+)["']/g) {
        my $dep = lc $1;
        push @deps, $known_ref->{$dep} if exists $known_ref->{$dep};
    }
    return @deps;
}

# _parse_go_deps -- Extract dependencies from go.mod.
#
# Go module paths in this repo follow the pattern:
#   github.com/adhithyan15/coding-adventures/code/packages/go/<name>
# We match the full module path in `require` blocks.
sub _parse_go_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $gomod = File::Spec->catfile($pkg->{path}, 'go.mod');
    my $text = _slurp($gomod) // return ();

    my @deps;
    while ($text =~ /^\s*(github\.com\/adhithyan15\/coding-adventures\/[^\s]+)/mg) {
        my $dep = $1;
        push @deps, $known_ref->{$dep} if exists $known_ref->{$dep};
    }
    return @deps;
}

# _parse_typescript_deps -- Extract dependencies from package.json.
#
# TypeScript packages in this repo use @coding-adventures/ npm scope.
# We look for the "dependencies" and "devDependencies" sections only,
# so that the package's own "name" field is not matched as a dependency.
sub _parse_typescript_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $pkgjson = File::Spec->catfile($pkg->{path}, 'package.json');
    my $text = _slurp($pkgjson) // return ();

    # Extract own package name so we can skip it.
    my ($own_name) = ($text =~ /^\s*"name"\s*:\s*"([^"]+)"/m);
    $own_name = lc($own_name // '');

    my @deps;
    while ($text =~ /"(\@coding-adventures\/[^"]+)"/g) {
        my $dep = lc $1;
        next if $dep eq $own_name;   # skip self-reference from "name" field
        push @deps, $known_ref->{$dep} if exists $known_ref->{$dep};
    }
    return @deps;
}

# _parse_rust_deps -- Extract internal dependencies from Cargo.toml.
#
# Which Cargo tables are build-graph edges
# ----------------------------------------
#
# A graph edge means "rebuild and re-test the dependent when this changes",
# so it must cover everything Cargo compiles INTO the crate:
#
#   table                                 edge?  why
#   ------------------------------------  -----  -----------------------------
#   [dependencies]                        yes    linked into the library
#   [build-dependencies]                  yes    compiled into build.rs
#   [build_dependencies]                  yes    legacy spelling of the above
#   [target.<cfg>.<one of the above>]     yes    union over every cfg: CI
#                                                builds on several hosts
#   [dev-dependencies] (every form)       no     tests only; Cargo allows a
#                                                dev-dependency cycle
#   [patch], [replace], [workspace], ...  no     not a dependency of the crate
#
# Both TOML spellings of a path dependency are read:
#
#   [dependencies]
#   logic-gates = { path = "../logic-gates" }          <- inline table
#   gates = { package = "logic-gates", path = "..." }  <- renamed
#
#   [dependencies.logic-gates]                         <- sub-table
#   path = "../logic-gates"
#
# The line procedure is the Rust paragraph of
# code/specs/build-tool-conformance.md; every build-tool front implements it
# and the shared fixture resolution-rust-field-aware pins it. Each candidate
# name is looked up among the known Cargo names (directory name and declared
# [package] name); duplicates -- the same crate in [dependencies] and
# [build-dependencies], say -- collapse to one dependency.
sub _parse_rust_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $cargo = File::Spec->catfile($pkg->{path}, 'Cargo.toml');
    my $text = _slurp($cargo) // return ();

    my (@deps, %seen);
    for my $crate (cargo_link_dependency_names($text)) {
        next unless exists $known_ref->{$crate};
        my $dep = $known_ref->{$crate};
        push @deps, $dep unless $seen{$dep}++;
    }
    return @deps;
}

# The table names whose path entries link into, or build, the crate.
# Compared exactly with `eq`: TOML keys are case-sensitive, so
# `[Dependencies]` is some other table.
my %CARGO_LINK_TABLE = map { $_ => 1 } qw(dependencies build-dependencies build_dependencies);

# cargo_link_dependency_names($text) -- every path dependency's crate name.
#
# Returns lower-case names in manifest order, duplicates kept (the caller
# collapses them). _slurp reads the manifest as raw bytes; every character
# the procedure compares is ASCII, so scanning bytes is exact.
#
# It is one pass over the lines with a three-state machine:
#
#   'ignored'  not a link table; its lines are skipped
#   'inline'   [D] or [target.C.D]: one entry per line
#   'sub'      [D.N] or [target.C.D.N]: the path/package keys of entry N
#
# Every header line -- even a malformed one, or an array of tables -- ends
# the current table, and so does the end of the file; a sub-table's entry is
# complete only then, because its keys may come in either order.
#
# One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside an
# inline table span lines, and the repository writes exactly that:
#
#   coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
#       "crash-injection",
#   ] }
#
# So while the value still has an unclosed `{` or `[`, the next lines are
# joined onto it with one space (and are not looked at as headers or
# entries). If the file ends first, the entry contributes nothing. The
# brackets are counted over the joined value as one text -- a string left
# open at the end of one line continues after the joining space -- but the
# joined value is never rescanned: _toml_bracket_scan carries its state from
# line to line and the pieces are joined once, so the join is linear in the
# entry's length.
sub cargo_link_dependency_names {
    my ($text) = @_;

    # Step 1: lines. A leading BOM is ignored, whether the text is raw bytes
    # (EF BB BF) or already decoded (U+FEFF). Split at LF only, keeping
    # trailing empty fields (-1), and drop one trailing CR per line.
    $text =~ s/\A(?:\xEF\xBB\xBF|\x{FEFF})//;
    my @lines = split /\n/, $text, -1;
    my $clean = sub {
        my ($raw) = @_;
        $raw =~ s/\r\z//;
        return _trim_spaces_and_tabs(_strip_toml_line_comment($raw));
    };

    my @names;
    my ($kind, $sub_key, $sub_path, $sub_package) = ('ignored');
    my $finish_table = sub {
        push @names, _ascii_lc($sub_package // $sub_key)
            if $kind eq 'sub' && defined $sub_path;
    };

    my $index = 0;
    while ($index < @lines) {
        my $line = $clean->($lines[$index++]);

        # Step 2: headers. Every header ends the current table.
        if ($line =~ /\A\[/ && $line =~ /\]\z/) {
            $finish_table->();
            ($kind, $sub_key) = _classify_cargo_link_header($line);
            ($sub_path, $sub_package) = (undef, undef);
            next;
        }

        my $equals = _index_toml_outside_strings($line, '=');
        next if $equals < 0;

        if ($kind eq 'inline') {
            # Step 3: inline entries, continued while a bracket is open.
            # The pieces are collected and joined once, and the scanner
            # state carries from piece to piece, so the join is linear.
            my $value = _trim_spaces_and_tabs(substr($line, $equals + 1));
            if ($value =~ /\A\{/) {
                my $scan = { depth => 0, quote => '', escape => 0 };
                _toml_bracket_scan($scan, $value);
                if ($scan->{depth} > 0) {
                    my @pieces = ($value);
                    my $closed = 0;
                    while (!$closed && $index < @lines) {
                        my $piece = $clean->($lines[$index++]);
                        _toml_bracket_scan($scan, ' ');
                        _toml_bracket_scan($scan, $piece);
                        push @pieces, $piece;
                        $closed = $scan->{depth} <= 0;
                    }
                    # If the file ended inside the value, there is no entry.
                    $value = $closed ? join(' ', @pieces) : undef;
                }
            }
            next unless defined $value;
            my $crate = _cargo_path_dependency_name(substr($line, 0, $equals), $value);
            push @names, $crate if defined $crate;
        } elsif ($kind eq 'sub') {
            # Step 4: only the keys `path` and `package` matter, bare or
            # quoted (`"path" = ...` is the same key); first wins.
            # `version`, `features`, `optional` ... are ignored, and so is
            # `Path`: TOML keys are case-sensitive.
            my $value = _toml_single_string(substr($line, $equals + 1));
            next unless defined $value;
            my $key = _toml_key_segment(substr($line, 0, $equals));
            next unless defined $key;
            if ($key eq 'path') {
                $sub_path //= $value;
            } elsif ($key eq 'package') {
                $sub_package //= $value;
            }
        }
    }
    $finish_table->();
    return @names;
}

# _classify_cargo_link_header($line) -- how to read the lines under a header.
#
# The line is already comment-stripped and trimmed, and starts with `[` and
# ends with `]`. Returns ($kind, $sub_key). The header is split into
# dotted-key segments at every `.` OUTSIDE a string and each segment is
# decoded, so a cfg header is one segment however much punctuation it quotes:
#
#   [target.'cfg(any(target_os = "linux", windows))'.dependencies]
#    target  C (one quoted segment)                   D
#
#   [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table pi
#   [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
#   ["dependencies"]                                      -> [dependencies]
#
# Exactly [D], [D.N], [target.C.D] and [target.C.D.N] are read; arrays of
# tables, malformed headers and every other segment list are ignored.
sub _classify_cargo_link_header {
    my ($line) = @_;
    return ('ignored') if $line =~ /\A\[\[/;    # array of tables, e.g. [[bin]]

    my @segments;
    for my $raw (_split_toml_outside_strings(substr($line, 1, length($line) - 2), '.', 0)) {
        my $segment = _toml_key_segment($raw);
        return ('ignored') unless defined $segment;
        push @segments, $segment;
    }
    # Drop `target` and the cfg, whatever it is.
    splice(@segments, 0, 2) if @segments >= 3 && $segments[0] eq 'target';
    return ('ignored') unless $CARGO_LINK_TABLE{ $segments[0] };
    return ('inline') if @segments == 1;
    return ('sub', $segments[1]) if @segments == 2;
    return ('ignored');
}

# _cargo_path_dependency_name($key, $value) -- the crate a complete inline
# entry names, or undef. Only path dependencies can point at a repository
# package; the name is the `package` rename if there is one, else the key:
#
#   logic-gates = { path = "../logic-gates" }                    -> logic-gates
#   gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
#   serde = "1"                                -> undef (not a table)
#   x = { version = "1", features = ["path"] } -> undef (no path field)
#   foo.path = "../foo"                        -> undef (dotted key)
#
# Fields are split at commas outside strings AND outside nested `[...]` or
# `{...}`, so a `features` array cannot leak a fake field and a nested
# table's `path` is not the entry's. A field key is decoded like an entry
# key, so `"path"` and `'path'` are the key `path` (TOML, and so Cargo, read
# them that way); the decoded key must then be exactly `path` or `package`
# (Cargo ignores `Package`; escapes are not decoded). The first usable field
# of each wins:
#
#   a = { "path" = "../a" }                 -> a
#   x = { 'package' = "a", path = "../a" }  -> a
#   a = { "Path" = "../a" }                 -> undef (keys are case-sensitive)
#   a = { path.x = "../a" }                 -> undef (dotted field key)
sub _cargo_path_dependency_name {
    my ($key, $value) = @_;
    my $crate = _toml_key_segment($key);
    return undef unless defined $crate;

    $value = _trim_spaces_and_tabs($value);
    return undef unless length($value) >= 2 && $value =~ /\A\{/ && $value =~ /\}\z/;

    my ($path, $package);
    for my $field (_split_toml_outside_strings(substr($value, 1, length($value) - 2), ',', 1)) {
        my $equals = _index_toml_outside_strings($field, '=');
        next if $equals < 0;
        my $field_value = _toml_single_string(substr($field, $equals + 1));
        next unless defined $field_value;
        # A dotted or malformed field key is no key at all (undef).
        my $field_key = _toml_key_segment(substr($field, 0, $equals));
        next unless defined $field_key;
        if ($field_key eq 'path') {
            $path //= $field_value;
        } elsif ($field_key eq 'package') {
            $package //= $field_value;
        }
    }
    return undef unless defined $path;
    return _ascii_lc($package // $crate);
}

# _toml_string_end($text, $start) -- index just past the string opening at
# $start (a `"` or `'`), or -1 if it is not closed on this line. In a basic
# "..." string a backslash makes the next character part of the string, so
# `\"` does not end it; a literal '...' string has no escapes.
sub _toml_string_end {
    my ($text, $start) = @_;
    my $quote = substr($text, $start, 1);
    my $length = length $text;
    for (my $index = $start + 1; $index < $length; $index++) {
        my $character = substr($text, $index, 1);
        if ($quote eq '"' && $character eq '\\') {
            $index++;
        } elsif ($character eq $quote) {
            return $index + 1;
        }
    }
    return -1;
}

# _strip_toml_line_comment($line) -- drop everything from the first `#`
# outside a string. An unclosed string runs to the end of the line.
sub _strip_toml_line_comment {
    my ($line) = @_;
    my $length = length $line;
    for (my $index = 0; $index < $length; $index++) {
        my $character = substr($line, $index, 1);
        if ($character eq '"' || $character eq "'") {
            my $end = _toml_string_end($line, $index);
            return $line if $end < 0;
            $index = $end - 1;
        } elsif ($character eq '#') {
            return substr($line, 0, $index);
        }
    }
    return $line;
}

# _index_toml_outside_strings($text, $target) -- index of the first $target
# character outside every string, or -1.
sub _index_toml_outside_strings {
    my ($text, $target) = @_;
    my $length = length $text;
    for (my $index = 0; $index < $length; $index++) {
        my $character = substr($text, $index, 1);
        if ($character eq '"' || $character eq "'") {
            my $end = _toml_string_end($text, $index);
            return -1 if $end < 0;
            $index = $end - 1;
        } elsif ($character eq $target) {
            return $index;
        }
    }
    return -1;
}

# _toml_bracket_scan($state, $text) -- count the `{` and `[` outside strings
# left unclosed, over text fed in pieces. $state is a hashref
# { depth, quote, escape }. Feeding "ab" and then "cd" leaves exactly the
# state feeding "abcd" would, because the state keeps what a rescan of the
# joined text would have to rediscover: whether it is inside a string (and
# which delimiter closes it), and whether the previous character was a
# backslash escaping the next one in a basic string. So a string still open
# at the end of one line continues after the joining space, as it does in
# the joined value:
#
#   feed '{ x = "q\\'   depth 1, inside "...", escape pending
#   feed ' '            the space is escaped: still inside the string
#   feed '", p = 1 }'   the string closes, `}` closes the table: depth 0
#
# Depth zero (or less) means an inline value is complete:
#
#   { path = "../a", features = [   -> 2
#   { path = "../a" }               -> 0
#
# Carrying this state is what keeps the continuation join linear: the reader
# used to rescan the whole growing value for every appended line, which is
# quadratic in a long entry.
sub _toml_bracket_scan {
    my ($state, $text) = @_;
    my ($depth, $quote, $escape) = @{$state}{qw(depth quote escape)};
    for my $character (split //, $text) {
        if ($escape) {
            $escape = 0;
        } elsif ($quote ne '') {
            if ($quote eq '"' && $character eq '\\') {
                $escape = 1;
            } elsif ($character eq $quote) {
                $quote = '';
            }
        } elsif ($character eq '"' || $character eq "'") {
            $quote = $character;
        } elsif ($character eq '{' || $character eq '[') {
            $depth++;
        } elsif ($character eq '}' || $character eq ']') {
            $depth--;
        }
    }
    @{$state}{qw(depth quote escape)} = ($depth, $quote, $escape);
    return;
}

# _split_toml_outside_strings($text, $separator, $nested) -- split at every
# separator outside strings. With $nested, a separator inside `[...]` or
# `{...}` does not split either. An unclosed string runs to the end of the
# last part.
sub _split_toml_outside_strings {
    my ($text, $separator, $nested) = @_;
    my @parts;
    my $start = 0;
    my $depth = 0;
    my $length = length $text;
    for (my $index = 0; $index < $length; $index++) {
        my $character = substr($text, $index, 1);
        if ($character eq '"' || $character eq "'") {
            my $end = _toml_string_end($text, $index);
            last if $end < 0;
            $index = $end - 1;
        } elsif ($nested && ($character eq '[' || $character eq '{')) {
            $depth++;
        } elsif ($nested && ($character eq ']' || $character eq '}')) {
            $depth--;
        } elsif ($character eq $separator && $depth == 0) {
            push @parts, substr($text, $start, $index - $start);
            $start = $index + 1;
        }
    }
    push @parts, substr($text, $start);
    return @parts;
}

# _toml_key_segment($raw) -- decode one key or header segment, or undef. A
# bare key (ASCII letters, digits, `_`, `-`) or exactly one complete string,
# whose value is the text between its quotes, verbatim (escapes are not
# decoded): `dependencies`, `'cfg(unix)'` -> `cfg(unix)`.
sub _toml_key_segment {
    my ($raw) = @_;
    my $segment = _trim_spaces_and_tabs($raw);
    return undef if $segment eq '';
    if ($segment =~ /\A["']/) {
        return undef unless _toml_string_end($segment, 0) == length $segment;
        return substr($segment, 1, length($segment) - 2);
    }
    # [A-Za-z0-9_-] spelled out: \w would also match non-ASCII letters.
    return $segment =~ /\A[A-Za-z0-9_-]+\z/ ? $segment : undef;
}

# _toml_single_string($raw) -- the content of a value that is exactly one
# non-empty string: `"1"` is one; `"a" "b"`, `1`, `""`, `["x"]` are not.
sub _toml_single_string {
    my ($raw) = @_;
    my $value = _trim_spaces_and_tabs($raw);
    return undef unless $value =~ /\A["']/;
    return undef unless _toml_string_end($value, 0) == length $value;
    my $content = substr($value, 1, length($value) - 2);
    return $content eq '' ? undef : $content;
}

# Trim only spaces and tabs (\s would also eat \f, \v, NBSP, ...).
sub _trim_spaces_and_tabs {
    my ($text) = @_;
    $text =~ s/\A[ \t]+//;
    $text =~ s/[ \t]+\z//;
    return $text;
}

# Lower-case only A-Z (`lc` may also fold Latin-1 or Unicode letters).
sub _ascii_lc {
    my ($text) = @_;
    $text =~ tr/A-Z/a-z/;
    return $text;
}

# _parse_elixir_deps -- Extract internal dependencies from mix.exs.
#
# Elixir packages use the :coding_adventures_<name> atom. mix.exs lines
# look like: {:coding_adventures_logic_gates, path: "../logic-gates"}
sub _parse_elixir_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $mixexs = File::Spec->catfile($pkg->{path}, 'mix.exs');
    my $text = _slurp($mixexs) // return ();

    my @deps;
    while ($text =~ /\{:(coding_adventures_[a-z_]+)/g) {
        my $dep = lc $1;
        push @deps, $known_ref->{$dep} // $known_ref->{":$dep"} // next;
    }
    return @deps;
}

# _parse_lua_deps -- Extract internal dependencies from a .rockspec file.
#
# Lua packages use the coding-adventures- prefix in their rockspec
# dependencies section. We look for rockspec files in the package directory.
sub _parse_lua_deps {
    my ($self, $pkg, $known_ref) = @_;

    # Find the .rockspec file (its name varies: name-version.rockspec).
    my $rockspec;
    opendir(my $dh, $pkg->{path}) or return ();
    for my $f (readdir $dh) {
        if ($f =~ /\.rockspec$/) {
            $rockspec = File::Spec->catfile($pkg->{path}, $f);
            last;
        }
    }
    closedir $dh;
    return () unless $rockspec;

    my $text = _slurp_utf8_metadata($pkg, $rockspec) // return ();

    my @deps;
    while ($text =~ /"(coding-adventures-[A-Za-z0-9-]+)/g) {
        my $dep = lc $1;
        next unless exists $known_ref->{$dep};
        next if $known_ref->{$dep} eq $pkg->{name};
        push @deps, $known_ref->{$dep};
    }
    return @deps;
}

# _parse_haskell_deps -- Extract internal dependencies from a .cabal file.
sub _parse_haskell_deps {
    my ($self, $pkg, $known_ref) = @_;    

    my $cabal;
    opendir(my $dh, $pkg->{path}) or return ();
    for my $f (readdir $dh) {
        if ($f =~ /\.cabal$/) {
            $cabal = File::Spec->catfile($pkg->{path}, $f);
            last;
        }
    }
    closedir $dh;
    return () unless $cabal;

    my $text = _slurp($cabal) // return ();

    my @deps;
    while ($text =~ /(coding-adventures-[a-zA-Z0-9-]+)/g) {
        my $dep = lc $1;
        next unless exists $known_ref->{$dep};
        next if $known_ref->{$dep} eq $pkg->{name};
        push @deps, $known_ref->{$dep};
    }
    return @deps;
}

sub _parse_swift_deps {
    my ($self, $pkg, $known_ref) = @_;
    my $manifest = File::Spec->catfile($pkg->{path}, 'Package.swift');
    my $text = _slurp($manifest) // return ();

    my @deps;
    while ($text =~ /\.package\s*\(\s*path\s*:\s*"([^"]+)"/g) {
        my $dep_path = $1;
        next if File::Spec->file_name_is_absolute($dep_path);
        my $cleaned = File::Spec->canonpath($dep_path);
        my @parts = grep { defined $_ && $_ ne '' } File::Spec->splitdir($cleaned);
        my $dep_dir = $parts[-1];
        next if !defined $dep_dir || $dep_dir eq '.' || $dep_dir eq '..';
        my $key = lc $dep_dir;
        push @deps, $known_ref->{$key} if exists $known_ref->{$key};
    }
    return @deps;
}

sub _parse_dotnet_deps {
    my ($self, $pkg, $known_ref) = @_;

    my @project_files;
    File::Find::find(
        {
            wanted => sub {
                return if -d $File::Find::name;
                return unless $_ =~ /\.(?:csproj|fsproj)\z/;
                push @project_files, $File::Find::name;
            },
            no_chdir => 1,
        },
        $pkg->{path},
    );

    my @deps;
    for my $project_file (@project_files) {
        my $text = _slurp($project_file) // next;
        while ($text =~ /<ProjectReference\s+Include\s*=\s*"([^"]+)"/g) {
            my $include = $1;
            next if File::Spec->file_name_is_absolute($include);
            my $cleaned = File::Spec->canonpath($include);
            my @parts = grep { defined $_ && $_ ne '' } File::Spec->splitdir($cleaned);
            next if !@parts;
            my $dep_dir = @parts >= 2 ? $parts[-2] : $parts[0];
            next if !defined $dep_dir || $dep_dir eq '.' || $dep_dir eq '..';
            my $key = lc $dep_dir;
            push @deps, $known_ref->{$key} if exists $known_ref->{$key};
        }
    }
    return @deps;
}

# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------

# _slurp -- Read an entire file into a scalar and return it.
#
# Returns undef if the file does not exist or cannot be read. This lets
# callers handle missing metadata files gracefully.
#
# @param $path -- File path.
# @return scalar with file contents, or undef.
sub _slurp {
    my ($path) = @_;
    return undef unless defined $path && -f $path;
    open(my $fh, '<', $path) or return undef;
    local $/;   # enable "slurp mode" — read entire file at once
    my $content = <$fh>;
    close $fh;
    return $content;
}

# _slurp_utf8_metadata -- Strictly decode a package metadata file as UTF-8.
#
# Package manifests are portable repository data. Decoding with FB_CROAK
# rejects malformed byte sequences instead of silently replacing them. The
# thrown object carries only stable package and repository-relative identities
# so callers never expose a host-specific absolute path.
sub _slurp_utf8_metadata {
    my ($pkg, $path) = @_;
    return unless defined $path && -f $path;

    open(my $fh, '<:raw', $path) or return;
    local $/;
    my $bytes = <$fh>;
    close $fh;

    my $text;
    my $decoded = eval {
        $text = decode('UTF-8', $bytes, FB_CROAK);
        1;
    };
    if (!$decoded) {
        die CodingAdventures::BuildTool::MetadataEncodingError->new(
            package  => $pkg->{name},
            manifest => _repository_relative_manifest($path),
        );
    }
    return $text;
}

sub _repository_relative_manifest {
    my ($path) = @_;
    my $normalized = File::Spec->canonpath($path);
    $normalized =~ s{\\}{/}g;
    # The greedy prefix makes this the LAST code/packages|programs pair, so a
    # checkout under a directory that itself holds one is never printed.
    return $1 if $normalized =~ m{\A(?:.*/)?(code/(?:packages|programs)/.+)\z}s;
    return File::Basename::basename($normalized);
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Resolver - Dependency resolution and graph building

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Resolver;
  use CodingAdventures::BuildTool::Discovery;

  my $d = CodingAdventures::BuildTool::Discovery->new(root => '/repo');
  $d->discover();

  my $r = CodingAdventures::BuildTool::Resolver->new();
  my $graph = $r->resolve($d->packages());

  for my $group ($graph->independent_groups()) {
      print "Build in parallel: ", join(', ', @$group), "\n";
  }

=head1 DESCRIPTION

Reads package metadata files (cpanfile, pyproject.toml, Gemfile, go.mod,
package.json, Cargo.toml, mix.exs, .rockspec) and builds a directed dependency
graph. Packages in the same independence group can be built in parallel.

=cut
