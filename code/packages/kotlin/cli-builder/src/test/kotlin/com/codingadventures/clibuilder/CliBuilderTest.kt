package com.codingadventures.clibuilder

import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlin.test.fail

class CliBuilderTest {
    @Test
    fun validatesMinimalSpec() {
        val path = writeSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "echo",
              "description": "Display text",
              "flags": [],
              "arguments": [
                {
                  "id": "text",
                  "name": "TEXT",
                  "description": "Text to display",
                  "type": "string",
                  "required": false,
                  "variadic": true,
                  "variadic_min": 0
                }
              ],
              "commands": []
            }
            """.trimIndent()
        )

        val result = CliBuilder.validateSpec(path)
        assertTrue(result.valid)
        assertEquals(emptyList(), result.errors)
    }

    @Test
    fun parsesCountFlagsAndExplicitFlags() {
        val outcome = parse(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "myapp",
              "description": "App with count flag",
              "version": "1.0.0",
              "flags": [
                {
                  "id": "verbose",
                  "short": "v",
                  "long": "verbose",
                  "description": "Increase verbosity",
                  "type": "count"
                },
                {
                  "id": "quiet",
                  "short": "q",
                  "long": "quiet",
                  "description": "Suppress output",
                  "type": "boolean"
                }
              ],
              "arguments": [],
              "commands": []
            }
            """.trimIndent(),
            "myapp",
            "-vqv",
        )

        val result = assertIs<ParseResult>(outcome)
        assertEquals(2L, result.flags["verbose"])
        assertEquals(true, result.flags["quiet"])
        assertEquals(listOf("verbose", "quiet", "verbose"), result.explicitFlags)
    }

    @Test
    fun supportsDefaultWhenPresentForEnumFlags() {
        val outcome = parse(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "myapp",
              "description": "App with enum optional value",
              "flags": [
                {
                  "id": "color",
                  "long": "color",
                  "description": "Colorize output",
                  "type": "enum",
                  "enum_values": ["always", "never", "auto"],
                  "default_when_present": "always",
                  "default": "auto"
                }
              ],
              "arguments": [
                {
                  "id": "file",
                  "display_name": "FILE",
                  "description": "Input file",
                  "type": "string",
                  "required": false
                }
              ],
              "commands": []
            }
            """.trimIndent(),
            "myapp",
            "--color",
            "file.txt",
        )

        val result = assertIs<ParseResult>(outcome)
        assertEquals("always", result.flags["color"])
        assertEquals("file.txt", result.arguments["file"])
    }

    @Test
    fun resolvesVariadicArgumentsWithLastWins() {
        val outcome = parse(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "cp",
              "description": "Copy files",
              "flags": [],
              "arguments": [
                {
                  "id": "source",
                  "name": "SOURCE",
                  "description": "Source file(s)",
                  "type": "string",
                  "required": true,
                  "variadic": true,
                  "variadic_min": 1
                },
                {
                  "id": "dest",
                  "name": "DEST",
                  "description": "Destination",
                  "type": "string",
                  "required": true
                }
              ],
              "commands": []
            }
            """.trimIndent(),
            "cp",
            "a.txt",
            "b.txt",
            "/dest",
        )

        val result = assertIs<ParseResult>(outcome)
        assertEquals(listOf("a.txt", "b.txt"), result.arguments["source"])
        assertEquals("/dest", result.arguments["dest"])
    }

    @Test
    fun returnsHelpAndVersionResults() {
        val spec = """
            {
              "cli_builder_spec_version": "1.0",
              "name": "tool",
              "description": "A tool",
              "version": "2.0.0",
              "flags": [
                {
                  "id": "verbose",
                  "long": "verbose",
                  "description": "Verbose output",
                  "type": "boolean"
                }
              ],
              "arguments": [],
              "commands": []
            }
        """.trimIndent()

        val help = parse(spec, "tool", "--help")
        val version = parse(spec, "tool", "--version")

        val helpResult = assertIs<HelpResult>(help)
        assertTrue(helpResult.text.contains("USAGE"))
        val versionResult = assertIs<VersionResult>(version)
        assertEquals("2.0.0", versionResult.version)
    }

    @Test
    fun inheritsParentCommandFlagsForNestedCommands() {
        val outcome = parse(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "git",
              "description": "Version control",
              "flags": [],
              "arguments": [],
              "commands": [
                {
                  "id": "remote",
                  "name": "remote",
                  "description": "Manage remotes",
                  "flags": [
                    {
                      "id": "dry-run",
                      "long": "dry-run",
                      "description": "Do not apply changes",
                      "type": "boolean"
                    }
                  ],
                  "arguments": [],
                  "commands": [
                    {
                      "id": "add",
                      "name": "add",
                      "description": "Add a remote",
                      "flags": [],
                      "arguments": [
                        {
                          "id": "name",
                          "name": "NAME",
                          "description": "Remote name",
                          "type": "string"
                        }
                      ],
                      "commands": []
                    }
                  ]
                }
              ]
            }
            """.trimIndent(),
            "git",
            "remote",
            "add",
            "--dry-run",
            "origin",
        )

        val result = assertIs<ParseResult>(outcome)
        assertEquals(true, result.flags["dry-run"])
        assertEquals("origin", result.arguments["name"])
        assertEquals(listOf("git", "remote", "add"), result.commandPath)
    }

    @Test
    fun reportsConflictingFlags() {
        try {
            parse(
                """
                {
                  "cli_builder_spec_version": "1.0",
                  "name": "echo",
                  "description": "Display text",
                  "flags": [
                    {
                      "id": "enable-escapes",
                      "short": "e",
                      "long": "enable-escapes",
                      "description": "Enable escapes",
                      "type": "boolean",
                      "conflicts_with": ["disable-escapes"]
                    },
                    {
                      "id": "disable-escapes",
                      "short": "E",
                      "long": "disable-escapes",
                      "description": "Disable escapes",
                      "type": "boolean",
                      "conflicts_with": ["enable-escapes"]
                    }
                  ],
                  "arguments": [],
                  "commands": []
                }
                """.trimIndent(),
                "echo",
                "-e",
                "-E",
            )
            fail("Expected ParseErrors")
        } catch (error: ParseErrors) {
            assertEquals(1, error.errors.size)
            assertEquals("conflicting_flags", error.errors.first().errorType)
        }
    }

    private fun parse(specJson: String, vararg argv: String): ParseOutcome {
        val specPath = writeSpec(specJson)
        return Parser(specPath, argv.toList()).parse()
    }

    private fun writeSpec(specJson: String): Path {
        val path = Files.createTempFile("cli-builder-", ".json")
        Files.writeString(path, specJson)
        return path
    }
}
