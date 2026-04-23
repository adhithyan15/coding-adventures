"""Standardized instruction streams for logic programs.

`logic-engine` already gives us a direct Python API for facts, rules, and
queries. `logic-instructions` adds a new layer above that: logic programs can
be packaged up as an explicit sequence of instructions, validated, and then
lowered into the current engine backend.

That instruction stream becomes the shared contract between:

- today's direct engine execution
- tomorrow's logic VM backend
- future frontends such as a Prolog parser
"""

from logic_instructions.instructions import (
    AssembledInstructionProgram,
    DynamicRelationDefInstruction,
    FactInstruction,
    InstructionOpcode,
    InstructionProgram,
    LogicInstruction,
    QueryInstruction,
    RelationDefInstruction,
    RuleInstruction,
    assemble,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
    run_all_queries,
    run_query,
    validate,
)

__all__ = [
    "__version__",
    "AssembledInstructionProgram",
    "DynamicRelationDefInstruction",
    "FactInstruction",
    "InstructionOpcode",
    "InstructionProgram",
    "LogicInstruction",
    "QueryInstruction",
    "RelationDefInstruction",
    "RuleInstruction",
    "assemble",
    "defdynamic",
    "defrel",
    "fact",
    "instruction_program",
    "query",
    "rule",
    "run_all_queries",
    "run_query",
    "validate",
]

__version__ = "0.2.0"
