"""Write composable CIL artifacts into minimal PE/CLI assemblies."""

from cli_assembly_writer.writer import (
    CLIAssemblyArtifact,
    CLIAssemblyConfig,
    CLIAssemblyWriter,
    CLIAssemblyWriterError,
    write_cli_assembly,
)

__all__ = [
    "CLIAssemblyArtifact",
    "CLIAssemblyConfig",
    "CLIAssemblyWriter",
    "CLIAssemblyWriterError",
    "write_cli_assembly",
]
