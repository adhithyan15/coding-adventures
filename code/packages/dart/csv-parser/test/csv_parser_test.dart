import 'package:coding_adventures_csv_parser/csv_parser.dart';
import 'package:test/test.dart';

void main() {
  group('parseCsv', () {
    test('parses a simple table', () {
      final rows = parseCsv('name,age,city\nAlice,30,New York\nBob,25,London\n');

      expect(rows, hasLength(2));
      expect(rows[0], {'name': 'Alice', 'age': '30', 'city': 'New York'});
      expect(rows[1], {'name': 'Bob', 'age': '25', 'city': 'London'});
    });

    test('returns values as strings', () {
      final rows = parseCsv('x,y\n1,2\n');

      expect(rows[0]['x'], '1');
      expect(rows[0]['y'], '2');
      expect(rows[0]['x'], isA<String>());
    });

    test('handles no trailing newline', () {
      final rows = parseCsv('name,value\nhello,world');

      expect(rows, hasLength(1));
      expect(rows[0], {'name': 'hello', 'value': 'world'});
    });

    test('parses quoted fields with embedded comma newline and escaped quotes', () {
      final rows = parseCsv(
        'id,note\n1,"Line one\nLine two"\n2,"She said ""hello"""\n',
      );

      expect(rows, hasLength(2));
      expect(rows[0]['note'], 'Line one\nLine two');
      expect(rows[1]['note'], 'She said "hello"');
    });

    test('handles empty fields', () {
      final rows = parseCsv('a,b,c\n1,,3\n,2,\n,,\n');

      expect(rows[0], {'a': '1', 'b': '', 'c': '3'});
      expect(rows[1], {'a': '', 'b': '2', 'c': ''});
      expect(rows[2], {'a': '', 'b': '', 'c': ''});
    });

    test('pads short rows and truncates long rows', () {
      final rows = parseCsv('a,b,c\n1\n2,two\n3,three,THREE,extra\n');

      expect(rows[0], {'a': '1', 'b': '', 'c': ''});
      expect(rows[1], {'a': '2', 'b': 'two', 'c': ''});
      expect(rows[2], {'a': '3', 'b': 'three', 'c': 'THREE'});
    });

    test('handles edge cases', () {
      expect(parseCsv(''), <CsvRow>[]);
      expect(parseCsv('name,age\n'), <CsvRow>[]);
      expect(parseCsv('name,age'), <CsvRow>[]);
      expect(parseCsv('a\n\n'), [
        {'a': ''},
      ]);
    });

    test('supports LF CRLF and CR line endings', () {
      expect(parseCsv('name\nAlice\nBob\n'), hasLength(2));
      expect(parseCsv('name\r\nAlice\r\nBob\r\n'), hasLength(2));
      expect(parseCsv('name\rAlice\rBob\r'), hasLength(2));
    });

    test('preserves whitespace', () {
      final rows = parseCsv('key,value\nspaced,  hello  \n');

      expect(rows[0]['value'], '  hello  ');
    });

    test('throws for unclosed quoted field', () {
      expect(
        () => parseCsv('name,value\n1,"unclosed\n'),
        throwsA(isA<UnclosedQuoteException>()),
      );
    });

    test('supports lenient quoted suffixes', () {
      final rows = parseCsv('a,b\n1,"hello"world\n');

      expect(rows[0]['b'], 'helloworld');
    });
  });

  group('parseCsvWithDelimiter', () {
    test('supports tab semicolon and pipe delimiters', () {
      expect(parseCsvWithDelimiter('name\tage\nAlice\t30\n', '\t')[0], {
        'name': 'Alice',
        'age': '30',
      });
      expect(parseCsvWithDelimiter('name;city\nAlice;Paris\n', ';')[0], {
        'name': 'Alice',
        'city': 'Paris',
      });
      expect(parseCsvWithDelimiter('a|b|c\n1|2|3\n', '|')[0], {
        'a': '1',
        'b': '2',
        'c': '3',
      });
    });

    test('rejects multi-character delimiters', () {
      expect(
        () => parseCsvWithDelimiter('a,b\n1,2\n', ',,'),
        throwsA(isA<ArgumentError>()),
      );
    });
  });

  group('UnclosedQuoteException', () {
    test('has the expected message', () {
      const error = UnclosedQuoteException();

      expect(error.message, 'Unclosed quoted field: EOF reached inside a quoted field');
      expect(error.toString(), contains('UnclosedQuoteException'));
    });
  });
}
