"""Pipeline — Orchestrator for the computing stack.

Chains lexer, parser, compiler, and VM into a single execution flow,
capturing traces at every stage for the HTML visualizer.

Usage::

    from pipeline import Pipeline

    result = Pipeline().run("x = 1 + 2")

    # Inspect each stage:
    print(result.lexer_stage.token_count)       # Number of tokens
    print(result.parser_stage.ast_dict)          # JSON-serializable AST
    print(result.compiler_stage.instructions_text)  # Human-readable bytecode
    print(result.vm_stage.final_variables)       # {"x": 3}
"""

from pipeline.orchestrator import (
    CompilerStage,
    LexerStage,
    ParserStage,
    Pipeline,
    PipelineResult,
    VMStage,
    ast_to_dict,
)

__all__ = [
    "CompilerStage",
    "LexerStage",
    "ParserStage",
    "Pipeline",
    "PipelineResult",
    "VMStage",
    "ast_to_dict",
]
