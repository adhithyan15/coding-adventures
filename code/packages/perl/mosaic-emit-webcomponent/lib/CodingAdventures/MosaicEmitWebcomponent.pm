package CodingAdventures::MosaicEmitWebcomponent;

# ============================================================================
# CodingAdventures::MosaicEmitWebcomponent — Emits a TypeScript Custom Element class (.ts)
# ============================================================================
#
# This is the Web Components backend for the Mosaic compiler pipeline:
#
#   Source → Lexer → Parser → Analyzer → MosaicIR → VM → **WCRenderer** → .ts
#
# The generated TypeScript class:
#   - Extends HTMLElement
#   - Uses Shadow DOM for style encapsulation
#   - Exposes Mosaic slots as property setters/getters
#   - Rebuilds shadow DOM via _render() on any property change
#   - Observes HTML attributes for primitive-typed slots
#
# Architecture: Fragment list + html string accumulation
# -------------------------------------------------------
#
# The renderer builds a flat list of RenderFragment hashrefs during VM
# traversal, then serializes them into a `_render()` method body using
# `let html = ''; html += ...;` statements.
#
# Security
# --------
#
# All text slot values use `this._escapeHtml()` before innerHTML insertion.
# Colors are always emitted as rgba() strings.
#
# Tag name convention
# -------------------
#
# PascalCase component names map to kebab-case with a "mosaic-" prefix:
#   ProfileCard → mosaic-profile-card
#   Button      → mosaic-button
#
# Public API
# ----------
#
#   my $result = CodingAdventures::MosaicEmitWebcomponent->emit($component);
#
# Returns a hashref: { filename => "Name.ts", content => "..." }

use strict;
use warnings;

use CodingAdventures::MosaicVm;
use CodingAdventures::MosaicAnalyzer;

our $VERSION = '0.01';

# ============================================================================
# Public API
# ============================================================================

sub emit {
    my ($class, $component) = @_;
    my $renderer = CodingAdventures::MosaicEmitWebcomponent::Renderer->new();
    my ($result, $err) = CodingAdventures::MosaicVm->run($component, $renderer);
    die "MosaicEmitWebcomponent: $err" if $err;
    return $result;
}

# ============================================================================
# Renderer class
# ============================================================================

package CodingAdventures::MosaicEmitWebcomponent::Renderer;

sub new {
    bless {
        component_name => '',
        slots          => [],
        fragments      => [],
        stack          => [],
    }, shift;
}

# ---- Renderer protocol -------------------------------------------------------

sub begin_component {
    my ($self, $name, $slots) = @_;
    $self->{component_name} = $name;
    $self->{slots}          = $slots;
    $self->{fragments}      = [];
    $self->{stack}          = [ { kind => 'component', fragments => [] } ];
}

sub end_component { }

sub begin_node {
    my ($self, $tag, $is_primitive, $props, $ctx) = @_;

    my $html      = $self->_build_open_tag($tag, $is_primitive, $props);
    my $self_close = _is_self_closing($tag);

    push @{ $self->{stack} }, {
        kind        => 'node',
        tag         => $tag,
        open_html   => $html,
        self_closing => $self_close,
        text_slot   => undef,
        text_literal => undef,
        frags       => [],
    };
}

sub end_node {
    my ($self, $tag) = @_;
    my $frame = pop @{ $self->{stack} };

    if ($frame->{self_closing}) {
        $self->_push_frag({ kind => 'self_closing', html => $frame->{open_html} });
        return;
    }

    $self->_push_frag({ kind => 'open_tag', html => $frame->{open_html} });

    # Emit children
    for my $frag (@{ $frame->{frags} }) {
        $self->_push_frag($frag);
    }

    # Text content
    if (defined $frame->{text_slot}) {
        $self->_push_frag({ kind => 'slot_ref', expr => $frame->{text_slot} });
    } elsif (defined $frame->{text_literal}) {
        $self->_push_frag({ kind => 'open_tag', html => $frame->{text_literal} });
    }

    $self->_push_frag({ kind => 'close_tag', tag => _to_html_tag($tag) });
}

sub render_slot_child {
    my ($self, $slot_name, $slot_type, $ctx) = @_;
    $self->_push_frag({ kind => 'slot_proj', slot_name => $slot_name });
}

sub begin_when {
    my ($self, $slot_name, $ctx) = @_;
    my $field = "_$slot_name";
    $field =~ s/-/_/g;
    $self->_push_frag({ kind => 'when_open', field => $field });
    push @{ $self->{stack} }, { kind => 'when', slot_name => $slot_name, frags => [] };
}

sub end_when {
    my ($self) = @_;
    my $frame = pop @{ $self->{stack} };
    $self->_push_frag($_) for @{ $frame->{frags} };
    $self->_push_frag({ kind => 'when_close' });
}

sub begin_each {
    my ($self, $slot_name, $item_name, $element_type, $ctx) = @_;
    my $field      = "_$slot_name";
    $field         =~ s/-/_/g;
    my $is_node_list = ($element_type->{kind} eq 'node' || $element_type->{kind} eq 'component') ? 1 : 0;
    $self->_push_frag({ kind => 'each_open', field => $field,
                         item_name => $item_name, is_node_list => $is_node_list });
    push @{ $self->{stack} }, { kind => 'each', slot_name => $slot_name, frags => [] };
}

sub end_each {
    my ($self) = @_;
    my $frame = pop @{ $self->{stack} };
    $self->_push_frag($_) for @{ $frame->{frags} };
    $self->_push_frag({ kind => 'each_close' });
}

sub emit {
    my ($self) = @_;
    my $content = $self->_build_file();
    return {
        filename => "$self->{component_name}.ts",
        content  => $content,
    };
}

# ---- Open tag builder --------------------------------------------------------

my %PRIMITIVE_HTML = (
    Column  => 'div',
    Row     => 'div',
    Box     => 'div',
    Stack   => 'div',
    Text    => 'span',
    Image   => 'img',
    Spacer  => 'div',
    Divider => 'hr',
    Scroll  => 'div',
    Icon    => 'span',
);

my %SELF_CLOSING_TAG = ( Image => 1, Divider => 1, Spacer => 1 );

my %BASE_STYLE_WC = (
    Column => 'display:flex;flex-direction:column',
    Row    => 'display:flex;flex-direction:row',
    Box    => 'position:relative',
    Stack  => 'position:relative',
    Scroll => 'overflow:auto',
);

sub _is_self_closing { exists $SELF_CLOSING_TAG{$_[0]} }

sub _to_html_tag {
    my ($tag) = @_;
    return $PRIMITIVE_HTML{$tag} // 'div';
}

sub _build_open_tag {
    my ($self, $tag, $is_primitive, $props) = @_;

    my $html_tag = $is_primitive ? ($PRIMITIVE_HTML{$tag} // 'div') : $self->_to_element_name($tag);

    my @style_parts;
    push @style_parts, $BASE_STYLE_WC{$tag} if $is_primitive && exists $BASE_STYLE_WC{$tag};

    my @attrs;
    my $text_slot;
    my $text_literal;

    for my $prop (@$props) {
        my $name  = $prop->{name};
        my $value = $prop->{value};

        if ($name eq 'content' && $tag eq 'Text') {
            if ($value->{kind} eq 'slot_ref') {
                $text_slot = $value->{slot_name};
            } else {
                $text_literal = $self->_value_to_html($value);
            }
            next;
        }
        if ($name eq 'source' && $tag eq 'Image') {
            push @attrs, 'src="' . $self->_value_to_html($value) . '"';
            next;
        }
        if ($name eq 'a11y-label') {
            push @attrs, 'aria-label="' . $self->_value_to_html($value) . '"';
            next;
        }

        push @style_parts, $self->_prop_to_css($name, $value);
    }

    my $style_str  = @style_parts ? ' style="' . join(';', @style_parts) . '"' : '';
    my $attrs_str  = @attrs ? ' ' . join(' ', @attrs) : '';

    return "<$html_tag$style_str$attrs_str>";
}

# ---- File builder ------------------------------------------------------------

sub _build_file {
    my ($self) = @_;

    my $name      = $self->{component_name};
    my $tag_name  = $self->_to_tag_name($name);
    my $class_tag = $name;

    # Property declarations
    my $prop_decls  = $self->_build_prop_decls();
    my $getters     = $self->_build_getters();
    my $observed    = $self->_build_observed_attrs();
    my $attr_changed = $self->_build_attr_changed();
    my $render_body = $self->_build_render_body();

    return <<TS;
// AUTO-GENERATED by Mosaic Perl compiler. Do not edit.

export class $class_tag extends HTMLElement {
$prop_decls
  static get observedAttributes(): string[] {
    return [$observed];
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback(): void {
    this._render();
  }

  attributeChangedCallback(name: string, _old: string | null, value: string | null): void {
$attr_changed
  }

$getters
  private _render(): void {
    let html = '';
$render_body
    this.shadowRoot!.innerHTML = html;
  }

  private _escapeHtml(s: unknown): string {
    return String(s ?? '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }
}

customElements.define('$tag_name', $class_tag);
TS
}

sub _build_prop_decls {
    my ($self) = @_;
    my @lines;
    for my $slot (@{ $self->{slots} }) {
        my $field   = $self->_field_name($slot->{name});
        my $ts_type = $self->_type_to_ts($slot->{type});
        my $undef   = $slot->{required} ? '' : ' | undefined';
        push @lines, "  private $field: $ts_type$undef;";
    }
    return join("\n", @lines);
}

sub _build_getters {
    my ($self) = @_;
    my @lines;
    for my $slot (@{ $self->{slots} }) {
        my $prop    = $slot->{name};
        my $field   = $self->_field_name($prop);
        my $ts_type = $self->_type_to_ts($slot->{type});
        push @lines,
            "  get $prop(): $ts_type { return this.$field; }",
            "  set $prop(v: $ts_type) { this.$field = v; this._render(); }";
    }
    return join("\n", @lines);
}

sub _build_observed_attrs {
    my ($self) = @_;
    my @prim_kinds = qw(text number bool image color);
    my %prim = map { $_ => 1 } @prim_kinds;
    my @attrs;
    for my $slot (@{ $self->{slots} }) {
        push @attrs, "'$slot->{name}'" if exists $prim{$slot->{type}{kind}};
    }
    return join(', ', @attrs);
}

sub _build_attr_changed {
    my ($self) = @_;
    my @prim_kinds = qw(text number bool image color);
    my %prim = map { $_ => 1 } @prim_kinds;
    my @lines = ('    switch (name) {');
    for my $slot (@{ $self->{slots} }) {
        next unless exists $prim{$slot->{type}{kind}};
        my $field = $self->_field_name($slot->{name});
        push @lines,
            "      case '$slot->{name}': this.$field = value as any; this._render(); break;";
    }
    push @lines, '    }';
    return join("\n", @lines);
}

sub _build_render_body {
    my ($self) = @_;
    # The fragments are in the root component frame
    my $comp_frame = $self->{stack}[0];
    my @lines;
    for my $frag (@{ $comp_frame->{fragments} }) {
        push @lines, $self->_frag_to_code($frag);
    }
    return join("\n", @lines);
}

sub _frag_to_code {
    my ($self, $frag) = @_;
    my $kind = $frag->{kind};

    if ($kind eq 'open_tag') {
        my $html = $frag->{html};
        $html =~ s/\\/\\\\/g;
        $html =~ s/'/\\'/g;
        return "    html += '$html';";
    }
    if ($kind eq 'close_tag') {
        return "    html += '</$frag->{tag}>';";
    }
    if ($kind eq 'self_closing') {
        my $html = $frag->{html};
        $html =~ s/>$/ \/>/;
        $html =~ s/\\/\\\\/g;
        $html =~ s/'/\\'/g;
        return "    html += '$html';";
    }
    if ($kind eq 'slot_ref') {
        my $field = $self->_field_name($frag->{slot_name} // '');
        return "    html += \`\${this._escapeHtml(this.$field)}\`;";
    }
    if ($kind eq 'slot_proj') {
        return "    html += '<slot name=\"$frag->{slot_name}\"></slot>';";
    }
    if ($kind eq 'when_open') {
        return "    if (this.$frag->{field}) {";
    }
    if ($kind eq 'when_close') {
        return "    }";
    }
    if ($kind eq 'each_open') {
        return "    this.$frag->{field}.forEach(($frag->{item_name}) => {";
    }
    if ($kind eq 'each_close') {
        return "    });";
    }
    return "    // unknown frag: $kind";
}

# ---- Value helpers -----------------------------------------------------------

sub _value_to_html {
    my ($self, $value) = @_;
    my $kind = $value->{kind};
    if ($kind eq 'string') {
        my $v = $value->{value};
        $v =~ s/&/&amp;/g;
        $v =~ s/</&lt;/g;
        $v =~ s/>/&gt;/g;
        $v =~ s/"/&quot;/g;
        return $v;
    }
    return $value->{value}                          if $kind eq 'number';
    return $value->{value} ? 'true' : 'false'       if $kind eq 'bool';
    return $value->{value}                          if $kind eq 'ident';
    return $self->_color_to_rgba($value)             if $kind eq 'color';
    return $self->_dim_to_css($value->{value}, $value->{unit}) if $kind eq 'dimension';
    return '';
}

sub _prop_to_css {
    my ($self, $name, $value) = @_;
    my $css_name = $name;
    $css_name =~ s/([A-Z])/-\L$1/g;  # camel to kebab (no-op for already kebab)
    my $css_val;
    if ($value->{kind} eq 'color') {
        $css_val = $self->_color_to_rgba($value);
    } elsif ($value->{kind} eq 'dimension') {
        $css_val = $self->_dim_to_css($value->{value}, $value->{unit});
    } elsif ($value->{kind} eq 'string') {
        my $v = $value->{value};
        $v =~ s/^"(.*)"$/$1/;
        $css_val = $v;
    } else {
        $css_val = $value->{value} // '';
    }
    return "$css_name:$css_val";
}

sub _color_to_rgba {
    my ($self, $v) = @_;
    my $alpha = sprintf("%.4g", $v->{a} / 255);
    return "rgba($v->{r},$v->{g},$v->{b},$alpha)";
}

sub _dim_to_css {
    my ($self, $value, $unit) = @_;
    return "${value}%" if $unit eq '%';
    return "${value}px";
}

# ---- Naming helpers ----------------------------------------------------------

# Convert PascalCase component name to kebab-case with mosaic- prefix.
# ProfileCard → mosaic-profile-card
sub _to_tag_name {
    my ($self, $name) = @_;
    $name =~ s/([A-Z])/-\L$1/g;
    $name =~ s/^-//;
    return "mosaic-$name";
}

# Convert PascalCase to kebab-case for component references.
sub _to_element_name {
    my ($self, $tag) = @_;
    my $kebab = $tag;
    $kebab =~ s/([A-Z])/-\L$1/g;
    $kebab =~ s/^-//;
    return "mosaic-$kebab";
}

# Slot name → private field name: hyphen to underscore, prefix with _
sub _field_name {
    my ($self, $name) = @_;
    $name =~ s/-/_/g;
    return "_$name";
}

sub _type_to_ts {
    my ($self, $type) = @_;
    my %MAP = (
        text   => 'string',
        number => 'number',
        bool   => 'boolean',
        image  => 'string',
        color  => 'string',
        node   => 'HTMLElement',
    );
    my $kind = $type->{kind};
    return $MAP{$kind}                                  if exists $MAP{$kind};
    return 'HTMLElement'                                if $kind eq 'component';
    return $self->_type_to_ts($type->{element_type}) . '[]' if $kind eq 'list';
    return 'unknown';
}

# Push a fragment to the innermost frame
sub _push_frag {
    my ($self, $frag) = @_;
    my $top = $self->{stack}[-1];
    if ($top->{kind} eq 'component') {
        push @{ $top->{fragments} }, $frag;
    } else {
        push @{ $top->{frags} }, $frag;
    }
}

1;

__END__

=head1 NAME

CodingAdventures::MosaicEmitWebcomponent - Emits TypeScript Custom Element classes from Mosaic IR

=head1 SYNOPSIS

    use CodingAdventures::MosaicEmitWebcomponent;
    use CodingAdventures::MosaicAnalyzer;

    my ($component, $err) = CodingAdventures::MosaicAnalyzer->analyze($source);
    die $err if $err;

    my $result = CodingAdventures::MosaicEmitWebcomponent->emit($component);
    print $result->{filename};  # "MyComponent.ts"
    print $result->{content};

=head1 DESCRIPTION

Port of the TypeScript MosaicWebComponentRenderer to Perl. Drives the MosaicVM
with a Web Components renderer that emits a TypeScript Custom Element class
using Shadow DOM and html string accumulation.

=head1 METHODS

=head2 emit($component)

Emit a Web Component TypeScript file from the given MosaicIR component hashref.
Returns a hashref with keys C<filename> and C<content>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
