/// Port of the reference `rust/mermaid-lexer` unit suite (135
/// tests), plus lane extras smoke-testing the 10 families the
/// reference leaves untested.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mermaid_lexer/mermaid_lexer.dart';
import 'package:test/test.dart';

/// Grammar type names (the reference's `type_name`, verbatim here).
List<String> names(List<Token> tokens) => [for (final t in tokens) t.type];

List<String> values(List<Token> tokens) => [for (final t in tokens) t.value];

List<String> nonEofValues(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') t.value
    ];

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

List<String> valuesOfType(List<Token> tokens, String type) => [
      for (final t in tokens)
        if (t.type == type) t.value,
    ];

bool windows2(List<String> values, String a, String b) {
  for (var i = 0; i + 1 < values.length; i++) {
    if (values[i] == a && values[i + 1] == b) return true;
  }
  return false;
}

void main() {
  group('mermaid-lexer', () {
    test('tokenizes block statements as complete lines', () {
      final tokens = tryTokenizeMermaidBlock(
          'block-beta\ncolumns 2\nA[Parser] B(IR)\nA --> B\n')!;
      expect(valuesOfType(tokens, 'STATEMENT_LINE'),
          equals(['columns 2', 'A[Parser] B(IR)', 'A --> B']));
    });

    test('tokenizes recursive block group boundaries', () {
      final tokens = tryTokenizeMermaidBlock(
          'block\n  block:pipeline:2\n    block\n      endpoint\n    end\n  end\n')!;
      expect(countType(tokens, 'GROUP_START'), equals(1));
      expect(countType(tokens, 'HEADER'), equals(2));
      expect(countType(tokens, 'GROUP_END'), equals(2));
      expect(tokens.any((t) => t.value.trim() == 'endpoint'), isTrue);
    });

    test('tokenizes labeled block group start as one grammar token', () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nblock:pipeline["Processing Pipeline"]:2\nA\nend\n')!;
      final group = tokens.firstWhere((t) => t.type == 'GROUP_START');
      expect(group.value.trim(),
          equals('block:pipeline["Processing Pipeline"]:2'));
    });

    test('tokenizes block arrow nodes through the pinned grammar', () {
      final tokens =
          tryTokenizeMermaidBlock('block\nflow<["Flow"]>(up, down)\n')!;
      expect(
          tokens.any((t) =>
              t.type == 'ARROW_NODE_LINE' &&
              t.value.trim() == 'flow<["Flow"]>(up, down)'),
          isTrue);
    });

    test('tokenizes block styled connections through the pinned grammar', () {
      final tokens = tryTokenizeMermaidBlock('block\nA -.-> B\nB ==> C\n')!;
      expect(tokens.any((t) => t.type == 'DOTTED_CONNECTION_LINE'), isTrue);
      expect(tokens.any((t) => t.type == 'THICK_CONNECTION_LINE'), isTrue);
    });

    test('tokenizes block bidirectional connections through the pinned grammar',
        () {
      final tokens =
          tryTokenizeMermaidBlock('block\nA <--> B\nB <-.-> C\nC <==> D\n')!;
      expect(countType(tokens, 'BIDIRECTIONAL_CONNECTION_LINE'), equals(3));
    });

    test(
        'tokenizes block circle and cross connections through the pinned grammar',
        () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nA o--x |handoff| B\nB x==o C\nC o-.-o D\nD --x A\n')!;
      expect(countType(tokens, 'MARKED_CONNECTION_LINE'), equals(4));
    });

    test('tokenizes compact block terminal markers through the pinned grammar',
        () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nsource-node--otarget-one\ntarget-one==xtarget-two\ntarget-two-.-otarget-three\n')!;
      expect(countType(tokens, 'COMPACT_MARKED_CONNECTION_LINE'), equals(3));
    });

    test('tokenizes inline block node connections through the pinned grammar',
        () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nid1["first"] --> id2["second"]\nstart("Start")==>stop("Stop")\n')!;
      expect(countType(tokens, 'INLINE_NODE_CONNECTION_LINE'), equals(2));
    });

    test('tokenizes block default class definition through the pinned grammar',
        () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nclassDef default fill:#e0f2fe,stroke:#0369a1,color:#0c4a6e\nA\n')!;
      expect(tokens.any((t) => t.type == 'DEFAULT_CLASSDEF_LINE'), isTrue);
    });

    test('tokenizes quoted labels between inline block nodes', () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nid1["first"] -- "solid" --> id2["second"]\nid2 == "thick" ==> id3\nid3 -. "dotted" -.-> id1\n')!;
      expect(
          countType(tokens, 'INLINE_LABELED_NODE_CONNECTION_LINE'), equals(3));
    });

    test('tokenizes block tilde links through the pinned grammar', () {
      final tokens = tryTokenizeMermaidBlock(
          'block\nsource[Start] ~~~ target(Stop)\nleft ~~~~ right\n')!;
      expect(countType(tokens, 'TILDE_NODE_CONNECTION_LINE'), equals(2));
    });

    test('tokenizes packet fields as complete lines', () {
      final tokens = tryTokenizeMermaidPacket(
          'packet-beta\n0-7: "Header"\n8-31: "Payload"\n')!;
      expect(valuesOfType(tokens, 'STATEMENT_LINE'),
          equals(['0-7: "Header"', '8-31: "Payload"']));
    });

    test('tokenizes kanban indentation as complete lines', () {
      final tokens =
          tryTokenizeMermaidKanban('kanban\n  Todo\n    task[Write tests]\n')!;
      expect(values(tokens), contains('  Todo'));
      expect(values(tokens), contains('    task[Write tests]'));
    });

    test('tokenizes architecture statements as complete lines', () {
      final tokens = tryTokenizeMermaidArchitecture(
          'architecture-beta\nservice api(server)[API]\napi:R --> L:db\n')!;
      expect(values(tokens), contains('service api(server)[API]'));
      expect(values(tokens), contains('api:R --> L:db'));
    });

    test('tokenizes radar axes and curves as complete statements', () {
      final tokens = tryTokenizeMermaidRadar(
          'radar-beta\naxis speed["Speed"], quality\ncurve product{80, 60}, baseline{\n50,\n40\n}\nshowLegend false, ticks 4\ngraticule polygon\n')!;
      expect(values(tokens), contains('axis speed["Speed"], quality'));
      expect(values(tokens),
          contains('curve product{80, 60}, baseline{\n50,\n40\n}'));
      expect(values(tokens), contains('showLegend false, ticks 4'));
      expect(values(tokens), contains('graticule polygon'));
    });

    test('tokenizes eventmodeling frames as complete statements', () {
      final tokens = tryTokenizeMermaidEventmodeling(
          'eventmodeling\nentity Sales.CartUI\nentity AddItem\ntf 01 ui CartUI\ntf 02 cmd AddItem ->> 01 [[AddItemData]]\ntf 03 evt ItemAdded `text`"accepted"\ndata AddItemData `json`{\n  "quantity": 2\n}\nnote 02 `md` {\n  Order reviewed\n}\ngwt 02\n  given evt CartCreated\n  when ui CartUI\n  then evt ItemAdded\n')!;
      for (final expected in [
        'entity Sales.CartUI',
        'entity AddItem',
        'tf 01 ui CartUI',
        'tf 02 cmd AddItem ->> 01',
        '[[AddItemData]]',
        'tf 03 evt ItemAdded `text`"accepted"',
        'data AddItemData `json`{\n  "quantity": 2\n}',
        'note 02 `md` {\n  Order reviewed\n}',
        'gwt 02\n  given evt CartCreated\n  when ui CartUI\n  then evt ItemAdded',
      ]) {
        expect(values(tokens), contains(expected));
      }
    });

    test('tokenizes mindmap indentation and shapes as complete lines', () {
      final tokens = tryTokenizeMermaidMindmap(
          'mindmap\n  root((Native))\n    Parser[Grammar first]\n')!;
      expect(valuesOfType(tokens, 'NODE_LINE'),
          equals(['  root((Native))', '    Parser[Grammar first]']));
    });

    test('version exists', () {
      expect(mermaidVersion, equals('0.84.0'));
    });

    test('tokenizes railroad ebnf expressions', () {
      final tokens = tryTokenizeMermaidRailroadEbnf(
          'railroad-ebnf-beta\nexpression = term, ("+" term)*;')!;
      expect(tokens.any((t) => t.type == 'HEADER'), isTrue);
      expect(tokens.any((t) => t.type == 'STAR'), isTrue);
      expect(tokens.any((t) => t.type == 'SEMICOLON'), isTrue);
    });

    test('tokenizes railroad abnf repetitions', () {
      final tokens = tryTokenizeMermaidRailroadAbnf(
          'railroad-abnf-beta\nrule = 2*4DIGIT / *"x";')!;
      expect(tokens.any((t) => t.type == 'HEADER'), isTrue);
      expect(tokens.any((t) => t.type == 'REPEAT'), isTrue);
      expect(tokens.any((t) => t.type == 'STAR'), isTrue);
    });

    test('tokenizes railroad peg operators', () {
      final tokens = tryTokenizeMermaidRailroadPeg(
          'railroad-peg-beta\nrule <- !"x" (item / .)+;')!;
      expect(tokens.any((t) => t.type == 'HEADER'), isTrue);
      expect(tokens.any((t) => t.type == 'ASSIGN'), isTrue);
      expect(tokens.any((t) => t.type == 'NOT'), isTrue);
      expect(tokens.any((t) => t.type == 'DOT'), isTrue);
    });

    test('tokenizes xychart statements case insensitively', () {
      final tokens = tryTokenizeMermaidXychart(
          'XYCHART horizontal\nX-AXIS "Quarter" [Q1, Q2]\nLINE Forecast [1, 2]\n')!;
      for (final expected in [
        'HEADER',
        'ORIENTATION',
        'X_AXIS_STATEMENT',
        'LINE_STATEMENT',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('tokenizes gantt core statements case insensitively', () {
      final tokens = tryTokenizeMermaidGantt(
          'GANTT\nTITLE Project\naccTitle: Timeline\nSECTION Build\nTask :done, t1, 2026-01-01, 2d\nCLICK t1 href "https://example.com" call inspect(t1)\n')!;
      for (final expected in [
        'HEADER',
        'TITLE_STATEMENT',
        'ACC_TITLE_STATEMENT',
        'SECTION_STATEMENT',
        'TASK_STATEMENT',
        'CLICK_STATEMENT',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('tokenizes gantt calendar and axis controls', () {
      final tokens = tryTokenizeMermaidGantt(
          'gantt\naxisFormat %m/%d\ntickInterval 1week\nexcludes weekends, 2026-01-01\nincludes 2026-01-03\ninclusiveEndDates\ntopAxis\ntodayMarker off\nweekday monday\nweekend friday\n')!;
      for (final expected in [
        'AXIS_FORMAT_STATEMENT',
        'TICK_INTERVAL_STATEMENT',
        'EXCLUDES_STATEMENT',
        'INCLUDES_STATEMENT',
        'INCLUSIVE_END_DATES',
        'TOP_AXIS',
        'TODAY_MARKER_STATEMENT',
        'WEEKDAY_STATEMENT',
        'WEEKEND_STATEMENT',
      ]) {
        expect(names(tokens), contains(expected), reason: 'missing $expected');
      }
    });

    test('tokenizes gantt today marker hex colors', () {
      final tokens = tryTokenizeMermaidGantt(
          'gantt\ntodayMarker stroke-width:5px,stroke:#00f,opacity:0.5\n')!;
      expect(
          tokens.any((t) =>
              t.type == 'TODAY_MARKER_STATEMENT' &&
              t.value.endsWith('stroke:#00f,opacity:0.5')),
          isTrue);
    });

    test('journey tokenizes sections and scored tasks', () {
      final tokens = tryTokenizeMermaidJourney(
          'JoUrNeY\naccTitle: Checkout\naccDescr {\nNative journey\n}\ntitle Checkout\nsection Payment<br/>Flow\nPay: 2: Alice, Bob')!;
      for (final expected in [
        'HEADER',
        'SECTION_STATEMENT',
        'TASK_STATEMENT',
        'ACC_TITLE_STATEMENT',
        'ACC_DESCR_BLOCK',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('journey rejects scores outside the documented range', () {
      for (final score in ['0', '6', '15']) {
        expect(
            tryTokenizeMermaidJourney(
                'journey\nsection Work\nTask: $score: Me'),
            isNull,
            reason: 'score $score');
      }
    });

    test('quadrant tokenizes native statements', () {
      final tokens = tokenizeMermaidQuadrant(
          'quadrantChart\ntitle Native portfolio\nx-axis Low --> High\nquadrant-1 Invest\nMetal: [0.75, 0.8]\n');
      for (final expected in [
        'HEADER',
        'TITLE_STATEMENT',
        'AXIS_STATEMENT',
        'QUADRANT_STATEMENT',
        'POINT_STATEMENT',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('quadrant tokenizes point classes and styles', () {
      final tokens = tokenizeMermaidQuadrant(
          'quadrantChart\nclassDef native color: #ff0000, radius: 10\nMetal:::native: [0.75, 0.8] stroke-width: 3px\n');
      expect(names(tokens), contains('CLASSDEF_STATEMENT'));
      expect(names(tokens), contains('POINT_STATEMENT'));
    });

    test('quadrant tokenizes accessibility metadata', () {
      final tokens = tokenizeMermaidQuadrant(
          'quadrantChart\naccTitle: Portfolio matrix\naccDescr {\nNative renderer priorities\nacross backends\n}\n');
      expect(names(tokens), contains('ACC_TITLE_STATEMENT'));
      expect(names(tokens), contains('ACC_DESCR_BLOCK'));
    });

    test('quadrant tokenizes keywords case insensitively', () {
      final tokens = tokenizeMermaidQuadrant(
          'QuAdRaNtChArT\nTiTlE Native portfolio\nX-AxIs Low ---> High\nQuAdRaNt-1 Invest\nClAsSdEf native radius: 10\n');
      for (final expected in [
        'HEADER',
        'TITLE_STATEMENT',
        'AXIS_STATEMENT',
        'QUADRANT_STATEMENT',
        'CLASSDEF_STATEMENT',
      ]) {
        expect(names(tokens), contains(expected), reason: 'missing $expected');
      }
    });

    test('quadrant skips leading and inline comments', () {
      final tokens = tokenizeMermaidQuadrant(
          '%% chart comment\nquadrantChart\nx-axis Low --> High %% axis comment\nMetal: [0.75, 0.8] %% point comment\n');
      expect(tokens.any((t) => t.value.contains('comment')), isFalse);
      expect(tokens.any((t) => t.type == 'AXIS_STATEMENT'), isTrue);
      expect(tokens.any((t) => t.type == 'POINT_STATEMENT'), isTrue);
    });

    test('tokenizes state transitions and aliases', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\ndirection LR\nstate "Still waiting" as Still\n[*] --> Still\nStill --> [*]: done\n');
      expect(names(tokens), contains('HEADER'));
      expect(countType(tokens, 'ARROW'), equals(2));
      expect(countType(tokens, 'EDGE_STATE'), equals(2));
      expect(
          tokens.any((t) =>
              t.value == 'Still waiting' || t.value == '"Still waiting"'),
          isTrue);
    });

    test('tokenizes state choice markers', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nstate First <<choice>>\nstate Second [[choice]]\n');
      expect(countType(tokens, 'CHOICE'), equals(2));
    });

    test('tokenizes state fork and join markers', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nstate Fork <<fork>>\nstate Join [[join]]\n');
      expect(
          valuesOfType(tokens, 'FORK_JOIN'), equals(['<<fork>>', '[[join]]']));
    });

    test('tokenizes state inline styles', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nstyle Ready fill:#fee2e2,stroke:#991b1b,color:#111827\n');
      expect(countType(tokens, 'HASH_COLOR'), equals(3));
      expect(tokens.where((t) => t.value == ',').length, equals(2));
    });

    test('tokenizes state class styles', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nclassDef warning fill:#fef3c7,stroke:#92400e\nclass Ready,Waiting warning\n');
      expect(tokens.any((t) => t.value == 'classDef'), isTrue);
      expect(tokens.any((t) => t.value == 'class'), isTrue);
      expect(tokens.where((t) => t.value == ',').length, equals(2));
    });

    test('tokenizes state inline class shorthand', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nStill:::quiet --> Moving:::active\n');
      expect(countType(tokens, 'STYLE_SEPARATOR'), equals(2));
    });

    test('tokenizes state attached notes', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nnote left of Ready: Waiting for work\nnote right of Running: Active\n');
      expect(tokens.where((t) => t.value == 'note').length, equals(2));
      expect(tokens.where((t) => t.value == 'of').length, equals(2));
      expect(tokens.where((t) => t.value == ':').length, equals(2));
    });

    test('tokenizes state multiline and floating notes', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nnote left of Ready\nFirst line\nSecond line\nend note\nnote "Floating" as N1\n');
      expect(countType(tokens, 'END_NOTE'), equals(1));
      expect(tokens.any((t) => t.type == 'STRING' && t.value == 'Floating'),
          isTrue);
    });

    test('tokenizes state line break variants as single tokens', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nReady: One<br>Two<br/>Three<br />Four<br\t/>Five\n');
      expect(valuesOfType(tokens, 'LINE_BREAK'),
          equals(['<br>', '<br/>', '<br />', '<br\t/>']));
    });

    test('tokenizes internal percent state identifiers without splitting', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nMoving --> Still%Active\n% standalone\n');
      expect(values(tokens), contains('Still%Active'));
      expect(values(tokens), contains('%'));
    });

    test('tokenizes state accessibility metadata', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\naccTitle: State lifecycle\naccDescr: Ready to running\naccDescr {\nMultiline description\n}\n');
      for (final name in ['ACC_TITLE', 'ACC_DESCR', 'ACC_DESCR_START']) {
        expect(tokens.any((t) => t.type == name), isTrue,
            reason: 'missing $name token');
      }
      expect(tokens.any((t) => t.value == '}'), isTrue);
    });

    test('tokenizes state click links', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nclick Ready "https://example.com" "Open ready state"\nclick Running href "https://example.com/run"\n');
      expect(tokens.where((t) => t.value == 'click').length, equals(2));
      expect(tokens.any((t) => t.value == 'href'), isTrue);
      expect(countType(tokens, 'STRING'), equals(3));
    });

    test('tokenizes state composite braces', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nstate Processing {\nQueued --> Running\n}\n');
      expect(tokens.any((t) => t.value == '{'), isTrue);
      expect(tokens.any((t) => t.value == '}'), isTrue);
    });

    test('tokenizes state concurrent region divider', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nstate Active {\nOff --> On\n--\nIdle --> Busy\n}\n');
      expect(tokens.any((t) => t.type == 'CONCURRENT'), isTrue);
    });

    test('tokenizes state hide empty description directive', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nhide empty description\nstate Junction <<choice>>\n');
      expect(tokens.any((t) => t.type == 'HIDE_EMPTY'), isTrue);
    });

    test('tokenizes state entities before hash colors', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\nReady: Metal #9829; native\nstyle Ready fill:#dbeafe\n');
      expect(tokens.any((t) => t.type == 'ENTITY'), isTrue);
      expect(tokens.any((t) => t.type == 'HASH_COLOR'), isTrue);
    });

    test('skips state hash comments without consuming entities or colors', () {
      final tokens = tokenizeMermaidState(
          'stateDiagram-v2\n# standalone comment\n#abc color-looking comment\nReady: Metal #9829; native\nReady --> Running # inline comment\nRunning --> Done #123; entity-looking comment\nstyle Ready fill:#dbeafe\n# final comment');
      expect(tokens.any((t) => t.value.contains('comment')), isFalse);
      expect(tokens.any((t) => t.type == 'ENTITY'), isTrue);
      expect(tokens.any((t) => t.type == 'HASH_COLOR'), isTrue);
      expect(tokens.last.type, equals('EOF'));
    });

    test('tokenizes header shapes and edges', () {
      final tokens =
          tokenizeMermaid('flowchart LR\nA[Start] -->|yes| B{Ship?}\n');
      expect(tokens[0].type, equals('KEYWORD'));
      expect(tokens[0].value, equals('flowchart'));
      expect(tokens[1].type, equals('DIRECTION'));
      expect(tokens[1].value, equals('LR'));
      expect(tokens[2].type, equals('NEWLINE'));
      expect(tokens[3].type, equals('NAME'));
      expect(tokens[3].value, equals('A'));
      expect(tokens[4].type, equals('RECT'));
      expect(tokens[4].value, equals('[Start]'));
      expect(tokens[5].type, equals('ARROW'));
      expect(tokens[6].type, equals('EDGE_LABEL'));
      expect(tokens[6].value, equals('|yes|'));
      expect(tokens[7].type, equals('NAME'));
      expect(tokens[7].value, equals('B'));
      expect(tokens[8].type, equals('DIAMOND'));
      expect(tokens[8].value, equals('{Ship?}'));
    });

    test('tokenizes swimlane labeled dotted endpoint markers', () {
      final tokens = tryTokenizeMermaidSwimlane(
          'swimlane-beta LR\na o-. circle .-o b\nb x-. cross .-x c\nc -. open .- d\n')!;
      expect(countType(tokens, 'DOTTED_LABEL_EDGE_LINE'), equals(3));
    });

    test('tokenizes swimlane arbitrary minimum link lengths', () {
      final tokens = tryTokenizeMermaidSwimlane(
          'swimlane-beta LR\na -----> b\nb ------ c\nc =====> d\nd ====== e\ne -....-> f\nf -....- g\n  ------> h\n')!;
      expect(countType(tokens, 'EDGE_LINE'), equals(4));
      expect(countType(tokens, 'DOTTED_LABEL_EDGE_LINE'), equals(2));
      expect(countType(tokens, 'CONTINUATION_LINE'), equals(1));
    });

    test('tokenizes swimlane long endpoint markers', () {
      expect(
          tryTokenizeMermaidSwimlane(
              'swimlane-beta LR\na o----> b\nb x-----o c\nc <====> d\nd o=====x e\ne <-...-> f\nf x-....-o g\n'),
          isNotNull);
    });

    test('tokenizes swimlane positional link styles', () {
      final tokens = tryTokenizeMermaidSwimlane(
          'swimlane-beta LR\na --> b\nlinkStyle default stroke:#64748b\nlinkStyle 0, 2 stroke:#dc2626;\n')!;
      expect(countType(tokens, 'LINK_STYLE_LINE'), equals(2));
    });

    test('tokenizes swimlane positional link curves', () {
      final tokens = tryTokenizeMermaidSwimlane(
          'swimlane-beta LR\na --> b\nlinkStyle default interpolate basis\nlinkStyle 0, 2 interpolate stepBefore;\n')!;
      expect(countType(tokens, 'LINK_STYLE_LINE'), equals(2));
    });

    test('comments are skipped', () {
      final tokens = tokenizeMermaid(
          '%% heading comment\nflowchart TD\n%% edge comment\nA --- B\n');
      expect(nonEofValues(tokens).any((v) => v.contains('comment')), isFalse);
      for (final expected in ['flowchart', 'TD', 'A', 'B']) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('shape tokens preserve delimiters', () {
      final tokens = tokenizeMermaid(
          'flowchart TB\nA((Circle)); B(Round); C[Rect]; D{Decision}\n');
      final customTokens = [
        for (final t in tokens) (t.type, t.value),
      ];
      expect(customTokens, contains(('CIRCLE', '((Circle))')));
      expect(customTokens, contains(('ROUND', '(Round)')));
      expect(customTokens, contains(('RECT', '[Rect]')));
      expect(customTokens, contains(('DIAMOND', '{Decision}')));
      expect(countType(tokens, 'SEMICOLON'), equals(3));
    });

    test('tokenizes pie sections', () {
      final tokens =
          tokenizeMermaidPie('pie showData\n"Dogs" : 60\n"Cats" : 40.5\n');
      expect(
          nonEofValues(tokens),
          equals([
            'pie',
            'showData',
            '\\n',
            'Dogs',
            ':',
            '60',
            '\\n',
            'Cats',
            ':',
            '40.5',
            '\\n'
          ]));
    });

    test('tokenizes sankey csv rows', () {
      final tokens = tokenizeMermaidSankey(
          'sankey-beta\nGrid,"Heating, homes",113.726\nGrid,Losses,56\n');
      expect(
          nonEofValues(tokens),
          equals([
            'sankey-beta',
            '\\n',
            'Grid',
            ',',
            'Heating, homes',
            ',',
            '113.726',
            '\\n',
            'Grid',
            ',',
            'Losses',
            ',',
            '56',
            '\\n'
          ]));
    });

    test('tokenizes gitgraph commands and attributes', () {
      final tokens = tokenizeMermaidGitgraph(
          'gitGraph LR:\ncommit id: "c1" msg: "Start"\nbranch develop\ncheckout develop\n');
      expect(nonEofValues(tokens)[0], equals('gitGraph'));
      for (final expected in ['LR', 'commit', 'c1', 'develop']) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('tokenizes er relationships and attributes', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER {\nstring name PK\n}\n');
      for (final expected in ['erDiagram', '||', '--', 'o{', 'PK']) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('tokenizes er accessibility metadata', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\naccTitle: Account model\naccDescr {\n  Customers and accounts\n}\n');
      expect(names(tokens), contains('ACC_TITLE'));
      expect(names(tokens), contains('ACC_DESCR_BLOCK'));
    });

    test('tokenizes er backtick escaped attributes', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nHOTEL {\n  `my.type` address\n  string `geo.accuracy` PK\n}\n');
      expect(valuesOfType(tokens, 'ESCAPED_ATTRIBUTE'),
          equals(['`my.type`', '`geo.accuracy`']));
    });

    test('tokenizes er comma separated attribute type parameters', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBUILDING {\n  public.geometry(point,4326) location\n}\n');
      expect(valuesOfType(tokens, 'COMMA_ATTRIBUTE_TYPE'),
          equals(['public.geometry(point,4326)']));
    });

    test('tokenizes er split attribute blocks with title field', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBOOK {\n  string title\n}\nBOOK {\n  string author\n}\n');
      expect(values(tokens).where((v) => v == '{').length, equals(2));
      expect(names(tokens), contains('TITLE_KW'));
    });

    test('tokenizes er statement keywords inside attribute blocks', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nSETTINGS {\n  string direction\n  string classDef\n  string class\n  string style\n}\n');
      for (final expected in [
        'DIRECTION_KW',
        'CLASSDEF_KW',
        'CLASS_KW',
        'STYLE_KW'
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('tokenizes er multi class composition', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nclassDef warm,cool fill:#fef3c7\nclass CUSTOMER,ORDER warm,cool\nPRODUCT:::warm,cool\nCUSTOMER:::warm ||--o{ ORDER:::cool : allows\n');
      for (final value in ['warm', 'cool', 'CUSTOMER', 'ORDER', 'PRODUCT']) {
        expect(values(tokens), contains(value));
      }
      expect(valuesOfType(tokens, 'STYLE_SEPARATOR'),
          equals([':::', ':::', ':::']));
    });

    test('tokenizes er direct style composition', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER\nORDER\nstyle CUSTOMER,ORDER fill:#dbeafe,stroke:#1d4ed8\nstyle CUSTOMER color:#14532d\n');
      expect(countType(tokens, 'STYLE_KW'), equals(2));
      expect(values(tokens), contains('CUSTOMER'));
      expect(values(tokens), contains('ORDER'));
      for (final color in ['#dbeafe', '#1d4ed8', '#14532d']) {
        expect(values(tokens), contains(color));
      }
    });

    test('tokenizes er bracketed attribute name suffix', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBOOK {\n  string author-ref[name](1)\n}\n');
      expect(valuesOfType(tokens, 'ATTRIBUTE_SUFFIX'), equals(['(1)']));
    });

    test('tokenizes er keywords case insensitively', () {
      final tokens = tokenizeMermaidEr(
          'ErDiAgRaM\nTiTlE Library model\nDiReCtIoN lR\nBOOK OnE oR mOrE To OnLy OnE AUTHOR : written by\n');
      for (final expected in [
        'HEADER',
        'TITLE_KW',
        'DIRECTION_KW',
        'DIRECTION',
        'ONE_OR_MORE',
        'IDENTIFYING',
        'ONLY_ONE',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('tokenizes er attribute keys case insensitively', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nACCOUNT {\n  string id pk\n  string owner_id fK, Uk\n}\n');
      expect(valuesOfType(tokens, 'ATTRIBUTE_KEY'), equals(['pk', 'fK', 'Uk']));
    });

    test(
        'tokenizes er numeric cardinalities without consuming numeric entities',
        () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER 1--one or more DELIVERY-ADDRESS : has\nROOT many to 1 1 : numeric child\n');
      expect(valuesOfType(tokens, 'NUMBER'), equals(['1', '1', '1']));
      expect(
          tokens.any(
              (t) => t.type == 'IDENTIFIER' && t.value == 'DELIVERY-ADDRESS'),
          isTrue);
    });

    test('tokenizes er numeric entities after one cardinality', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\na many to 1 1: label\na many to 1 12: label\n');
      expect(valuesOfType(tokens, 'NUMBER'), equals(['1', '1', '1', '12']));
    });

    test('distinguishes er entity strings from general quoted text', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\n"Customer Account" ||--|| ORDER : "100% complete"\n');
      final quoted = [
        for (final t in tokens)
          if (t.type == 'ENTITY_STRING')
            ('ENTITY_STRING', t.value)
          else if (t.type == 'STRING')
            ('STRING', t.value),
      ];
      expect(
          quoted,
          equals([
            ('ENTITY_STRING', 'Customer Account'),
            ('STRING', '100% complete')
          ]));
    });

    test('tokenizes er bare and quoted entity name forms', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nDUCK-BILLED_PLATYPUS ||--|| "Blo@rf?" : maps\n_foo\n"a~`!@#\$^&*()-=+[]{}|/;:\'.?¡⁄™€£‹¢›∞"\n');
      expect(
          valuesOfType(tokens, 'IDENTIFIER'), contains('DUCK-BILLED_PLATYPUS'));
      expect(valuesOfType(tokens, 'IDENTIFIER'), contains('_foo'));
      expect(valuesOfType(tokens, 'ENTITY_STRING'),
          equals(['Blo@rf?', 'a~`!@#\$^&*()-=+[]{}|/;:\'.?¡⁄™€£‹¢›∞']));
    });

    test('tokenizes er duplicate entity declarations', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER\nORDER\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER\nORDER\n');
      final identifiers = valuesOfType(tokens, 'IDENTIFIER');
      expect(identifiers.where((v) => v == 'CUSTOMER').length, equals(3));
      expect(identifiers.where((v) => v == 'ORDER').length, equals(3));
    });

    test('tokenizes er relationship label forms', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nACCOUNT ||--|| PROFILE : ""\nPROFILE ||--o{ ORDER : "contains"\nORDER }o..|| PRODUCT : assigned\n');
      final labels = [
        for (final t in tokens)
          if (t.type == 'STRING' || t.type == 'ENTITY_STRING') t.value,
      ];
      expect(labels, equals(['', 'contains']));
      expect(tokens.any((t) => t.value == 'assigned'), isTrue);
    });

    test('tokenizes er cyclical relationships', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nA ||--|{ B : likes\nB ||--|{ C : likes\nC ||--|{ A : likes\n');
      expect(countType(tokens, 'IDENTIFYING'), equals(3));
      expect(countType(tokens, 'ONE_OR_MORE'), equals(3));
      expect(tokens.where((t) => t.value == 'likes').length, equals(3));
    });

    test('tokenizes er complex topology', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER }|..|{ DELIVERY-ADDRESS : has\nCUSTOMER ||--o{ ORDER : places\nCUSTOMER ||--o{ INVOICE : "liable for"\nDELIVERY-ADDRESS ||--o{ ORDER : receives\nINVOICE ||--|{ ORDER : covers\nORDER ||--|{ ORDER-ITEM : includes\nPRODUCT-CATEGORY ||--|{ PRODUCT : contains\nPRODUCT ||--o{ ORDER-ITEM : "ordered in"\n');
      expect(
          tokens
              .where(
                  (t) => t.type == 'IDENTIFYING' || t.type == 'NON_IDENTIFYING')
              .length,
          equals(8));
      for (final expected in [
        'DELIVERY-ADDRESS',
        'ORDER-ITEM',
        'PRODUCT-CATEGORY'
      ]) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
    });

    test('tokenizes er variable node width fixture', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nPRIVATE_FINANCIAL_INSTITUTION {\n  string name\n  int turnover\n}\nPRIVATE_FINANCIAL_INSTITUTION ||..|{ EMPLOYEE : employs\nEMPLOYEE { bool officer_of_firm }\n');
      expect(
          tokens
              .where((t) => t.value == 'PRIVATE_FINANCIAL_INSTITUTION')
              .length,
          equals(2));
      for (final expected in [
        'turnover',
        'EMPLOYEE',
        'officer_of_firm',
        'employs'
      ]) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
    });

    test('tokenizes er mixed special entity topology', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER ||--o{ 1 : places\n1 ||--|{ u : contains\n1.5\nu ||--|| 2.5 : processes\n2.5 {\n  string id\n  float value\n}\nu {\n  varchar(50) name\n  int count\n}\n');
      for (final expected in [
        'CUSTOMER',
        '1',
        'u',
        '1.5',
        '2.5',
        'varchar(50)'
      ]) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
      expect(
          tokens
              .where(
                  (t) => t.type == 'IDENTIFYING' || t.type == 'NON_IDENTIFYING')
              .length,
          equals(3));
    });

    test('tokenizes er keyword substring in entity alias', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\np[Photograph] {\n  varchar(12) jobId\n  date dateCreated\n}\n');
      expect(tokens.any((t) => t.value == 'Photograph'), isTrue);
      expect(tokens.any((t) => t.value == 'graph'), isFalse);
      for (final expected in ['p', 'varchar(12)', 'jobId', 'dateCreated']) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
    });

    test('tokenizes er disconnected recursive topology', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nDEAD_PARROT\nHERMIT\nRECLUSE\nSOCIALITE }o--o{ SOCIALITE : "interacts with"\nRECLUSE }o--o{ SOCIALITE : avoids\n');
      for (final expected in [
        'DEAD_PARROT',
        'HERMIT',
        'RECLUSE',
        'SOCIALITE',
        'interacts with'
      ]) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
      expect(tokens.where((t) => t.value == 'SOCIALITE').length, equals(3));
      expect(countType(tokens, 'IDENTIFYING'), equals(2));
    });

    test('tokenizes er attribute constraint lists and comments', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER {\n  int customer_number PK, FK "comment1"\n  datetime customer_status_start_datetime PK,UK, FK\n  datetime customer_status_end_datetime PK , UK "comment3"\n  string customer_firstname\n  string customer_lastname "comment5"\n}\n');
      expect(
          tokens
              .where(
                  (t) => t.value == 'PK' || t.value == 'FK' || t.value == 'UK')
              .length,
          equals(7));
      for (final expected in ['comment1', 'comment3', 'comment5']) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
    });

    test('tokenizes er key prefix attribute names', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBOOK {\n  int fk_title FK\n  string pk_author PK\n  string uk_address UK\n  float pk_price PK "comment"\n}\n');
      for (final expected in [
        'fk_title',
        'pk_author',
        'uk_address',
        'pk_price'
      ]) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
      expect(tokens.where((t) => t.value == 'PK').length, equals(2));
      expect(tokens.where((t) => t.value == 'FK').length, equals(1));
      expect(tokens.where((t) => t.value == 'UK').length, equals(1));
    });

    test('tokenizes er attribute type forms', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER {\n  type~T~ genericValue\n  string[] readers FK\n  character(10) isbn PK, FK\n  string? alias UK\n}\n');
      for (final expected in ['type~T~', 'string', 'character(10)', '?']) {
        expect(nonEofValues(tokens), contains(expected));
      }
      expect(
          nonEofValues(tokens)
              .where((v) => v == 'FK' || v == 'PK' || v == 'UK')
              .length,
          equals(4));
    });

    test('tokenizes er aliases and class shorthand after relationships', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nORDER ||--o{ customer : placed_by\ncustomer["Customer Account"]:::primary {\n  string id PK\n}\n');
      expect(names(tokens), contains('ENTITY_STRING'));
      expect(names(tokens), contains('STYLE_SEPARATOR'));
      for (final expected in ['[', 'Customer Account', ']', ':::']) {
        expect(values(tokens), contains(expected));
      }
    });

    test('tokenizes er parent cardinality and u entity name', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nPROJECT u--o{ TEAM_MEMBER : parent\nu ||--|| ORDER : places\n');
      expect(valuesOfType(tokens, 'MD_PARENT'), equals(['u', 'u']));
      expect(tokens.any((t) => t.value == 'PROJECT'), isTrue);
      expect(tokens.any((t) => t.value == 'TEAM_MEMBER'), isTrue);
    });

    test('tokenizes er empty and compact attribute blocks', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBOOK{}\nAUTHOR         {}\nBOOK{string title PK "Display title"}\n');
      expect(values(tokens).where((v) => v == '{').length, equals(3));
      expect(values(tokens).where((v) => v == '}').length, equals(3));
      for (final expected in ['string', 'title', 'PK', 'Display title']) {
        expect(values(tokens), contains(expected));
      }
    });

    test('tokenizes er relationship role line breaks as quoted text', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nPERSON ||--o| ACCOUNT : "has<br/>one"\n');
      expect(valuesOfType(tokens, 'ENTITY_STRING'), equals(['has<br/>one']));
    });

    test('tokenizes er prototype safe identifiers', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\n__proto__ ||--|| constructor : links\nconstructor ||--|| prototype : links\n');
      for (final expected in ['__proto__', 'constructor', 'prototype']) {
        expect(values(tokens), contains(expected));
      }
    });

    test('tokenizes er empty quoted relationship role', () {
      final tokens =
          tokenizeMermaidEr('erDiagram\nCUSTOMER ||--|{ ORDER : ""\n');
      expect(valuesOfType(tokens, 'STRING'), equals(['']));
    });

    test('tokenizes er numeric entity identifiers', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\n1\n2.5 {\n  string value\n}\n99.99\n2.5 ||--|| 99.99 : ""\n');
      expect(valuesOfType(tokens, 'NUMBER'),
          equals(['1', '2.5', '99.99', '2.5', '99.99']));
    });

    test('tokenizes er recursive relationships', () {
      final tokens =
          tokenizeMermaidEr('erDiagram\nNODE ||--o{ NODE : "leads to"\n');
      expect(tokens.where((t) => t.value == 'NODE').length, equals(2));
      expect(tokens.any((t) => t.value == 'leads to'), isTrue);
    });

    test('tokenizes er parallel relationships', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCAR ||--o{ PERSON : "insured for"\nCAR }o--|| PERSON : "owned by"\n');
      expect(tokens.where((t) => t.value == 'CAR').length, equals(2));
      expect(tokens.where((t) => t.value == 'PERSON').length, equals(2));
      expect(tokens.any((t) => t.value == 'insured for'), isTrue);
      expect(tokens.any((t) => t.value == 'owned by'), isTrue);
    });

    test('tokenizes er leading asterisk attribute names', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nBOOK {\n  string *title\n  id *the_Primary_Key PK\n}\n');
      expect(values(tokens), contains('*title'));
      expect(values(tokens), contains('*the_Primary_Key'));
    });

    test('tokenizes er attribute name boundaries', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nENTITY {\n  string name1\n  int value2\n}\n');
      expect(valuesOfType(tokens, 'IDENTIFIER'), contains('name1'));
      expect(valuesOfType(tokens, 'IDENTIFIER'), contains('value2'));
      expect(tryTokenizeMermaidEr('erDiagram\nBOOK {\n  string (author\n}\n'),
          isNull);
      expect(tryTokenizeMermaidEr('erDiagram\nBOOK {\n  string )author\n}\n'),
          isNull);
    });

    test('tokenizes er attribute type boundaries', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nENTITY {\n  PKType primary\n  PK id\n  1 numeric\n}\n');
      expect(tokens.any((t) => t.type == 'IDENTIFIER' && t.value == 'PKType'),
          isTrue);
      expect(tokens.any((t) => t.type == 'ATTRIBUTE_KEY' && t.value == 'PK'),
          isTrue);
      expect(tokens.any((t) => t.type == 'NUMBER' && t.value == '1'), isTrue);
    });

    test('tokenizes er attribute comment line break variants', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nENTITY {\n  string first "Primary<br>identifier"\n  string second "Second<br/>line"\n  string third "Third<br />line"\n}\n');
      expect(
          values(tokens).where((v) => v.contains('<br')).toList(),
          equals(
              ['Primary<br>identifier', 'Second<br/>line', 'Third<br />line']));
    });

    test('tokenizes er entity alias line break variants', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nFIRST["Primary<br>Entity"]\nSECOND["Second<br/>Entity"]\nTHIRD["Third<br />Entity"]\n');
      expect(
          values(tokens).where((v) => v.contains('<br')).toList(),
          equals(
              ['Primary<br>Entity', 'Second<br/>Entity', 'Third<br />Entity']));
    });

    test('tokenizes er title line break variants', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\ntitle "Domain<br>Model<br/>Across<br />Lines"\nENTITY\n');
      expect(values(tokens).where((v) => v.contains('<br')).toList(),
          equals(['Domain<br>Model<br/>Across<br />Lines']));
    });

    test('tokenizes er unicode symbol identifiers', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\n_**testẽζ➕Ø😀㌕ぼ**_\nperson[😀]\nperson ||--o| a : __hẽ😀__\n');
      expect(
          valuesOfType(tokens, 'IDENTIFIER'), contains('_**testẽζ➕Ø😀㌕ぼ**_'));
      expect(valuesOfType(tokens, 'IDENTIFIER'), contains('😀'));
      expect(valuesOfType(tokens, 'IDENTIFIER'), contains('__hẽ😀__'));
      final compact = tokenizeMermaidEr('erDiagram\nA||--||B:has\n');
      expect(compact.any((t) => t.value == '--'), isTrue);
      expect(compact.where((t) => t.value == '||').length, equals(2));
    });

    test('tokenizes er mixed relationship operators', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nCUSTOMER 1.-one or more DELIVERY_ADDRESS : mixed left\nCUSTOMER 1-.one or more POSTAL_ADDRESS : mixed right\n');
      expect(valuesOfType(tokens, 'NON_IDENTIFYING'), equals(['.-', '-.']));
    });

    test('tokenizes complete er symbolic cardinality matrix', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nA ||--|{ B : a\nC |o..o{ D : b\nE }|--|| F : c\nG }o..o| H : d\n');
      final cardinalities = [
        for (final t in tokens)
          if (t.type == 'ZERO_OR_ONE' ||
              t.type == 'ZERO_OR_MORE' ||
              t.type == 'ONE_OR_MORE' ||
              t.type == 'ONLY_ONE')
            t.value,
      ];
      expect(cardinalities,
          equals(['||', '|{', '|o', 'o{', '}|', '||', '}o', 'o|']));
    });

    test('tokenizes er cardinality word and numeric aliases', () {
      final tokens = tokenizeMermaidEr(
          'erDiagram\nA one or zero to many B : a\nC many(0) to many(1) D : b\nE only one optionally to 1+ F : c\nG 0+ optionally to 1 H : d\n');
      final cardinalities = [
        for (final t in tokens)
          if (t.type == 'NUMBER' ||
              t.type == 'ZERO_OR_ONE' ||
              t.type == 'ZERO_OR_MORE' ||
              t.type == 'ONE_OR_MORE' ||
              t.type == 'ONLY_ONE')
            t.value,
      ];
      expect(
          cardinalities,
          equals([
            'one or zero',
            'many',
            'many(0)',
            'many(1)',
            'only one',
            '1+',
            '0+',
            '1'
          ]));
    });

    test('tokenizes c4 macros and keyed arguments', () {
      final tokens = tokenizeMermaidC4(
          'C4Context\nPerson(user, "Customer", \$sprite="person")\nRel(user, bank, "Uses")\nUpdateElementStyle(user, \$shape=RoundedBoxShape())\n');
      for (final expected in [
        'C4Context',
        'Person',
        '\$sprite',
        'Rel',
        'RoundedBoxShape()'
      ]) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('keeps c4 macro prefixed aliases as identifiers', () {
      final tokens = tokenizeMermaidC4(
          'C4Context\nPerson(customerA, "Customer")\nSystem(SystemAA, "Bank")\nRel(SystemAA, customerA, "Uses")\n');
      for (final alias in ['customerA', 'SystemAA']) {
        expect(tokens.any((t) => t.type == 'IDENTIFIER' && t.value == alias),
            isTrue);
      }
      expect(tokens.any((t) => t.value == 'AA'), isFalse);
    });

    test('does not tokenize unsupported c4 rel b as a macro', () {
      final tokens = tokenizeMermaidC4('C4Context\nRel_B(a, b, "Uses")\n');
      expect(
          tokens.any((t) => t.type == 'RELATION_MACRO' && t.value == 'Rel_B'),
          isFalse);
    });

    test('tokenizes c4 legacy accessibility description', () {
      final tokens = tokenizeMermaidC4(
          'C4Context\naccDescription Customer banking landscape\nPerson(user, "Customer")\n');
      final description = tokens.firstWhere((t) => t.type == 'ACC_DESCRIPTION');
      expect(description.value,
          equals('accDescription Customer banking landscape'));
    });

    test('tokenizes sequence participants messages and notes', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant A as Alice\nA->>+Bob: Hello Bob\nnote right of Bob: Ready\n');
      for (final expected in ['sequenceDiagram', '->>', '+', 'note', 'Ready']) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('tokenizes sequence keywords case insensitively', () {
      final tokens = tokenizeMermaidSequence(
          'SeQuEnCeDiAgRaM\nPaRtIcIpAnT A As Alice\nNoTe RiGhT Of A: WRAP: Ready\n');
      for (final expected in [
        'participant',
        'as',
        'note',
        'right',
        'of',
        'wrap:'
      ]) {
        expect(values(tokens), contains(expected));
      }
    });

    test('tokenizes sequence control blocks', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nalt Ready\nA->>B: Go\nelse Waiting\nloop Retry\nB-->>A: Wait\nend\nend\n');
      for (final expected in ['alt', 'else', 'loop']) {
        expect(nonEofValues(tokens), contains(expected));
      }
      expect(nonEofValues(tokens).where((v) => v == 'end').length, equals(2));
    });

    test('tokenizes sequence participant lifecycle', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\ncreate participant Worker as Background Worker\ndestroy Worker\n');
      for (final expected in ['create', 'destroy', 'Worker']) {
        expect(nonEofValues(tokens), contains(expected));
      }
    });

    test('tokenizes sequence box color as one token', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nbox rgba(33,66,99,0.5) Services\nparticipant API\nend\n');
      expect(tokens.any((t) => t.value == 'rgba(33,66,99,0.5)'), isTrue);
    });

    test('tokenizes sequence hsl colors as one token', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nbox hsl(180, 100%, 50%) Services\nparticipant API\nend\nrect hsla(30, 100%, 50%, 0.25)\nAPI->>DB: Query\nend\n');
      expect(valuesOfType(tokens, 'COLOR'),
          equals(['hsl(180, 100%, 50%)', 'hsla(30, 100%, 50%, 0.25)']));
    });

    test('tokenizes sequence participant configuration', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant API@{ "type": "boundary", "alias": "Public API" }\n');
      expect(tokens.any((t) => t.value == 'API'), isTrue);
      expect(tokens.any((t) => t.type == 'CONFIG'), isTrue);
    });

    test('tokenizes sequence half arrows', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nA-|\\B: filled top\nA-|/B: filled bottom\nA-\\\\B: stick top\nA-//B: stick bottom\nA/|-B: reverse filled top\nA\\\\-B: reverse stick bottom\n');
      for (final expected in [
        'SOLID_FILLED_TOP',
        'SOLID_FILLED_BOTTOM',
        'SOLID_STICK_TOP',
        'SOLID_STICK_BOTTOM',
        'SOLID_REVERSE_FILLED_TOP',
        'SOLID_REVERSE_STICK_BOTTOM',
      ]) {
        expect(names(tokens), contains(expected));
      }
    });

    test('tokenizes sequence central connections', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nAlice->>()John: destination\nAlice()->>John: source\nJohn()->>()Alice: both\n');
      expect(countType(tokens, 'CENTRAL'), equals(4));
    });

    test('tokenizes sequence autonumber decimals', () {
      final tokens =
          tokenizeMermaidSequence('sequenceDiagram\nautonumber 10.5 2.25\n');
      expect(valuesOfType(tokens, 'NUMBER'), equals(['10.5', '2.25']));
    });

    test('tokenizes sequence actor links', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nlink Alice: Dashboard @ https://example.com/alice\nlinks Bob: {"Wiki": "https://example.com/wiki"}\n');
      expect(tokens.any((t) => t.type == 'URL'), isTrue);
      expect(tokens.any((t) => t.type == 'JSON_OBJECT'), isTrue);
    });

    test('tokenizes sequence actor properties', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nproperties Alice: {"role": "admin", "active": true}\n');
      expect(tokens.any((t) => t.value == 'properties'), isTrue);
      expect(tokens.any((t) => t.type == 'JSON_OBJECT'), isTrue);
    });

    test('tokenizes sequence actor details reference', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\ndetails Alice: alice-info\n');
      for (final expected in ['details', 'alice', '-', 'info']) {
        expect(tokens.any((t) => t.value == expected), isTrue);
      }
    });

    test('tokenizes sequence accessibility statements', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\naccTitle: Transfer flow\naccDescr: Banking interaction\n');
      expect(tokens.any((t) => t.value == 'accTitle'), isTrue);
      expect(tokens.any((t) => t.value == 'accDescr'), isTrue);
    });

    test('tokenizes multiline sequence accessibility description', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\naccDescr {\n  Transfers funds\n  between accounts\n}\n');
      expect(
          tokens.any((t) =>
              t.type == 'ACC_DESCR_BLOCK' &&
              t.value.contains('between accounts')),
          isTrue);
    });

    test('tokenizes sequence semicolon terminators', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram;participant Alice;Alice->>Bob: Hello;');
      expect(tokens.where((t) => t.value == ';').length, equals(3));
    });

    test('tokenizes sequence entities without splitting the semicolon', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nAlice->>Bob: I #9829; you #infin; times\n');
      expect(valuesOfType(tokens, 'ENTITY'), equals(['#9829;', '#infin;']));
      expect(tokens.any((t) => t.value == ';'), isFalse);
    });

    test('skips sequence hash comments without dropping entities', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\n# heading comment\nA->>B: I #9829; diagrams # trailing comment\n');
      expect(tokens.any((t) => t.value.contains('comment')), isFalse);
      expect(tokens.any((t) => t.value == '#9829;'), isTrue);
    });

    test('tokenizes sequence line break variants', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant Alice as One<br>Two<br/>Three<br />Four\nAlice->>Bob: One<br>Two<br/>Three<br />Four\n');
      expect(countType(tokens, 'LINE_BREAK'), equals(6));
      expect(tokens.any((t) => t.value == 'as' && t.type == 'AS'), isTrue);
    });

    test('tokenizes sequence wrap directives', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nAlice->>Bob:wrap: A long message\nnote over Alice,Bob: nowrap: A note\n');
      expect(
          valuesOfType(tokens, 'WRAP_DIRECTIVE'), equals(['wrap:', 'nowrap:']));
      expect(countType(tokens, 'COLON'), equals(2));
    });

    test('tokenizes multiword sequence actor references', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant Customer Portal\nCustomer Portal->>Order Service: Submit\n');
      expect(windows2(values(tokens), 'Customer', 'Portal'), isTrue);
      expect(windows2(values(tokens), 'Order', 'Service'), isTrue);
    });

    test('tokenizes hyphenated sequence actor references', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant Customer-Portal\nCustomer-Portal->>Order-Service: Submit\n');
      expect(tokens.where((t) => t.value == '-').length, equals(3));
    });

    test('tokenizes equals sign sequence actor references', () {
      final tokens = tokenizeMermaidSequence(
          'sequenceDiagram\nparticipant Alice=Wonderland\nAlice=Wonderland->>Bob: Submit\n');
      expect(tokens.where((t) => t.value == '=').length, equals(2));
    });

    test('tokenizes wardley flow links as grammar lines', () {
      final tokens = tryTokenizeMermaidWardley(
          'wardley-beta\nA -.-> B\nA +> B\nA +\'demand\'<> B\n')!;
      expect(valuesOfType(tokens, 'LINK_LINE').map((v) => v.trim()).toList(),
          equals(['A -.-> B', 'A +> B', 'A +\'demand\'<> B']));
    });

    test('tokenizes wardley accessibility metadata as grammar lines', () {
      final tokens = tryTokenizeMermaidWardley(
          'wardley-beta\naccTitle: Strategy map\naccDescr: Components and dependencies\n')!;
      expect(tokens.any((t) => t.type == 'ACC_TITLE_LINE'), isTrue);
      expect(tokens.any((t) => t.type == 'ACC_DESCR_LINE'), isTrue);
    });

    test('untested families lex their headers', () {
      // The reference leaves 10 families without tests; every one
      // must still construct, lex its header line, and end with EOF.
      final cases = <String, List<Token>? Function()>{
        'cynefin': () => tryTokenizeMermaidCynefin('cynefin-beta\n'),
        'info': () => tryTokenizeMermaidInfo('info\n'),
        'ishikawa': () => tryTokenizeMermaidIshikawa('ishikawa-beta\n'),
        'requirement': () =>
            tryTokenizeMermaidRequirement('requirementDiagram\n'),
        'timeline': () => tryTokenizeMermaidTimeline('timeline\n'),
        'treemap': () => tryTokenizeMermaidTreemap('treemap-beta\n'),
        'treeview': () => tryTokenizeMermaidTreeview('treeview-beta\n'),
        'venn': () => tryTokenizeMermaidVenn('venn-beta\n'),
        'zenuml': () => tryTokenizeMermaidZenuml('zenuml\n'),
        'railroad': () => tryTokenizeMermaidRailroad('railroad-beta\n'),
      };
      for (final entry in cases.entries) {
        final tokens = entry.value();
        expect(tokens, isNotNull, reason: entry.key);
        expect(tokens!.last.type, equals('EOF'), reason: entry.key);
      }
    });

    test('every factory constructs a working lexer', () {
      final factories = <String, GrammarLexer Function()>{
        'mermaid': () => createMermaidLexer('flowchart LR\n'),
        'pie': () => createMermaidPieLexer('pie\n'),
        'sankey': () => createMermaidSankeyLexer('sankey-beta\n'),
        'gitgraph': () => createMermaidGitgraphLexer('gitGraph\n'),
        'er': () => createMermaidErLexer('erDiagram\n'),
        'c4': () => createMermaidC4Lexer('C4Context\n'),
        'sequence': () => createMermaidSequenceLexer('sequenceDiagram\n'),
        'state': () => createMermaidStateLexer('stateDiagram-v2\n'),
        'quadrant': () => createMermaidQuadrantLexer('quadrantChart\n'),
        'journey': () => createMermaidJourneyLexer('journey\n'),
        'timeline': () => createMermaidTimelineLexer('timeline\n'),
        'mindmap': () => createMermaidMindmapLexer('mindmap\n'),
        'block': () => createMermaidBlockLexer('block-beta\n'),
        'packet': () => createMermaidPacketLexer('packet-beta\n'),
        'kanban': () => createMermaidKanbanLexer('kanban\n'),
        'architecture': () =>
            createMermaidArchitectureLexer('architecture-beta\n'),
        'radar': () => createMermaidRadarLexer('radar-beta\n'),
        'eventmodeling': () =>
            createMermaidEventmodelingLexer('eventmodeling\n'),
        'treemap': () => createMermaidTreemapLexer('treemap-beta\n'),
        'venn': () => createMermaidVennLexer('venn-beta\n'),
        'ishikawa': () => createMermaidIshikawaLexer('ishikawa-beta\n'),
        'wardley': () => createMermaidWardleyLexer('wardley-beta\n'),
        'cynefin': () => createMermaidCynefinLexer('cynefin-beta\n'),
        'treeview': () => createMermaidTreeviewLexer('treeview-beta\n'),
        'swimlane': () => createMermaidSwimlaneLexer('swimlane-beta\n'),
        'railroad': () => createMermaidRailroadLexer('railroad-beta\n'),
        'railroad_ebnf': () =>
            createMermaidRailroadEbnfLexer('railroad-ebnf-beta\n'),
        'railroad_abnf': () =>
            createMermaidRailroadAbnfLexer('railroad-abnf-beta\n'),
        'railroad_peg': () =>
            createMermaidRailroadPegLexer('railroad-peg-beta\n'),
        'info': () => createMermaidInfoLexer('info\n'),
        'zenuml': () => createMermaidZenumlLexer('zenuml\n'),
        'requirement': () =>
            createMermaidRequirementLexer('requirementDiagram\n'),
        'xychart': () => createMermaidXychartLexer('xychart horizontal\n'),
        'gantt': () => createMermaidGanttLexer('gantt\n'),
      };
      expect(factories.length, equals(34));
      for (final entry in factories.entries) {
        expect(entry.value().tokenize().last.type, equals('EOF'),
            reason: entry.key);
      }
    });
  });
}
