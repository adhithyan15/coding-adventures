use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VerilogParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::VerilogParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

sub parse_verilog {
    my ($src) = @_;
    return CodingAdventures::VerilogParser->parse_verilog($src);
}

sub parse_verilog_version {
    my ($src, $version) = @_;
    return CodingAdventures::VerilogParser->parse_verilog($src, $version);
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
    my $node = CodingAdventures::VerilogParser::ASTNode->new('module_declaration', []);
    is( $node->rule_name, 'module_declaration', 'rule_name' );
    is( $node->is_leaf,   0,                    'not a leaf' );
    is( ref($node->children), 'ARRAY',          'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'MODULE', value => 'module', line => 1, col => 1 };
    my $leaf = CodingAdventures::VerilogParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,     'token',  'rule_name is token' );
    is( $leaf->is_leaf,       1,        'is_leaf returns 1' );
    is( $leaf->token->{type}, 'MODULE', 'token type is MODULE' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is source_text' => sub {
    my $ast = parse_verilog("module empty; endmodule");
    is( $ast->rule_name, 'source_text', 'root is source_text' );
};

subtest 'empty source parses' => sub {
    my $ast = parse_verilog("");
    is( $ast->rule_name, 'source_text', 'root is source_text' );
    is( scalar @{ $ast->children }, 0, 'no children' );
};

subtest 'default version matches explicit 2005' => sub {
    my $default_ast = parse_verilog("module empty; endmodule");
    my $versioned_ast = parse_verilog_version("module empty; endmodule", '2005');
    is( $default_ast->rule_name, $versioned_ast->rule_name, 'same root rule' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { parse_verilog_version("module empty; endmodule", '2099') },
        'unsupported version dies'
    );
};

subtest 'source_text contains description' => sub {
    my $ast  = parse_verilog("module empty; endmodule");
    my $desc = find_node($ast, 'description');
    ok( defined $desc, 'description node found' );
};

subtest 'description contains module_declaration' => sub {
    my $ast = parse_verilog("module empty; endmodule");
    my $md  = find_node($ast, 'module_declaration');
    ok( defined $md, 'module_declaration node' );
};

# ============================================================================
# Module declarations
# ============================================================================

subtest 'empty module' => sub {
    my $ast = parse_verilog("module empty; endmodule");
    is( $ast->rule_name, 'source_text', 'root' );
    ok( defined find_node($ast, 'module_declaration'), 'module_declaration' );
};

subtest 'multiple modules' => sub {
    my $src = "module a; endmodule\nmodule b; endmodule";
    my $ast = parse_verilog($src);
    my $count = count_nodes($ast, 'module_declaration');
    is( $count, 2, '2 module declarations' );
};

subtest 'module with port list' => sub {
    my $ast = parse_verilog("module m(input a, output b); endmodule");
    ok( defined find_node($ast, 'module_declaration'), 'module_declaration' );
    ok( defined find_node($ast, 'port_list'), 'port_list' );
};

# ============================================================================
# Continuous assignments
# ============================================================================

subtest 'assign y = a & b' => sub {
    my $ast = parse_verilog(<<'END');
module and_gate(input a, input b, output y);
  assign y = a & b;
endmodule
END
    ok( defined find_node($ast, 'continuous_assign'), 'continuous_assign' );
    ok( defined find_node($ast, 'assignment'),        'assignment' );
};

subtest 'assign with expression' => sub {
    my $ast = parse_verilog(<<'END');
module adder(input [7:0] a, input [7:0] b, output [7:0] sum);
  assign sum = a + b;
endmodule
END
    ok( defined find_node($ast, 'continuous_assign'), 'continuous_assign' );
};

# ============================================================================
# Always blocks
# ============================================================================

subtest 'always @(posedge clk) q <= d' => sub {
    my $ast = parse_verilog(<<'END');
module dff(input clk, input d, output reg q);
  always @(posedge clk)
    q <= d;
endmodule
END
    ok( defined find_node($ast, 'always_construct'),   'always_construct' );
    ok( defined find_node($ast, 'sensitivity_list'),   'sensitivity_list' );
    ok( defined find_node($ast, 'sensitivity_item'),   'sensitivity_item' );
    ok( defined find_node($ast, 'nonblocking_assignment'), 'nonblocking_assignment' );
};

subtest 'always @(*) combinational' => sub {
    my $ast = parse_verilog(<<'END');
module mux(input a, input b, input sel, output reg y);
  always @(*) begin
    if (sel) y = a;
    else y = b;
  end
endmodule
END
    ok( defined find_node($ast, 'always_construct'), 'always_construct' );
    ok( defined find_node($ast, 'if_statement'),     'if_statement' );
};

# ============================================================================
# If statements and blocks
# ============================================================================

subtest 'if/else in always block' => sub {
    my $ast = parse_verilog(<<'END');
module m(input clk, input reset, output reg q);
  always @(posedge clk) begin
    if (reset) q <= 0;
    else q <= q + 1;
  end
endmodule
END
    ok( defined find_node($ast, 'if_statement'), 'if_statement' );
    ok( defined find_node($ast, 'block_statement'), 'block_statement' );
};

# ============================================================================
# Case statements
# ============================================================================

subtest 'case statement' => sub {
    my $ast = parse_verilog(<<'END');
module m(input [1:0] sel, output reg [7:0] y);
  always @(*) begin
    case (sel)
      2'b00: y = 8'h00;
      2'b01: y = 8'hFF;
      default: y = 8'h55;
    endcase
  end
endmodule
END
    ok( defined find_node($ast, 'case_statement'), 'case_statement' );
    ok( defined find_node($ast, 'case_item'),      'case_item' );
};

# ============================================================================
# Expressions
# ============================================================================

subtest 'binary expression a + b' => sub {
    my $ast = parse_verilog(<<'END');
module m(input [7:0] a, input [7:0] b, output [7:0] y);
  assign y = a + b;
endmodule
END
    ok( defined find_node($ast, 'additive_expr'), 'additive_expr' );
};

subtest 'ternary expression sel ? a : b' => sub {
    my $ast = parse_verilog(<<'END');
module m(input sel, input a, input b, output y);
  assign y = sel ? a : b;
endmodule
END
    ok( defined find_node($ast, 'ternary_expr'), 'ternary_expr' );
};

subtest 'bitwise AND' => sub {
    my $ast = parse_verilog(<<'END');
module m(input a, input b, output y);
  assign y = a & b;
endmodule
END
    ok( defined find_node($ast, 'bit_and_expr'), 'bit_and_expr' );
};

# ============================================================================
# Module instantiation
# ============================================================================

subtest 'module instantiation' => sub {
    my $ast = parse_verilog(<<'END');
module top;
  wire a, b, y;
  and_gate u1 (.a(a), .b(b), .y(y));
endmodule
END
    ok( defined find_node($ast, 'module_instantiation'), 'module_instantiation' );
    ok( defined find_node($ast, 'instance'),             'instance' );
    ok( defined find_node($ast, 'named_port_connection'),'named_port_connection' );
};

# ============================================================================
# Mixed programs
# ============================================================================

subtest 'full counter module' => sub {
    my $src = <<'END';
module counter #(parameter WIDTH = 8) (
  input clk,
  input reset,
  output reg [WIDTH-1:0] count
);
  always @(posedge clk) begin
    if (reset)
      count <= 0;
    else
      count <= count + 1;
  end
endmodule
END
    my $ast = parse_verilog($src);
    is( $ast->rule_name, 'source_text', 'root is source_text' );
    ok( defined find_node($ast, 'module_declaration'),  'module_declaration' );
    ok( defined find_node($ast, 'parameter_port_list'), 'parameter_port_list' );
    ok( defined find_node($ast, 'always_construct'),    'always_construct' );
    ok( defined find_node($ast, 'if_statement'),        'if_statement' );
    ok( count_nodes($ast, 'nonblocking_assignment') >= 2, 'at least 2 nonblocking assignments' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'garbage input raises die' => sub {
    ok(
        dies { CodingAdventures::VerilogParser->parse_verilog('@@@ NOT VERILOG') },
        'garbage input causes die'
    );
};

done_testing;
