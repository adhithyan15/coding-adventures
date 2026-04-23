import 'package:coding_adventures_state_machine/state_machine.dart';
import 'package:test/test.dart';

void main() {
  group('DFA', () {
    DFA makeTurnstile() {
      return DFA(
        <String>{'locked', 'unlocked'},
        <String>{'coin', 'push'},
        <String, String>{
          transitionKey('locked', 'coin'): 'unlocked',
          transitionKey('locked', 'push'): 'locked',
          transitionKey('unlocked', 'coin'): 'unlocked',
          transitionKey('unlocked', 'push'): 'locked',
        },
        'locked',
        <String>{'unlocked'},
      );
    }

    test('processes events and traces transitions', () {
      final machine = makeTurnstile();
      expect(machine.process('coin'), 'unlocked');
      expect(machine.currentState, 'unlocked');
      expect(
        machine.trace,
        [
          const TransitionRecord(
            source: 'locked',
            event: 'coin',
            target: 'unlocked',
            actionName: null,
          ),
        ],
      );
    });

    test('checks acceptance without mutating state', () {
      final machine = makeTurnstile();
      machine.process('coin');

      expect(machine.accepts(['push', 'coin']), isTrue);
      expect(machine.currentState, 'unlocked');
      expect(machine.trace, hasLength(1));
    });

    test('reports reachability and missing transitions', () {
      final machine = DFA(
        <String>{'q0', 'q1', 'qDead'},
        <String>{'a', 'b'},
        <String, String>{
          transitionKey('q0', 'a'): 'q1',
          transitionKey('q1', 'a'): 'q0',
        },
        'q0',
        <String>{'q1'},
      );

      expect(machine.reachableStates(), <String>{'q0', 'q1'});
      expect(machine.isComplete(), isFalse);
      expect(machine.validate().join('\n'), contains('Unreachable states'));
      expect(machine.validate().join('\n'), contains('Missing transitions'));
    });

    test('renders dot and tables', () {
      final machine = makeTurnstile();

      expect(machine.toDot(), contains('digraph DFA'));
      expect(machine.toAscii(), contains('locked'));
      expect(machine.toTable().first, ['State', 'coin', 'push']);
    });
  });

  group('NFA', () {
    NFA makeContainsAb() {
      return NFA(
        <String>{'q0', 'q1', 'q2'},
        <String>{'a', 'b'},
        <String, Set<String>>{
          transitionKey('q0', 'a'): <String>{'q0', 'q1'},
          transitionKey('q0', 'b'): <String>{'q0'},
          transitionKey('q1', 'b'): <String>{'q2'},
          transitionKey('q2', 'a'): <String>{'q2'},
          transitionKey('q2', 'b'): <String>{'q2'},
        },
        'q0',
        <String>{'q2'},
      );
    }

    test('tracks epsilon closures and non-deterministic branching', () {
      final machine = NFA(
        <String>{'q0', 'q1', 'q2', 'q3'},
        <String>{'a'},
        <String, Set<String>>{
          transitionKey('q0', EPSILON): <String>{'q1'},
          transitionKey('q1', EPSILON): <String>{'q2'},
          transitionKey('q2', 'a'): <String>{'q3'},
        },
        'q0',
        <String>{'q3'},
      );

      expect(machine.currentStates, <String>{'q0', 'q1', 'q2'});
      expect(machine.process('a'), <String>{'q3'});
    });

    test('accepts and converts to a language-equivalent dfa', () {
      final nfa = makeContainsAb();
      final dfa = nfa.toDfa();

      expect(nfa.accepts(['a', 'b']), isTrue);
      expect(nfa.accepts(['b', 'a']), isFalse);
      expect(dfa.accepts(['a', 'b']), isTrue);
      expect(dfa.accepts(['b', 'a']), isFalse);
    });

    test('renders epsilon transitions in dot output', () {
      final machine = NFA(
        <String>{'q0', 'q1'},
        <String>{'a'},
        <String, Set<String>>{
          transitionKey('q0', EPSILON): <String>{'q1'},
        },
        'q0',
        <String>{'q1'},
      );

      expect(machine.toDot(), contains('ε'));
    });
  });

  group('Minimize', () {
    test('merges equivalent states', () {
      final machine = DFA(
        <String>{'q0', 'q1', 'q2'},
        <String>{'a', 'b'},
        <String, String>{
          transitionKey('q0', 'a'): 'q1',
          transitionKey('q0', 'b'): 'q2',
          transitionKey('q1', 'a'): 'q1',
          transitionKey('q1', 'b'): 'q1',
          transitionKey('q2', 'a'): 'q2',
          transitionKey('q2', 'b'): 'q2',
        },
        'q0',
        <String>{'q1', 'q2'},
      );

      final minimized = minimize(machine);
      expect(minimized.states.length, 2);
      expect(minimized.accepts(['a']), isTrue);
      expect(minimized.accepts(['b']), isTrue);
    });
  });

  group('ModalStateMachine', () {
    test('switches modes and resets target machines', () {
      final data = DFA(
        <String>{'text', 'tagDetected'},
        <String>{'char', 'openAngle'},
        <String, String>{
          transitionKey('text', 'char'): 'text',
          transitionKey('text', 'openAngle'): 'tagDetected',
          transitionKey('tagDetected', 'char'): 'text',
          transitionKey('tagDetected', 'openAngle'): 'tagDetected',
        },
        'text',
        <String>{'text'},
      );
      final tag = DFA(
        <String>{'readingName', 'tagDone'},
        <String>{'char', 'closeAngle'},
        <String, String>{
          transitionKey('readingName', 'char'): 'readingName',
          transitionKey('readingName', 'closeAngle'): 'tagDone',
          transitionKey('tagDone', 'char'): 'readingName',
          transitionKey('tagDone', 'closeAngle'): 'tagDone',
        },
        'readingName',
        <String>{'tagDone'},
      );

      final machine = ModalStateMachine(
        <String, DFA>{
          'data': data,
          'tag': tag,
        },
        <String, String>{
          transitionKey('data', 'enterTag'): 'tag',
          transitionKey('tag', 'exitTag'): 'data',
        },
        'data',
      );

      machine.switchMode('enterTag');
      machine.process('char');
      machine.process('closeAngle');
      expect(machine.activeMachine.currentState, 'tagDone');

      machine.switchMode('exitTag');
      machine.switchMode('enterTag');
      expect(machine.activeMachine.currentState, 'readingName');
      expect(machine.modeTrace, hasLength(3));
    });
  });

  group('PushdownAutomaton', () {
    PushdownAutomaton makeBalancedParens() {
      return PushdownAutomaton(
        <String>{'q0', 'accept'},
        <String>{'(', ')'},
        <String>{'(', r'$'},
        <PDATransition>[
          const PDATransition(
            source: 'q0',
            event: '(',
            stackRead: r'$',
            target: 'q0',
            stackPush: <String>[r'$', '('],
          ),
          const PDATransition(
            source: 'q0',
            event: '(',
            stackRead: '(',
            target: 'q0',
            stackPush: <String>['(', '('],
          ),
          const PDATransition(
            source: 'q0',
            event: ')',
            stackRead: '(',
            target: 'q0',
            stackPush: <String>[],
          ),
          const PDATransition(
            source: 'q0',
            event: null,
            stackRead: r'$',
            target: 'accept',
            stackPush: <String>[],
          ),
        ],
        'q0',
        r'$',
        <String>{'accept'},
      );
    }

    test('accepts balanced parentheses and keeps stack trace', () {
      final machine = makeBalancedParens();

      expect(machine.accepts(['(', '(', ')', ')']), isTrue);
      expect(machine.accepts(['(', ')', ')']), isFalse);

      final trace = machine.processSequence(['(', ')']);
      expect(trace, hasLength(greaterThanOrEqualTo(2)));
      expect(machine.currentState, 'accept');
    });

    test('resets state and stack', () {
      final machine = makeBalancedParens();
      machine.process('(');
      expect(machine.stackTop, '(');

      machine.reset();
      expect(machine.currentState, 'q0');
      expect(machine.stack, [r'$']);
      expect(machine.trace, isEmpty);
    });
  });
}
