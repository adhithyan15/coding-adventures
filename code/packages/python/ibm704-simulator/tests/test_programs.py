"""End-to-end programs that exercise multi-instruction behavior.

These mirror the kinds of programs FORTRAN I and LISP 1 would have produced.
"""

from __future__ import annotations

from ibm704_simulator import (
    OP_ADD,
    OP_CLA,
    OP_FAD,
    OP_FMP,
    OP_HTR,
    OP_LDQ,
    OP_MPY,
    OP_PAX,
    OP_PDX,
    OP_PXA,
    OP_STO,
    OP_STQ,
    OP_STZ,
    OP_TRA,
    PREFIX_TIX,
    IBM704Simulator,
    encode_type_a,
    encode_type_b,
    fp_to_float,
    float_to_fp,
    make_word,
    pack_program,
)


def _run(sim: IBM704Simulator, words: list[int]) -> None:
    """Run a program by writing words and stepping until halt."""
    for i, w in enumerate(words):
        sim._memory[i] = w  # noqa: SLF001
    while not sim._halted:  # noqa: SLF001
        sim.step()


def test_sum_1_to_5() -> None:
    """Compute 1+2+3+4+5 = 15 using a TIX loop.

    Layout::

        0:  CLA  N        ; AC = 5
        1:  PAX  0,1      ; IRA = 5
        2:  STZ  SUM      ; SUM = 0
        3:  CLA  SUM      ; (loop top) AC = SUM
        4:  ADD  IRA-via-PXA-trick ... no, simpler — load counter via PXA

    Actually the simplest formulation: store the running counter into memory
    via PXA + STO, add it to SUM, then TIX to decrement.
    """
    sim = IBM704Simulator()
    n_addr = 100
    sum_addr = 101
    counter_addr = 102

    program = [
        # 0: CLA N  → AC = 5 (we'll load IRA from this)
        encode_type_b(OP_CLA, 0, n_addr),
        # 1: PAX 0,1 → IRA = 5
        encode_type_b(OP_PAX, 1, 0),
        # 2: STZ SUM
        encode_type_b(OP_STZ, 0, sum_addr),
        # 3 (LOOP): PXA 0,1 → AC = IRA  (the counter value)
        encode_type_b(OP_PXA, 1, 0),
        # 4: STO COUNTER
        encode_type_b(OP_STO, 0, counter_addr),
        # 5: CLA SUM
        encode_type_b(OP_CLA, 0, sum_addr),
        # 6: ADD COUNTER
        encode_type_b(OP_ADD, 0, counter_addr),
        # 7: STO SUM
        encode_type_b(OP_STO, 0, sum_addr),
        # 8: TIX 3,1,1 → IRA -= 1; if IRA > 0 goto LOOP
        encode_type_a(PREFIX_TIX, decrement=1, tag=1, address=3),
        # 9: HTR
        encode_type_b(OP_HTR, 0, 9),
    ]

    sim._memory[n_addr] = make_word(0, 5)  # noqa: SLF001
    _run(sim, program)

    state = sim.get_state()
    # Sum 1+2+3+4+5 = 15. (Loop runs IRA=5,4,3,2,1; sums them.)
    assert state.memory[sum_addr] == make_word(0, 15)
    assert state.halted is True


def test_factorial_5() -> None:
    """Compute 5! = 120 using MPY in a loop.

    Strategy:
    - IRA holds the loop counter (5, 4, 3, 2)
    - MEM[result] holds the running product (start at 1)
    - Each iteration: MQ = result; AC,MQ = MQ * counter; store MQ to result.
    """
    sim = IBM704Simulator()
    n_addr = 100
    result_addr = 101
    counter_addr = 102

    program = [
        # 0: CLA N (=5) → AC = 5
        encode_type_b(OP_CLA, 0, n_addr),
        # 1: PAX 0,1 → IRA = 5
        encode_type_b(OP_PAX, 1, 0),
        # 2: CLA ONE → AC = 1
        encode_type_b(OP_CLA, 0, 103),  # mem[103] = 1
        # 3: STO RESULT
        encode_type_b(OP_STO, 0, result_addr),
        # 4 (LOOP): PXA 0,1 → AC = IRA (the counter)
        encode_type_b(OP_PXA, 1, 0),
        # 5: STO COUNTER (so MPY can use it)
        encode_type_b(OP_STO, 0, counter_addr),
        # 6: LDQ RESULT → MQ = result
        encode_type_b(OP_LDQ, 0, result_addr),
        # 7: MPY COUNTER → AC,MQ = MQ * counter
        encode_type_b(OP_MPY, 0, counter_addr),
        # Wait — MPY result low part is in MQ, but we need it stored as a
        # regular word. Use STQ to write MQ.
        # Actually a 32-bit factorial fits in MQ alone (low 35 bits) so we can
        # simply do STQ.
        # 8: STQ RESULT
        encode_type_b(OP_STQ, 0, result_addr),
        # 9: TIX 4,1,1 → IRA -= 1; if IRA > 0 goto LOOP
        encode_type_a(PREFIX_TIX, decrement=1, tag=1, address=4),
        # 10: HTR
        encode_type_b(OP_HTR, 0, 10),
    ]

    sim._memory[n_addr] = make_word(0, 5)  # noqa: SLF001
    sim._memory[103] = make_word(0, 1)  # ONE  # noqa: SLF001
    _run(sim, program)

    state = sim.get_state()
    # Loop iterates with counter = 5, 4, 3, 2 (TIX skips the last). So we
    # compute 1 * 5 * 4 * 3 * 2 = 120 exactly. Note: TIX falls through
    # when IRA <= 1, so IRA=1 doesn't run an iteration.
    assert state.memory[result_addr] == make_word(0, 120)


def test_lisp_style_car_extraction() -> None:
    """LISP cons cell: car in address field, cdr in decrement field.

    Pull out the car of a cell using PAX → PXA → STO.
    """
    sim = IBM704Simulator()
    cell_addr = 100
    result_addr = 101

    # Encode a cons cell: car=42, cdr=99.
    cell_word = (99 << 18) | 42

    program = [
        # 0: CLA CELL → AC = whole word
        encode_type_b(OP_CLA, 0, cell_addr),
        # 1: PAX 0,1 → IRA = address bits = car (42)
        encode_type_b(OP_PAX, 1, 0),
        # 2: PXA 0,1 → AC = IRA, sign cleared
        encode_type_b(OP_PXA, 1, 0),
        # 3: STO RESULT
        encode_type_b(OP_STO, 0, result_addr),
        # 4: HTR
        encode_type_b(OP_HTR, 0, 4),
    ]
    sim._memory[cell_addr] = cell_word  # noqa: SLF001
    _run(sim, program)
    state = sim.get_state()
    assert state.memory[result_addr] == make_word(0, 42)


def test_lisp_style_cdr_extraction() -> None:
    """Pull out the cdr (decrement field) of a cons cell."""
    sim = IBM704Simulator()
    cell_addr = 100
    result_addr = 101
    cell_word = (99 << 18) | 42  # car=42, cdr=99

    program = [
        # 0: CLA CELL → AC = whole word
        encode_type_b(OP_CLA, 0, cell_addr),
        # 1: PDX 0,1 → IRA = decrement bits = cdr (99)
        encode_type_b(OP_PDX, 1, 0),
        # 2: PXA 0,1 → AC = IRA
        encode_type_b(OP_PXA, 1, 0),
        # 3: STO RESULT
        encode_type_b(OP_STO, 0, result_addr),
        # 4: HTR
        encode_type_b(OP_HTR, 0, 4),
    ]
    sim._memory[cell_addr] = cell_word  # noqa: SLF001
    _run(sim, program)
    state = sim.get_state()
    assert state.memory[result_addr] == make_word(0, 99)


def test_floating_point_polynomial() -> None:
    """Evaluate y = a*x + b in floating-point.

    Uses LDQ + FMP + FAD + STO.
    """
    sim = IBM704Simulator()
    a_addr = 100
    x_addr = 101
    b_addr = 102
    y_addr = 103

    program = [
        # 0: LDQ A → MQ = a
        encode_type_b(OP_LDQ, 0, a_addr),
        # 1: FMP X → AC,MQ = a * x
        encode_type_b(OP_FMP, 0, x_addr),
        # 2: FAD B → AC = a*x + b
        encode_type_b(OP_FAD, 0, b_addr),
        # 3: STO Y
        encode_type_b(OP_STO, 0, y_addr),
        # 4: HTR
        encode_type_b(OP_HTR, 0, 4),
    ]
    sim._memory[a_addr] = float_to_fp(2.0)  # noqa: SLF001
    sim._memory[x_addr] = float_to_fp(3.0)  # noqa: SLF001
    sim._memory[b_addr] = float_to_fp(1.0)  # noqa: SLF001
    _run(sim, program)
    state = sim.get_state()
    assert fp_to_float(state.memory[y_addr]) == 7.0  # 2*3 + 1


def test_execute_uses_packed_byte_input() -> None:
    """Verify the protocol-shaped ``execute(bytes)`` call works.

    The trick: ``execute`` calls ``reset()`` which zeros memory, so any
    constants the program references must live in the byte stream itself.
    We embed a constant at word index 2 and load it from there.
    """
    sim = IBM704Simulator()
    program_bytes = pack_program(
        [
            # 0: CLA 2 — load constant at word 2 into AC
            encode_type_b(OP_CLA, 0, 2),
            # 1: HTR 0 — halt
            encode_type_b(OP_HTR, 0, 0),
            # 2: data word, +123
            make_word(0, 123),
        ]
    )
    result = sim.execute(program_bytes)
    assert result.ok
    assert result.halted is True
    assert result.steps == 2
    assert result.final_state.accumulator_magnitude == 123


def test_max_steps_exceeded() -> None:
    """An infinite loop should bail out at max_steps."""
    sim = IBM704Simulator()
    # TRA 0 — jump to self forever.
    program = pack_program([encode_type_b(OP_TRA, 0, 0)])
    result = sim.execute(program, max_steps=100)
    assert result.ok is False
    assert result.halted is False
    assert "max_steps" in (result.error or "")
    assert result.steps == 100
