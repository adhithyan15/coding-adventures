import pytest

from beam_vm_simulator import BeamVMSimulator

TAG_U = 0
TAG_I = 1
TAG_A = 2
TAG_X = 3
TAG_Y = 4
TAG_F = 5


def _encode_small(tag: int, value: int) -> bytes:
    return bytes([(value << 4) | tag])


def _chunk(chunk_id: str, payload: bytes) -> bytes:
    padding = b"\x00" * ((4 - (len(payload) % 4)) % 4)
    return (
        chunk_id.encode("ascii")
        + len(payload).to_bytes(4, "big")
        + payload
        + padding
    )


def _build_beam(
    atoms: list[str],
    code: bytes,
    imports: list[tuple[int, int, int]] | None = None,
    exports: list[tuple[int, int, int]] | None = None,
) -> bytes:
    atom_payload = (-len(atoms)).to_bytes(4, "big", signed=True)
    for atom in atoms:
        atom_bytes = atom.encode("utf-8")
        atom_payload += _encode_small(TAG_U, len(atom_bytes)) + atom_bytes

    imports = imports or []
    exports = exports or []
    code_payload = (
        (16).to_bytes(4, "big")
        + (0).to_bytes(4, "big")
        + (184).to_bytes(4, "big")
        + max(1, len(exports)).to_bytes(4, "big")
        + max(1, len(exports)).to_bytes(4, "big")
        + code
    )
    imp_payload = len(imports).to_bytes(4, "big")
    for module_atom, function_atom, arity in imports:
        imp_payload += (
            module_atom.to_bytes(4, "big")
            + function_atom.to_bytes(4, "big")
            + arity.to_bytes(4, "big")
        )
    exp_payload = len(exports).to_bytes(4, "big")
    for function_atom, arity, label in exports:
        exp_payload += (
            function_atom.to_bytes(4, "big")
            + arity.to_bytes(4, "big")
            + label.to_bytes(4, "big")
        )
    chunks = b"".join(
        [
            _chunk("AtU8", atom_payload),
            _chunk("Code", code_payload),
            _chunk("StrT", b""),
            _chunk("ImpT", imp_payload),
            _chunk("ExpT", exp_payload),
        ]
    )
    form_payload = b"BEAM" + chunks
    return b"FOR1" + len(form_payload).to_bytes(4, "big") + form_payload


def _build_addition_beam() -> bytes:
    atoms = ["demo", "main", "erlang", "+"]
    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([64]),
            _encode_small(TAG_I, 1),
            _encode_small(TAG_X, 0),
            bytes([64]),
            _encode_small(TAG_I, 2),
            _encode_small(TAG_X, 1),
            bytes([7]),
            _encode_small(TAG_U, 2),
            _encode_small(TAG_U, 0),
            bytes([19]),
        ]
    )
    return _build_beam(atoms, code, imports=[(3, 4, 2)], exports=[(2, 0, 1)])


def test_simulate_simple_main_function() -> None:
    simulator = BeamVMSimulator()
    result = simulator.execute(_build_addition_beam())
    assert result.ok
    assert result.final_state.module_name == "demo"
    assert result.final_state.x_registers[0] == 3


def test_step_without_load_raises() -> None:
    with pytest.raises(RuntimeError, match="No BEAM module"):
        BeamVMSimulator().step()


def test_step_after_halt_raises() -> None:
    simulator = BeamVMSimulator()
    result = simulator.execute(_build_addition_beam())
    assert result.ok
    with pytest.raises(RuntimeError, match="halted"):
        simulator.step()


def test_max_steps_exceeded_on_infinite_loop() -> None:
    atoms = ["demo", "main"]
    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([61]),
            _encode_small(TAG_F, 1),
        ]
    )
    beam = _build_beam(atoms, code, exports=[(2, 0, 1)])
    result = BeamVMSimulator().execute(beam, max_steps=3)
    assert result.ok is False
    assert result.error == "max_steps (3) exceeded"


def test_call_and_return_to_helper() -> None:
    atoms = ["demo", "main", "helper"]
    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([4]),
            _encode_small(TAG_U, 0),
            _encode_small(TAG_F, 2),
            bytes([19]),
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 3),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 2),
            bytes([64]),
            _encode_small(TAG_I, 7),
            _encode_small(TAG_X, 0),
            bytes([19]),
        ]
    )
    beam = _build_beam(atoms, code, exports=[(2, 0, 1)])
    result = BeamVMSimulator().execute(beam)
    assert result.ok
    assert result.final_state.x_registers[0] == 7


def test_call_only_and_y_register_moves() -> None:
    atoms = ["demo", "main", "helper"]
    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([12]),
            _encode_small(TAG_U, 0),
            _encode_small(TAG_U, 0),
            bytes([16]),
            _encode_small(TAG_U, 0),
            _encode_small(TAG_U, 0),
            bytes([6]),
            _encode_small(TAG_U, 0),
            _encode_small(TAG_F, 2),
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 3),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 2),
            bytes([64]),
            _encode_small(TAG_I, 5),
            _encode_small(TAG_Y, 0),
            bytes([64]),
            _encode_small(TAG_Y, 0),
            _encode_small(TAG_X, 0),
            bytes([18]),
            _encode_small(TAG_U, 0),
            bytes([19]),
        ]
    )
    beam = _build_beam(atoms, code, exports=[(2, 0, 1)])
    result = BeamVMSimulator().execute(beam)
    assert result.ok
    assert result.final_state.x_registers[0] == 5
    assert result.final_state.y_registers[0] == 5


def test_call_ext_only_halts_immediately() -> None:
    atoms = ["demo", "main", "erlang", "+"]
    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([64]),
            _encode_small(TAG_I, 1),
            _encode_small(TAG_X, 0),
            bytes([64]),
            _encode_small(TAG_I, 4),
            _encode_small(TAG_X, 1),
            bytes([78]),
            _encode_small(TAG_U, 2),
            _encode_small(TAG_U, 0),
        ]
    )
    beam = _build_beam(atoms, code, imports=[(3, 4, 2)], exports=[(2, 0, 1)])
    result = BeamVMSimulator().execute(beam)
    assert result.ok
    assert result.final_state.x_registers[0] == 5
