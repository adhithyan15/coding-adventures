use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VhdlParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::VhdlParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

sub parse_vhdl {
    my ($src) = @_;
    return CodingAdventures::VhdlParser->parse_vhdl($src);
}

sub parse_vhdl_version {
    my ($src, $version) = @_;
    return CodingAdventures::VhdlParser->parse_vhdl($src, $version);
}

sub find_node {
    my ($node, $rule_name) = @_;
    return undef unless ref($node);
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if defined $found;
    }
    return undef;
}

sub count_nodes {
    my ($node, $rule_name) = @_;
    return 0 unless ref($node);
    my $n = ($node->rule_name eq $rule_name) ? 1 : 0;
    for my $child (@{ $node->children }) {
        $n += count_nodes($child, $rule_name);
    }
    return $n;
}

# ============================================================================
# ASTNode unit tests
# ============================================================================

subtest 'ASTNode inner node' => sub {
    my $node = CodingAdventures::VhdlParser::ASTNode->new('entity_declaration', []);
    is( $node->rule_name, 'entity_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                    'not a leaf' );
    is( ref($node->children), 'ARRAY',          'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'ENTITY', value => 'entity', line => 1, col => 1 };
    my $leaf = CodingAdventures::VhdlParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token',  'rule_name is token' );
    is( $leaf->is_leaf,       1,        'is_leaf returns 1' );
    is( $leaf->token->{type}, 'ENTITY', 'token type is ENTITY' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is design_file' => sub {
    my $ast = parse_vhdl("entity empty is end entity;");
    is( $ast->rule_name, 'design_file', 'root is design_file' );
};

subtest 'empty source parses' => sub {
    my $ast = parse_vhdl("");
    is( $ast->rule_name, 'design_file', 'root is design_file' );
    is( scalar @{ $ast->children }, 0, 'no children' );
};

subtest 'default version matches explicit 2008' => sub {
    my $default_ast = parse_vhdl("entity empty is end entity;");
    my $versioned_ast = parse_vhdl_version("entity empty is end entity;", '2008');
    is( $default_ast->rule_name, $versioned_ast->rule_name, 'same root rule' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { parse_vhdl_version("entity empty is end entity;", '2099') },
        'unsupported version dies'
    );
};

subtest 'design_file contains design_unit' => sub {
    my $ast  = parse_vhdl("entity empty is end entity;");
    my $unit = find_node($ast, 'design_unit');
    ok( defined $unit, 'design_unit node found' );
};

# ============================================================================
# Entity declarations
# ============================================================================

subtest 'empty entity' => sub {
    my $ast = parse_vhdl("entity empty is end entity;");
    is( $ast->rule_name, 'design_file', 'root' );
    ok( defined find_node($ast, 'entity_declaration'), 'entity_declaration' );
};

subtest 'entity with ports' => sub {
    my $ast = parse_vhdl(<<'END');
entity half_adder is
  port (a, b : in std_logic; sum, carry : out std_logic);
end entity half_adder;
END
    ok( defined find_node($ast, 'entity_declaration'), 'entity_declaration' );
    ok( defined find_node($ast, 'port_clause'),        'port_clause' );
    ok( defined find_node($ast, 'interface_list'),     'interface_list' );
};

subtest 'multiple entities' => sub {
    my $src = <<'END';
entity a is end entity;
entity b is end entity;
END
    my $ast   = parse_vhdl($src);
    my $count = count_nodes($ast, 'entity_declaration');
    is( $count, 2, '2 entity declarations' );
};

subtest 'entity with generics' => sub {
    my $ast = parse_vhdl(<<'END');
entity parameterized is
  generic (WIDTH : integer := 8);
  port (clk : in std_logic; data : out std_logic_vector(WIDTH-1 downto 0));
end entity parameterized;
END
    ok( defined find_node($ast, 'entity_declaration'), 'entity_declaration' );
    ok( defined find_node($ast, 'generic_clause'),     'generic_clause' );
    ok( defined find_node($ast, 'port_clause'),        'port_clause' );
};

# ============================================================================
# Architecture bodies
# ============================================================================

subtest 'empty architecture' => sub {
    my $ast = parse_vhdl(<<'END');
entity empty is end entity;
architecture rtl of empty is
begin
end architecture rtl;
END
    ok( defined find_node($ast, 'entity_declaration'),  'entity_declaration' );
    ok( defined find_node($ast, 'architecture_body'),   'architecture_body' );
};

subtest 'architecture with signal declaration' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal carry : std_logic;
begin
end architecture rtl;
END
    ok( defined find_node($ast, 'architecture_body'),   'architecture_body' );
    ok( defined find_node($ast, 'signal_declaration'),  'signal_declaration' );
};

# ============================================================================
# Concurrent signal assignments
# ============================================================================

subtest 'concurrent signal assignment' => sub {
    my $ast = parse_vhdl(<<'END');
entity half_adder is
  port (a, b : in std_logic; sum : out std_logic);
end entity;

architecture rtl of half_adder is
begin
  sum <= a xor b;
end architecture;
END
    ok( defined find_node($ast, 'architecture_body'),            'architecture_body' );
    ok( defined find_node($ast, 'signal_assignment_concurrent'), 'signal_assignment_concurrent' );
};

subtest 'multiple concurrent assignments' => sub {
    my $ast = parse_vhdl(<<'END');
entity ha is
  port (a, b : in std_logic; s, c : out std_logic);
end entity;

architecture rtl of ha is
begin
  s <= a xor b;
  c <= a and b;
end architecture;
END
    my $count = count_nodes($ast, 'signal_assignment_concurrent');
    is( $count, 2, '2 concurrent signal assignments' );
};

# ============================================================================
# Process statements
# ============================================================================

subtest 'process with sensitivity list' => sub {
    my $ast = parse_vhdl(<<'END');
entity reg is
  port (clk, d : in std_logic; q : out std_logic);
end entity;

architecture rtl of reg is
begin
  process(clk)
  begin
    if rising_edge(clk) then
      q <= d;
    end if;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'process_statement'),  'process_statement' );
    ok( defined find_node($ast, 'sensitivity_list'),   'sensitivity_list' );
    ok( defined find_node($ast, 'if_statement'),       'if_statement' );
    ok( defined find_node($ast, 'signal_assignment_seq'), 'signal_assignment_seq' );
};

subtest 'process without sensitivity list' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal count : integer := 0;
begin
  process
  begin
    count <= count + 1;
    wait for 10 ns;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'process_statement'), 'process_statement' );
};

# ============================================================================
# If statements
# ============================================================================

subtest 'if/elsif/else' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal a, b, sel, y : std_logic;
begin
  process(sel, a, b)
  begin
    if sel = '1' then
      y <= a;
    elsif sel = '0' then
      y <= b;
    else
      y <= '0';
    end if;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'if_statement'), 'if_statement found' );
};

# ============================================================================
# Case statements
# ============================================================================

subtest 'case statement' => sub {
    my $ast = parse_vhdl(<<'END');
entity mux4 is
  port (sel : in std_logic_vector(1 downto 0);
        a, b, c, d : in std_logic;
        y : out std_logic);
end entity;

architecture rtl of mux4 is
begin
  process(sel, a, b, c, d)
  begin
    case sel is
      when "00" => y <= a;
      when "01" => y <= b;
      when "10" => y <= c;
      when others => y <= d;
    end case;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'case_statement'), 'case_statement' );
};

# ============================================================================
# For loops
# ============================================================================

subtest 'for loop' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
begin
  process
  begin
    for i in 0 to 7 loop
      null;
    end loop;
    wait;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'loop_statement'), 'loop_statement' );
    ok( defined find_node($ast, 'null_statement'), 'null_statement' );
};

# ============================================================================
# Component instantiation
# ============================================================================

subtest 'component instantiation' => sub {
    my $ast = parse_vhdl(<<'END');
entity top is end entity;
architecture rtl of top is
  component half_adder is
    port (a, b : in std_logic; sum, carry : out std_logic);
  end component;
  signal a, b, s, c : std_logic;
begin
  u1 : half_adder port map (a => a, b => b, sum => s, carry => c);
end architecture;
END
    ok( defined find_node($ast, 'component_declaration'),   'component_declaration' );
    ok( defined find_node($ast, 'component_instantiation'), 'component_instantiation' );
};

# ============================================================================
# Generate statements
# ============================================================================

subtest 'for-generate' => sub {
    my $ast = parse_vhdl(<<'END');
entity ripple is
  port (a, b : in std_logic_vector(3 downto 0); sum : out std_logic_vector(3 downto 0));
end entity;
architecture rtl of ripple is
begin
  gen: for i in 0 to 3 generate
    sum(i) <= a(i) xor b(i);
  end generate gen;
end architecture;
END
    ok( defined find_node($ast, 'generate_statement'), 'generate_statement' );
};

# ============================================================================
# Context items
# ============================================================================

subtest 'library and use clauses' => sub {
    my $ast = parse_vhdl(<<'END');
library ieee;
use ieee.std_logic_1164.all;

entity m is end entity;
END
    ok( defined find_node($ast, 'library_clause'), 'library_clause' );
    ok( defined find_node($ast, 'use_clause'),     'use_clause' );
};

# ============================================================================
# Expressions
# ============================================================================

subtest 'logical expression a and b' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is
  port (a, b : in std_logic; y : out std_logic);
end entity;
architecture rtl of m is
begin
  y <= a and b;
end architecture;
END
    ok( defined find_node($ast, 'logical_expr'), 'logical_expr' );
};

subtest 'relational expression a = b' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal a, b, y : std_logic;
begin
  process(a, b)
  begin
    if a = b then
      y <= '1';
    else
      y <= '0';
    end if;
  end process;
end architecture;
END
    ok( defined find_node($ast, 'relation'), 'relation' );
};

subtest 'arithmetic expression a + b' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal a, b, y : integer;
begin
  y <= a + b;
end architecture;
END
    ok( defined find_node($ast, 'adding_expr'), 'adding_expr' );
};

subtest 'concatenation a & b' => sub {
    my $ast = parse_vhdl(<<'END');
entity m is end entity;
architecture rtl of m is
  signal a, b : std_logic;
  signal y : std_logic_vector(1 downto 0);
begin
  y <= a & b;
end architecture;
END
    ok( defined find_node($ast, 'adding_expr'), 'adding_expr (& concat)' );
};

# ============================================================================
# Full design: counter
# ============================================================================

subtest 'full counter design' => sub {
    my $src = <<'END';
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity counter is
  generic (WIDTH : integer := 8);
  port (
    clk   : in  std_logic;
    reset : in  std_logic;
    count : out std_logic_vector(WIDTH-1 downto 0)
  );
end entity counter;

architecture rtl of counter is
  signal count_reg : unsigned(WIDTH-1 downto 0);
begin
  process(clk)
  begin
    if rising_edge(clk) then
      if reset = '1' then
        count_reg <= (others => '0');
      else
        count_reg <= count_reg + 1;
      end if;
    end if;
  end process;
  count <= std_logic_vector(count_reg);
end architecture rtl;
END
    my $ast = parse_vhdl($src);
    is( $ast->rule_name, 'design_file', 'root is design_file' );
    ok( defined find_node($ast, 'library_clause'),       'library_clause' );
    ok( defined find_node($ast, 'use_clause'),           'use_clause' );
    ok( defined find_node($ast, 'entity_declaration'),   'entity_declaration' );
    ok( defined find_node($ast, 'generic_clause'),       'generic_clause' );
    ok( defined find_node($ast, 'architecture_body'),    'architecture_body' );
    ok( defined find_node($ast, 'signal_declaration'),   'signal_declaration' );
    ok( defined find_node($ast, 'process_statement'),    'process_statement' );
    ok( count_nodes($ast, 'if_statement') >= 2,          'at least 2 if_statements' );
    ok( defined find_node($ast, 'signal_assignment_seq'), 'signal_assignment_seq' );
    ok( defined find_node($ast, 'signal_assignment_concurrent'), 'signal_assignment_concurrent' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'garbage input raises die' => sub {
    ok(
        dies { CodingAdventures::VhdlParser->parse_vhdl('@@@ NOT VHDL') },
        'garbage input causes die'
    );
};

done_testing;
