package com.codingadventures.clibuilder;

import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class CliBuilderTest {
    @Test
    void validatesMinimalSpec() throws IOException {
        var path = writeSpec("""
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
                """);

        var result = CliBuilder.validateSpec(path);
        assertTrue(result.valid());
        assertEquals(List.of(), result.errors());
    }

    @Test
    void parsesCountFlagsAndExplicitFlags() throws IOException {
        ParseOutcome outcome = parse("""
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
                """, "myapp", "-vqv");

        assertInstanceOf(ParseResult.class, outcome);
        ParseResult result = (ParseResult) outcome;
        assertEquals(2L, result.flags().get("verbose"));
        assertEquals(Boolean.TRUE, result.flags().get("quiet"));
        assertEquals(List.of("verbose", "quiet", "verbose"), result.explicitFlags());
    }

    @Test
    void supportsDefaultWhenPresentForEnumFlags() throws IOException {
        ParseOutcome outcome = parse("""
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
                """, "myapp", "--color", "file.txt");

        ParseResult result = assertInstanceOf(ParseResult.class, outcome);
        assertEquals("always", result.flags().get("color"));
        assertEquals("file.txt", result.arguments().get("file"));
    }

    @Test
    void resolvesVariadicArgumentsWithLastWins() throws IOException {
        ParseOutcome outcome = parse("""
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
                """, "cp", "a.txt", "b.txt", "/dest");

        ParseResult result = assertInstanceOf(ParseResult.class, outcome);
        assertEquals(List.of("a.txt", "b.txt"), result.arguments().get("source"));
        assertEquals("/dest", result.arguments().get("dest"));
    }

    @Test
    void returnsHelpAndVersionResults() throws IOException {
        String spec = """
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
                """;

        ParseOutcome help = parse(spec, "tool", "--help");
        ParseOutcome version = parse(spec, "tool", "--version");

        HelpResult helpResult = assertInstanceOf(HelpResult.class, help);
        assertTrue(helpResult.text().contains("USAGE"));
        VersionResult versionResult = assertInstanceOf(VersionResult.class, version);
        assertEquals("2.0.0", versionResult.version());
    }

    @Test
    void inheritsParentCommandFlagsForNestedCommands() throws IOException {
        ParseOutcome outcome = parse("""
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
                """, "git", "remote", "add", "--dry-run", "origin");

        ParseResult result = assertInstanceOf(ParseResult.class, outcome);
        assertEquals(Boolean.TRUE, result.flags().get("dry-run"));
        assertEquals("origin", result.arguments().get("name"));
        assertEquals(List.of("git", "remote", "add"), result.commandPath());
    }

    @Test
    void reportsConflictingFlags() throws IOException {
        ParseErrors error = assertThrows(ParseErrors.class, () -> parse("""
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
                """, "echo", "-e", "-E"));

        assertEquals(1, error.errors().size());
        assertEquals("conflicting_flags", error.errors().getFirst().errorType());
    }

    private static ParseOutcome parse(String specJson, String... argv) throws IOException {
        Path specPath = writeSpec(specJson);
        return new Parser(specPath, List.of(argv)).parse();
    }

    private static Path writeSpec(String specJson) throws IOException {
        Path path = Files.createTempFile("cli-builder-", ".json");
        Files.writeString(path, specJson);
        return path;
    }
}
