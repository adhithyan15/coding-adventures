package com.codingadventures.sqlcsvsource;

import com.codingadventures.csvparser.CsvParser;
import com.codingadventures.sqlexecutionengine.SqlExecutionEngine;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

public final class SqlCsvSource {
    private SqlCsvSource() {
    }

    public static CsvDataSource csvDataSource(String directory) {
        return csvDataSource(Path.of(directory));
    }

    public static CsvDataSource csvDataSource(Path directory) {
        return new CsvDataSource(directory);
    }

    public static SqlExecutionEngine.QueryResult executeCsv(String sql, String directory) {
        return executeCsv(sql, Path.of(directory));
    }

    public static SqlExecutionEngine.QueryResult executeCsv(String sql, Path directory) {
        return SqlExecutionEngine.execute(sql, csvDataSource(directory));
    }

    public static SqlExecutionEngine.ExecutionResult tryExecuteCsv(String sql, String directory) {
        return tryExecuteCsv(sql, Path.of(directory));
    }

    public static SqlExecutionEngine.ExecutionResult tryExecuteCsv(String sql, Path directory) {
        return SqlExecutionEngine.tryExecute(sql, csvDataSource(directory));
    }

    public static final class CsvDataSource implements SqlExecutionEngine.DataSource {
        private final Path directory;

        public CsvDataSource(String directory) {
            this(Path.of(directory));
        }

        public CsvDataSource(Path directory) {
            this.directory = directory;
        }

        public Path directory() {
            return directory;
        }

        @Override
        public List<String> schema(String tableName) {
            String source = readTable(tableName);
            try {
                return List.copyOf(parseHeader(source));
            } catch (CsvParser.CsvParseException ex) {
                throw new SqlExecutionEngine.SqlExecutionException("parsing CSV header for table " + tableName, ex);
            }
        }

        @Override
        public List<Map<String, Object>> scan(String tableName) {
            String source = readTable(tableName);
            List<Map<String, String>> stringRows;
            try {
                stringRows = CsvParser.parseCSV(source);
            } catch (CsvParser.CsvParseException ex) {
                throw new SqlExecutionEngine.SqlExecutionException("parsing CSV for table " + tableName, ex);
            }

            List<Map<String, Object>> rows = new ArrayList<>(stringRows.size());
            for (Map<String, String> stringRow : stringRows) {
                Map<String, Object> row = new LinkedHashMap<>();
                for (Map.Entry<String, String> entry : stringRow.entrySet()) {
                    row.put(entry.getKey(), coerce(entry.getValue()));
                }
                rows.add(row);
            }
            return rows;
        }

        private String readTable(String tableName) {
            try {
                return Files.readString(csvPath(tableName), StandardCharsets.UTF_8);
            } catch (NoSuchFileException ex) {
                throw new SqlExecutionEngine.SqlExecutionException("table not found: " + tableName, ex);
            } catch (IOException ex) {
                throw new SqlExecutionEngine.SqlExecutionException("reading CSV table: " + tableName, ex);
            }
        }

        private Path csvPath(String tableName) {
            return directory.resolve(tableName + ".csv");
        }
    }

    static Object coerce(String value) {
        if (value.isEmpty()) {
            return null;
        }

        String lower = value.toLowerCase(Locale.ROOT);
        if (lower.equals("true")) {
            return true;
        }
        if (lower.equals("false")) {
            return false;
        }

        try {
            return Long.parseLong(value);
        } catch (NumberFormatException ignored) {
            // Fall through to floating point parsing.
        }

        try {
            return Double.parseDouble(value);
        } catch (NumberFormatException ignored) {
            return value;
        }
    }

    private static List<String> parseHeader(String source) throws CsvParser.CsvParseException {
        String header = firstRecord(source);
        if (header.isEmpty()) {
            return List.of();
        }
        return parseRecord(header);
    }

    private static String firstRecord(String source) throws CsvParser.CsvParseException {
        StringBuilder out = new StringBuilder();
        boolean quoted = false;
        for (int index = 0; index < source.length(); index++) {
            char ch = source.charAt(index);
            if (quoted) {
                if (ch == '"') {
                    if (index + 1 < source.length() && source.charAt(index + 1) == '"') {
                        out.append(ch);
                        out.append(source.charAt(++index));
                    } else {
                        quoted = false;
                        out.append(ch);
                    }
                } else {
                    out.append(ch);
                }
            } else if (ch == '"') {
                quoted = true;
                out.append(ch);
            } else if (ch == '\n' || ch == '\r') {
                return out.toString();
            } else {
                out.append(ch);
            }
        }
        if (quoted) {
            throw new CsvParser.CsvParseException("unclosed quoted field at end of header");
        }
        return out.toString();
    }

    private static List<String> parseRecord(String record) throws CsvParser.CsvParseException {
        List<String> fields = new ArrayList<>();
        StringBuilder field = new StringBuilder();
        boolean quoted = false;
        boolean afterQuote = false;

        for (int index = 0; index < record.length(); index++) {
            char ch = record.charAt(index);
            if (quoted) {
                if (ch == '"') {
                    if (index + 1 < record.length() && record.charAt(index + 1) == '"') {
                        field.append('"');
                        index++;
                    } else {
                        quoted = false;
                        afterQuote = true;
                    }
                } else {
                    field.append(ch);
                }
            } else if (ch == ',' && !afterQuote) {
                fields.add(field.toString().trim());
                field.setLength(0);
            } else if (ch == ',') {
                fields.add(field.toString().trim());
                field.setLength(0);
                afterQuote = false;
            } else if (ch == '"' && field.length() == 0 && !afterQuote) {
                quoted = true;
            } else {
                field.append(ch);
            }
        }

        if (quoted) {
            throw new CsvParser.CsvParseException("unclosed quoted field in header");
        }
        fields.add(field.toString().trim());
        return fields.stream().filter(column -> !column.isEmpty()).toList();
    }
}
