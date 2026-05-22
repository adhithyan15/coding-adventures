typedef CsvRow = Map<String, String>;

class UnclosedQuoteException implements Exception {
  const UnclosedQuoteException([
    this.message = 'Unclosed quoted field: EOF reached inside a quoted field',
  ]);

  final String message;

  @override
  String toString() => 'UnclosedQuoteException: $message';
}

List<CsvRow> parseCsv(String source) => parseCsvWithDelimiter(source, ',');

List<CsvRow> parseCsvWithDelimiter(String source, String delimiter) {
  if (delimiter.length != 1) {
    throw ArgumentError.value(delimiter, 'delimiter', 'must be one character');
  }

  final rawRows = _tokeniseRows(source, delimiter);
  if (rawRows.isEmpty) return <CsvRow>[];

  final header = rawRows.first;
  final dataRows = rawRows.skip(1);
  if (dataRows.isEmpty) return <CsvRow>[];

  return dataRows.map((row) => _buildRowMap(header, row)).toList();
}

enum _ParseState {
  fieldStart,
  inUnquotedField,
  inQuotedField,
  inQuotedMaybeEnd,
}

List<List<String>> _tokeniseRows(String source, String delimiter) {
  final rows = <List<String>>[];
  var currentRow = <String>[];
  final field = StringBuffer();
  var state = _ParseState.fieldStart;
  var index = 0;

  while (index < source.length) {
    final ch = source[index];

    switch (state) {
      case _ParseState.fieldStart:
        if (ch == '"') {
          state = _ParseState.inQuotedField;
        } else if (ch == delimiter) {
          currentRow.add('');
        } else if (_isNewlineStart(ch)) {
          if (currentRow.isNotEmpty) {
            currentRow.add('');
          }
          index = _consumeNewline(source, index);
          rows.add(currentRow);
          currentRow = <String>[];
        } else {
          field.write(ch);
          state = _ParseState.inUnquotedField;
        }
        break;
      case _ParseState.inUnquotedField:
        if (ch == delimiter) {
          currentRow.add(field.toString());
          field.clear();
          state = _ParseState.fieldStart;
        } else if (_isNewlineStart(ch)) {
          currentRow.add(field.toString());
          field.clear();
          index = _consumeNewline(source, index);
          rows.add(currentRow);
          currentRow = <String>[];
          state = _ParseState.fieldStart;
        } else {
          field.write(ch);
        }
        break;
      case _ParseState.inQuotedField:
        if (ch == '"') {
          state = _ParseState.inQuotedMaybeEnd;
        } else {
          field.write(ch);
        }
        break;
      case _ParseState.inQuotedMaybeEnd:
        if (ch == '"') {
          field.write('"');
          state = _ParseState.inQuotedField;
        } else if (ch == delimiter) {
          currentRow.add(field.toString());
          field.clear();
          state = _ParseState.fieldStart;
        } else if (_isNewlineStart(ch)) {
          currentRow.add(field.toString());
          field.clear();
          index = _consumeNewline(source, index);
          rows.add(currentRow);
          currentRow = <String>[];
          state = _ParseState.fieldStart;
        } else {
          field.write(ch);
          state = _ParseState.inUnquotedField;
        }
        break;
    }

    index += 1;
  }

  if (state == _ParseState.inQuotedField) {
    throw const UnclosedQuoteException();
  }

  if (state == _ParseState.inUnquotedField) {
    currentRow.add(field.toString());
  } else if (state == _ParseState.inQuotedMaybeEnd) {
    currentRow.add(field.toString());
  }

  if (currentRow.isNotEmpty) {
    rows.add(currentRow);
  }

  return rows;
}

CsvRow _buildRowMap(List<String> header, List<String> data) {
  final row = <String, String>{};
  for (var index = 0; index < header.length; index += 1) {
    row[header[index]] = index < data.length ? data[index] : '';
  }
  return row;
}

bool _isNewlineStart(String ch) => ch == '\n' || ch == '\r';

int _consumeNewline(String source, int index) {
  if (source[index] == '\r' &&
      index + 1 < source.length &&
      source[index + 1] == '\n') {
    return index + 1;
  }
  return index;
}
