package com.codingadventures.sqlcsvsource

import com.codingadventures.csvparser.CsvParseException
import com.codingadventures.csvparser.parseCSV
import com.codingadventures.sqlexecutionengine.DataSource
import com.codingadventures.sqlexecutionengine.ExecutionResult
import com.codingadventures.sqlexecutionengine.QueryResult
import com.codingadventures.sqlexecutionengine.SqlExecutionEngine
import com.codingadventures.sqlexecutionengine.SqlExecutionException
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.NoSuchFileException
import java.nio.file.Path
import java.util.Locale

class CsvDataSource(private val directory: Path) : DataSource {
    constructor(directory: String) : this(Path.of(directory))

    fun directory(): Path = directory

    override fun schema(tableName: String): List<String> {
        return try {
            parseHeader(readTable(tableName)).toList()
        } catch (ex: CsvParseException) {
            throw SqlExecutionException("parsing CSV header for table $tableName", ex)
        }
    }

    override fun scan(tableName: String): List<Map<String, Any?>> {
        val stringRows = try {
            parseCSV(readTable(tableName))
        } catch (ex: CsvParseException) {
            throw SqlExecutionException("parsing CSV for table $tableName", ex)
        }

        return stringRows.map { row ->
            linkedMapOf<String, Any?>().also { coerced ->
                for ((key, value) in row) {
                    coerced[key] = coerce(value)
                }
            }
        }
    }

    private fun readTable(tableName: String): String {
        return try {
            Files.readString(csvPath(tableName), StandardCharsets.UTF_8)
        } catch (ex: NoSuchFileException) {
            throw SqlExecutionException("table not found: $tableName", ex)
        } catch (ex: IOException) {
            throw SqlExecutionException("reading CSV table: $tableName", ex)
        }
    }

    private fun csvPath(tableName: String): Path = directory.resolve("$tableName.csv")

    companion object {
        fun coerce(value: String): Any? {
            if (value.isEmpty()) return null

            return when (value.lowercase(Locale.ROOT)) {
                "true" -> true
                "false" -> false
                else -> {
                    value.toLongOrNull()
                        ?: value.toDoubleOrNull()
                        ?: value
                }
            }
        }
    }
}

object SqlCsvSource {
    fun csvDataSource(directory: String): CsvDataSource = CsvDataSource(directory)

    fun csvDataSource(directory: Path): CsvDataSource = CsvDataSource(directory)

    fun executeCsv(sql: String, directory: String): QueryResult =
        executeCsv(sql, Path.of(directory))

    fun executeCsv(sql: String, directory: Path): QueryResult =
        SqlExecutionEngine.execute(sql, csvDataSource(directory))

    fun tryExecuteCsv(sql: String, directory: String): ExecutionResult =
        tryExecuteCsv(sql, Path.of(directory))

    fun tryExecuteCsv(sql: String, directory: Path): ExecutionResult =
        SqlExecutionEngine.tryExecute(sql, csvDataSource(directory))
}

private fun parseHeader(source: String): List<String> {
    val header = firstRecord(source)
    if (header.isEmpty()) return emptyList()
    return parseRecord(header).filter { it.isNotEmpty() }
}

private fun firstRecord(source: String): String {
    val out = StringBuilder()
    var quoted = false
    var index = 0
    while (index < source.length) {
        val ch = source[index]
        if (quoted) {
            if (ch == '"') {
                if (index + 1 < source.length && source[index + 1] == '"') {
                    out.append(ch)
                    index += 1
                    out.append(source[index])
                } else {
                    quoted = false
                    out.append(ch)
                }
            } else {
                out.append(ch)
            }
        } else if (ch == '"') {
            quoted = true
            out.append(ch)
        } else if (ch == '\n' || ch == '\r') {
            return out.toString()
        } else {
            out.append(ch)
        }
        index += 1
    }
    if (quoted) throw CsvParseException("unclosed quoted field at end of header")
    return out.toString()
}

private fun parseRecord(record: String): List<String> {
    val fields = mutableListOf<String>()
    val field = StringBuilder()
    var quoted = false
    var afterQuote = false
    var index = 0

    while (index < record.length) {
        val ch = record[index]
        if (quoted) {
            if (ch == '"') {
                if (index + 1 < record.length && record[index + 1] == '"') {
                    field.append('"')
                    index += 1
                } else {
                    quoted = false
                    afterQuote = true
                }
            } else {
                field.append(ch)
            }
        } else if (ch == ',') {
            fields += field.toString().trim()
            field.clear()
            afterQuote = false
        } else if (ch == '"' && field.isEmpty() && !afterQuote) {
            quoted = true
        } else {
            field.append(ch)
        }
        index += 1
    }

    if (quoted) throw CsvParseException("unclosed quoted field in header")
    fields += field.toString().trim()
    return fields
}
