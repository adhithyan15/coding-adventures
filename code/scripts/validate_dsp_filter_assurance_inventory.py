#!/usr/bin/env python3
"""Validate the pinned AF-04a DSP filter API/evidence inventory.

This intentionally recognizes a small, documented source syntax. It fails
closed on drift instead of guessing that an unrecognized declaration is absent.
"""
from __future__ import annotations

import argparse
import ast
import copy
import json
import re
import subprocess
import sys
from pathlib import Path
from functools import lru_cache
from typing import Any, Callable

PINNED_COMMIT = "cb60dc9820fc069325130193118e75fdba9b96bc"
PINNED_TREE = "134b3571f5833c29c567c93489f3f5c012fba35f"
INVENTORY_PATH = "code/specs/assurance/dsp-filters-inventory-v1.json"

EXPECTED_ROOT_EXPORTS = {
    "rust": {
        "Biquad", "sosfilt", "WindowType", "design_low_pass", "design_high_pass",
        "design_band_pass", "try_design_low_pass", "try_design_high_pass",
        "try_design_band_pass", "butterworth_lowpass", "chebyshev1_lowpass",
        "iir", "fir", "fir_fft", "MAX_SAMPLES", "MAX_TAPS", "MAX_ORDER",
        "FilterError", "design",
    },
    "python": {
        "Biquad", "sos_cascade", "sosfilt", "CheckedBiquad", "checked_sosfilt",
        "WindowType", "design_low_pass", "design_high_pass", "design_band_pass",
        "butterworth_lowpass", "chebyshev1_lowpass", "iir", "fir", "fir_fft",
        "MAX_ORDER", "MAX_SAMPLES", "MAX_TAPS", "FilterError",
        "EmptySignalError", "EmptyKernelError", "InvalidCoefficientError",
    },
    "typescript": {
        "Biquad", "sosfilt", "sosCascade", "sos_cascade", "CheckedBiquad",
        "checkedSosfilt", "MAX_FILTER_TAPS", "MAX_TAPS", "MAX_ORDER",
        "MAX_SIGNAL_LEN", "MAX_SAMPLES", "butterworthBandpass",
        "butterworthHighpass", "butterworthLowpass", "chebyshev1Lowpass",
        "designBandPass", "designHighPass", "designLowPass", "firwin",
        "FilterCoefficients", "FirwinOptions", "EmptyKernelError",
        "EmptySignalError", "FilterError", "FftError", "InvalidCoefficientError",
        "fir", "firFft", "fir_fft", "filtfilt", "iir", "lfilter",
    },
}
EXPECTED_PUBLIC_MODULES = {"design", "iir"}
EXPECTED_PRIVATE_REEXPORTS = {"biquad": {"Biquad", "sosfilt"}}
EXPECTED_EXPORT_ENTRY_KINDS = {
    "rust": ({(name, "value", "type") for name in {"Biquad", "WindowType", "FilterError"}} |
             {(name, "value", "function") for name in {"sosfilt", "design_low_pass", "design_high_pass", "design_band_pass", "try_design_low_pass", "try_design_high_pass", "try_design_band_pass", "butterworth_lowpass", "chebyshev1_lowpass", "iir", "fir", "fir_fft"}} |
             {(name, "value", "constant") for name in {"MAX_SAMPLES", "MAX_TAPS", "MAX_ORDER"}} |
             {("design", "value", "module"), ("iir", "module_path", "public_module")}),
    "python": ({(name, "value_and_type", "type") for name in {"Biquad", "CheckedBiquad", "WindowType", "FilterError", "EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"}} |
               {(name, "value", "function") for name in {"sos_cascade", "sosfilt", "checked_sosfilt", "design_low_pass", "design_high_pass", "design_band_pass", "butterworth_lowpass", "chebyshev1_lowpass", "iir", "fir", "fir_fft"}} |
               {(name, "value", "constant") for name in {"MAX_ORDER", "MAX_SAMPLES", "MAX_TAPS"}}),
    "typescript": ({(name, "value_and_type", "type") for name in {"Biquad", "CheckedBiquad", "EmptyKernelError", "EmptySignalError", "FilterError", "FftError", "InvalidCoefficientError"}} |
                   {(name, "type_only", "interface") for name in {"FilterCoefficients", "FirwinOptions"}} |
                   {(name, "value", "function") for name in {"sosfilt", "sosCascade", "sos_cascade", "checkedSosfilt", "butterworthBandpass", "butterworthHighpass", "butterworthLowpass", "chebyshev1Lowpass", "designBandPass", "designHighPass", "designLowPass", "firwin", "fir", "firFft", "fir_fft", "filtfilt", "iir", "lfilter"}} |
                   {(name, "value", "constant") for name in {"MAX_FILTER_TAPS", "MAX_TAPS", "MAX_ORDER", "MAX_SIGNAL_LEN", "MAX_SAMPLES"}}),
}
EXPECTED_METHODS = {
    "rust": {"Biquad": {"new", "process_sample", "process", "reset"}},
    "python": {
        "Biquad": {"__init__", "process_sample", "process", "reset", "lowpass", "highpass", "bandpass", "notch", "allpass", "peak"},
        "CheckedBiquad": {"__init__", "process_sample", "process", "reset"},
    },
    "typescript": {
        "Biquad": {"constructor", "reset", "processSample", "process", "lowpass", "highpass", "bandpass", "notch", "allpass", "peak"},
        "CheckedBiquad": {"constructor", "processSample", "process", "reset"},
    },
}

EXPECTED_SOURCE_PATHS = {
    "rust": {
        "code/packages/rust/dsp-filters/src/lib.rs", "code/packages/rust/dsp-filters/src/design.rs",
        "code/packages/rust/dsp-filters/src/iir.rs", "code/packages/rust/dsp-filters/src/biquad.rs",
        "code/packages/rust/dsp-filters/Cargo.toml",
    },
    "python": {
        "code/packages/python/dsp-filters/src/dsp_filters/__init__.py", "code/packages/python/dsp-filters/src/dsp_filters/fir.py",
        "code/packages/python/dsp-filters/src/dsp_filters/design.py", "code/packages/python/dsp-filters/src/dsp_filters/iir.py",
        "code/packages/python/dsp-filters/src/dsp_filters/biquad.py", "code/packages/python/dsp-filters/src/dsp_filters/checked_biquad.py",
        "code/packages/python/dsp-filters/pyproject.toml",
    },
    "typescript": {
        "code/packages/typescript/dsp-filters/src/index.ts", "code/packages/typescript/dsp-filters/src/constants.ts",
        "code/packages/typescript/dsp-filters/src/errors.ts", "code/packages/typescript/dsp-filters/src/fir.ts",
        "code/packages/typescript/dsp-filters/src/design.ts", "code/packages/typescript/dsp-filters/src/iir.ts",
        "code/packages/typescript/dsp-filters/src/biquad.ts", "code/packages/typescript/dsp-filters/src/checked-biquad.ts",
        "code/packages/typescript/dsp-filters/package.json", "code/packages/typescript/dsp-fft/src/windows.ts",
    },
}
EXPECTED_TEST_PATHS = {
    "rust": {"code/packages/rust/dsp-filters/tests/biquad_sos.rs", "code/packages/rust/dsp-filters/tests/public_design_api.rs"},
    "python": {
        "code/packages/python/dsp-filters/tests/test_biquad_sos.py", "code/packages/python/dsp-filters/tests/test_checked_biquad.py",
        "code/packages/python/dsp-filters/tests/test_checked_properties.py", "code/packages/python/dsp-filters/tests/test_design.py",
        "code/packages/python/dsp-filters/tests/test_designer_normalization.py", "code/packages/python/dsp-filters/tests/test_fir.py",
        "code/packages/python/dsp-filters/tests/test_hostile_inputs.py", "code/packages/python/dsp-filters/tests/test_iir.py",
        "code/packages/python/dsp-filters/tests/test_pbt.py", "code/packages/python/dsp-filters/tests/test_shared_conformance.py",
    },
    "typescript": {
        "code/packages/typescript/dsp-filters/tests/biquad.test.ts", "code/packages/typescript/dsp-filters/tests/checked-biquad.test.ts",
        "code/packages/typescript/dsp-filters/tests/checked-properties.test.ts", "code/packages/typescript/dsp-filters/tests/design.test.ts",
        "code/packages/typescript/dsp-filters/tests/fir.test.ts", "code/packages/typescript/dsp-filters/tests/hostile_inputs.test.ts",
        "code/packages/typescript/dsp-filters/tests/iir.test.ts", "code/packages/typescript/dsp-filters/tests/shared_conformance.test.ts",
    },
}
EXPECTED_FUNCTION_EXPORTS = {
    "rust": {"sosfilt", "design_low_pass", "design_high_pass", "design_band_pass", "try_design_low_pass", "try_design_high_pass", "try_design_band_pass", "butterworth_lowpass", "chebyshev1_lowpass", "iir", "fir", "fir_fft"},
    "python": {"sos_cascade", "sosfilt", "checked_sosfilt", "design_low_pass", "design_high_pass", "design_band_pass", "butterworth_lowpass", "chebyshev1_lowpass", "iir", "fir", "fir_fft"},
    "typescript": {"sosfilt", "sosCascade", "sos_cascade", "checkedSosfilt", "butterworthBandpass", "butterworthHighpass", "butterworthLowpass", "chebyshev1Lowpass", "designBandPass", "designHighPass", "designLowPass", "firwin", "fir", "firFft", "fir_fft", "filtfilt", "iir", "lfilter"},
}
EXPECTED_PRODUCER_PATHS = {
    "python": {"code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/fuzz.py", "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py"},
    "typescript": {"code/packages/typescript/test-bootstrap/src/fuzz.ts", "code/packages/typescript/test-bootstrap/src/pbt.ts"},
    "rust": {"code/packages/rust/test-bootstrap/src/fuzz.rs", "code/packages/rust/test-bootstrap/src/pbt.rs"},
}
EXPECTED_PRODUCER_CLAIMS = {
    "python": ("seeded 1-in-10 corpus admission lottery, no execution-edge feedback", "exceptions recorded as crashes; callback return values ignored; no measured watchdog or native coverage"),
    "typescript": ("target-supplied CoverageBitmap semantic feedback; not native instrumentation", "runner records pass/crash; declared timeout/OOM outcomes currently dropped from report; no watchdog is implied"),
    "rust": ("target-supplied CoverageBitmap semantic feedback; not native instrumentation", "fuzz reports callback-returned Timeout/Oom; PBT is separate and has no fuzz-outcome accounting"),
}
EXPECTED_ERROR_OPERATION_KEYS = {
    "rust": {"fir", "fir_fft", "iir", "Biquad", "sosfilt", "legacy_design_*", "try_design_*", "butterworth_lowpass", "chebyshev1_lowpass"},
    "python": {"fir", "fir_fft", "iir", "Biquad", "legacy_sos", "CheckedBiquad", "checked_sosfilt", "design_*", "butterworth_lowpass", "chebyshev1_lowpass"},
    "typescript": {"fir", "firFft", "lfilter/iir", "filtfilt", "Biquad", "sosfilt", "CheckedBiquad", "checkedSosfilt", "design*"},
}
EXPECTED_OPERATION_ERRORS = {
    "rust": {
        "fir": ["EmptySignal", "EmptyKernel", "InvalidCoefficient"],
        "fir_fft": ["EmptySignal", "EmptyKernel", "InvalidCoefficient", "Fft"],
        "iir": ["EmptySignal", "EmptyKernel", "InvalidCoefficient"],
        "Biquad": ["InvalidCoefficient"],
        "sosfilt": ["EmptySignal", "InvalidCoefficient"],
        "legacy_design_*": ["panic"], "try_design_*": ["InvalidCoefficient"],
        "butterworth_lowpass": ["panic"], "chebyshev1_lowpass": ["panic"],
    },
    "python": {
        "fir": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"],
        "fir_fft": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError", "underlying FFT exceptions propagate; FftError unused"],
        "iir": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"],
        "Biquad": ["InvalidCoefficientError", "EmptySignalError"],
        "legacy_sos": ["EmptySignalError", "InvalidCoefficientError"],
        "CheckedBiquad": ["EmptySignalError", "InvalidCoefficientError", "caller accessor exceptions propagate"],
        "checked_sosfilt": ["EmptySignalError", "InvalidCoefficientError", "caller accessor exceptions propagate"],
        "design_*": ["InvalidCoefficientError"], "butterworth_lowpass": ["InvalidCoefficientError"],
        "chebyshev1_lowpass": ["InvalidCoefficientError"],
    },
    "typescript": {
        "fir": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"],
        "firFft": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError", "upstream FFT exceptions propagate"],
        "lfilter/iir": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"],
        "filtfilt": ["EmptySignalError", "EmptyKernelError", "InvalidCoefficientError"],
        "Biquad": ["EmptySignalError", "InvalidCoefficientError"],
        "sosfilt": ["EmptySignalError", "InvalidCoefficientError"],
        "CheckedBiquad": ["EmptySignalError", "InvalidCoefficientError"],
        "checkedSosfilt": ["EmptySignalError", "InvalidCoefficientError", "indexed accessor exceptions propagate"],
        "design*": ["InvalidCoefficientError"],
    },
}
EXPECTED_MEMBER_KEYS = {
    "rust": {"Biquad", "WindowType", "FilterError"},
    "python": {"Biquad", "CheckedBiquad", "WindowType"},
    "typescript": {"Biquad", "CheckedBiquad", "FirwinOptions", "FilterCoefficients"},
}
EXPECTED_NOTE_OPERATIONS = {
    "rust": {"fir/fir_fft", "iir", "Biquad", "sosfilt", "design_low_pass/design_high_pass/design_band_pass", "butterworth_lowpass/chebyshev1_lowpass"},
    "python": {"fir/fir_fft", "iir", "Biquad", "sos_cascade/sosfilt", "CheckedBiquad/checked_sosfilt", "design_low_pass/design_high_pass/design_band_pass", "butterworth_lowpass/chebyshev1_lowpass"},
    "typescript": {"fir/firFft/fir_fft", "lfilter/iir", "filtfilt", "Biquad", "sosfilt/sosCascade/sos_cascade", "CheckedBiquad/checkedSosfilt", "designLowPass/designHighPass/designBandPass/firwin", "butterworthLowpass/butterworthHighpass/butterworthBandpass/chebyshev1Lowpass"},
}

EXPECTED_WARRANT_OPS = {
    "fir", "fir_fft", "iir", "design_low_pass", "design_high_pass", "design_band_pass"
}
EXPECTED_ALIASES = {
    "python": {"sosfilt": {"target": "sos_cascade", "argument_adapter": "sos_cascade(signal, sos)", "parameters": ["sos", "signal"]}},
    "typescript": {"sosCascade": "sosfilt", "sos_cascade": "sosfilt", "fir_fft": "firFft"},
    "rust": {},
}

# AF-04a deliberately records only source-configured evidence at this subject.
# These tables are an independently reviewed baseline, not values read back
# from the JSON being validated.
EXPECTED_RUNG_KEYS = {
    (lane, operation) for lane in EXPECTED_ROOT_EXPORTS
    for operation in ("Biquad/sosfilt", "fir/fir_fft/iir/designers")
}
EXPECTED_RUNG_VALUES = {
    ("rust", "Biquad/sosfilt"): ("present", "configured_not_run", "bounded_dyadic_oracles_configured", "not_live", "not_live"),
    ("python", "Biquad/sosfilt"): ("present", "configured_not_run", "bounded_integer_oracles_configured", "not_live", "not_live"),
    ("typescript", "Biquad/sosfilt"): ("present", "configured_not_run", "bounded_integer_oracles_configured", "not_live", "not_live"),
    ("rust", "fir/fir_fft/iir/designers"): ("present", "test_sources_present_run_not_pinned_here", "partial_test_sources", "not_live", "not_live"),
    ("python", "fir/fir_fft/iir/designers"): ("present", "test_sources_present_run_not_pinned_here", "partial_test_sources", "not_live", "not_live"),
    ("typescript", "fir/fir_fft/iir/designers"): ("present", "test_sources_present_run_not_pinned_here", "partial_test_sources", "not_live", "not_live"),
}
EXPECTED_EVIDENCE = {
    ("rust", "Biquad/sosfilt", "public_execution", "code/packages/rust/dsp-filters/tests/biquad_sos.rs", frozenset({
        "shared_fixture_runs_public_biquad_and_sos_exports",
        "checked_biquad_rejects_bad_coefficients_and_is_failure_atomic",
        "checked_caps_and_fma_distinguishing_recurrence_are_pinned",
    })),
    ("rust", "Biquad/SOS VER05 direct/feedback models", "model", "code/packages/rust/dsp-filters/tests/biquad_sos.rs", frozenset({
        "ver05_exact_direct_form_dyadic_property_has_nonzero_budget",
        "ver05_exact_two_feedback_dyadic_property_has_nonzero_budget",
        "ver05_general_f32_block_error_property_preserves_entry_state",
    })),
    ("rust", "Biquad/SOS structured fuzz", "fuzz", "code/packages/rust/dsp-filters/tests/biquad_sos.rs", frozenset({"ver06_structured_raw_bit_fuzz_replays_and_exercises_minimization"})),
    ("python", "Biquad/SOS", "public_execution", "code/packages/python/dsp-filters/tests/test_shared_conformance.py", frozenset({
        "test_shared_fixture_all_biquad_partitions_and_state_semantics", "test_shared_fixture_checked_biquad_all_partitions_and_state_semantics",
        "test_shared_fixture_sosfilt_is_one_shot_and_empty_behavior", "test_seeded_partition_property_samples", "test_seeded_fixture_schema_fuzz_cases",
    })),
    ("python", "checked properties and source mutants", "property_and_mutation", "code/packages/python/dsp-filters/tests/test_checked_properties.py", frozenset({
        "test_ver05_exact_direct_and_feedback_integer_oracles", "test_seeded_raw_binary64_state_and_error_cases",
        "test_ver06_seeded_mutation_fuzz_replays_and_detects_same_target_fault",
    })),
    ("typescript", "Biquad/SOS", "public_execution", "code/packages/typescript/dsp-filters/tests/shared_conformance.test.ts", frozenset({
        "checks whole, fixed chunks, sample calls, reset replay, and all 32,768 partitions",
        "checks 64 seeded partition-property samples", "rejects 128 seeded malformed-fixture fuzz mutations",
    })),
    ("typescript", "checked properties and source mutants", "property_and_mutation", "code/packages/typescript/dsp-filters/tests/checked-properties.test.ts", frozenset({
        "checks 512 exact integer FIR and feedback recurrence cases with domain-preserving shrinkers",
        "runs structured raw-bit fuzz with manual semantic outcome feedback, replay, and fault minimization",
    })),
}
EXPECTED_EVIDENCE_DOMAINS = {
    ("rust", "Biquad/sosfilt", "public_execution"): "shared 16-sample exact dyadic fixture; fixed [3,1,12], samplewise, reset; 32768 contiguous partitions and 524288 Biquad outputs",
    ("rust", "Biquad/SOS VER05 direct/feedback models", "model"): "VER05 512 direct cases seed AF030001; 512 two-feedback cases seed AF030002; 128 general f32 block cases seed AF030003",
    ("rust", "Biquad/SOS structured fuzz", "fuzz"): "VER06 seed AF03F022, 1024 bounded iterations, five structured initial seeds, max input 40 bytes, 11 corpus classes and 11 recorded manual outcome bins; caller bitmap only",
    ("python", "Biquad/SOS", "public_execution"): "shared fixed fixture; 32768 contiguous partitions/524288 outputs; 64 seeded partition samples AF02B001; 128 schema mutations AF02B003",
    ("python", "checked properties and source mutants", "property_and_mutation"): "bounded integer FIR/feedback domains and seeded raw-float/mutation corpora; exact run evidence must pin seed and imported source SHA",
    ("typescript", "Biquad/SOS", "public_execution"): "shared fixed fixture; 32768 contiguous partitions/524288 outputs; 64 seeded partition samples AF02B101; 128 schema mutations AF02B103",
    ("typescript", "checked properties and source mutants", "property_and_mutation"): "bounded integer FIR/feedback domains and seeded raw-float/mutation corpora; source recompiled before any mutation evidence",
}

LANE_SCHEMA_KEYS = {
    "rust": {"package", "precision", "root_exports", "public_modules", "module_exports", "private_module_reexports", "members", "parameter_and_behavior_notes", "source_blobs", "test_sources", "export_targets", "root_export_entries", "member_signatures", "function_signatures", "constant_values", "operation_errors", "inline_unit_test_identities", "test_identities_by_path", "test_identity_status"},
    "python": {"package", "precision", "root_exports", "members", "module_only_symbols", "parameter_and_behavior_notes", "source_blobs", "test_sources", "export_targets", "aliases", "root_export_entries", "member_signatures", "function_signatures", "constant_values", "operation_errors", "test_identities_by_path", "test_identity_status"},
    "typescript": {"package", "precision", "root_exports", "members", "parameter_and_behavior_notes", "source_blobs", "test_sources", "export_targets", "aliases", "root_export_entries", "member_signatures", "function_signatures", "constant_values", "operation_errors", "test_identities_by_path", "test_identity_status"},
}


def exact_keys(value: Any, required: set[str], category: str, label: str) -> None:
    require(isinstance(value, dict) and set(value) == required, category, f"{label} keys differ: expected {sorted(required)}")

EXPECTED_MEMBER_SIGNATURES = {'python': {'Biquad.__init__': '(self,b0:float,b1:float,b2:float,a0:float,a1:float,a2:float)->None',
            'Biquad.allpass': '(cls,center_norm:float,q:float=1)->Biquad',
            'Biquad.bandpass': '(cls,center_norm:float,q:float=1)->Biquad',
            'Biquad.highpass': '(cls,cutoff_norm:float,q:float=1/sqrt(2))->Biquad',
            'Biquad.lowpass': '(cls,cutoff_norm:float,q:float=1/sqrt(2))->Biquad',
            'Biquad.notch': '(cls,center_norm:float,q:float=1)->Biquad',
            'Biquad.peak': '(cls,center_norm:float,gain_db:float,q:float=1)->Biquad',
            'Biquad.process': '(self,signal:Sequence[float])->list[float]',
            'Biquad.process_sample': '(self,x:float)->float',
            'Biquad.reset': '(self)->None',
            'CheckedBiquad.__init__': '(self,b0:object,b1:object,b2:object,a0:object,a1:object,a2:object)->None',
            'CheckedBiquad.process': '(self,signal:Sequence[object])->list[float]',
            'CheckedBiquad.process_sample': '(self,sample:object)->float',
            'CheckedBiquad.reset': '(self)->None'},
 'rust': {'Biquad::new': '(coefficients: [f32;6]) -> Result<Biquad, FilterError>',
          'Biquad::process': '(&mut self, input: &[f32]) -> Result<Vec<f32>, FilterError>',
          'Biquad::process_sample': '(&mut self, input: f32) -> Result<f32, FilterError>',
          'Biquad::reset': '(&mut self) -> ()'},
 'typescript': {'Biquad.allpass': '(centerNorm:number,q:number=1)->Biquad',
                'Biquad.bandpass': '(centerNorm:number,q:number=1)->Biquad',
                'Biquad.constructor': '(b0:number,b1:number,b2:number,a0:number,a1:number,a2:number)->Biquad',
                'Biquad.highpass': '(cutoffNorm:number,q:number=1/Math.SQRT2)->Biquad',
                'Biquad.lowpass': '(cutoffNorm:number,q:number=1/Math.SQRT2)->Biquad',
                'Biquad.notch': '(centerNorm:number,q:number=1)->Biquad',
                'Biquad.peak': '(centerNorm:number,gainDb:number,q:number=1)->Biquad',
                'Biquad.process': '(signal:Float64Array|ArrayLike<number>)->Float64Array',
                'Biquad.processSample': '(x:number)->number',
                'Biquad.reset': '()->void',
                'CheckedBiquad.constructor': '(b0:unknown,b1:unknown,b2:unknown,a0:unknown,a1:unknown,a2:unknown)->CheckedBiquad',
                'CheckedBiquad.process': '(signal:ArrayLike<unknown>)->Float64Array',
                'CheckedBiquad.processSample': '(sample:unknown)->number',
                'CheckedBiquad.reset': '()->void'}}
class ValidationError(Exception):
    def __init__(self, category: str, message: str):
        super().__init__(f"{category}: {message}")
        self.category = category


def require(condition: bool, category: str, message: str) -> None:
    if not condition:
        raise ValidationError(category, message)


def git_text(repo: Path, *args: str) -> str:
    proc = subprocess.run(["git", "-C", str(repo), *args], text=True, capture_output=True)
    if proc.returncode:
        raise ValidationError("source_identity", f"git {' '.join(args)} failed: {proc.stderr.strip()}")
    return proc.stdout


@lru_cache(maxsize=None)
def pinned_blob(repo: Path, path: str) -> tuple[str, str]:
    oid = git_text(repo, "rev-parse", f"{PINNED_COMMIT}:{path}").strip()
    body = git_text(repo, "show", f"{PINNED_COMMIT}:{path}")
    return oid, body


def exported_python_names(source: str) -> tuple[list[str], dict[str, str]]:
    tree = ast.parse(source)
    exports: list[str] | None = None
    export_declarations = 0
    targets: dict[str, str] = {}
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets):
            export_declarations += 1
            exports = ast.literal_eval(node.value)
        elif isinstance(node, ast.ImportFrom):
            if any(alias.name == "*" for alias in node.names):
                raise ValidationError("unsupported_export_syntax", "Python star import at package root")
            module = node.module or ""
            for alias in node.names:
                local = alias.asname or alias.name
                targets[local] = f"{module}.{alias.name}"
        elif isinstance(node, ast.AugAssign) and isinstance(node.target, ast.Name) and node.target.id == "__all__":
            raise ValidationError("unsupported_export_syntax", "Python __all__ mutation syntax is unsupported")
    if exports is None or export_declarations != 1:
        raise ValidationError("unsupported_export_syntax", "Python package root is missing literal __all__")
    if not isinstance(exports, list) or any(not isinstance(x, str) for x in exports):
        raise ValidationError("unsupported_export_syntax", "Python __all__ is not a literal string list")
    return exports, targets


def exported_typescript(source: str) -> tuple[list[str], dict[str, str], dict[str, str]]:
    names: list[str] = []
    targets: dict[str, str] = {}
    kinds: dict[str, str] = {}
    if re.search(r"(?m)^\s*export\s+(?:\*|type\s*\{)", source):
        raise ValidationError("unsupported_export_syntax", "TypeScript export-star/type-block syntax is outside the checked grammar")
    pattern = re.compile(r"export\s*\{([^}]*)\}\s*from\s*['\"]([^'\"]+)['\"]\s*;?", re.S)
    matches = list(pattern.finditer(source))
    stripped = pattern.sub("", source)
    if re.search(r"^\s*export\s*\{", stripped, re.M):
        raise ValidationError("unsupported_export_syntax", "unparsed TypeScript export block")
    if re.search(r"(?m)^\s*export\s+(?:\*|type\s*\{)", stripped):
        raise ValidationError("unsupported_export_syntax", "unparsed TypeScript export-star/type block")
    if not matches:
        raise ValidationError("unsupported_export_syntax", "no TypeScript re-export blocks found")
    for match in matches:
        module = match.group(2)
        for raw in match.group(1).split(","):
            item = raw.strip()
            if not item:
                continue
            is_type_only = bool(re.match(r"^type\s+", item))
            item = re.sub(r"^type\s+", "", item)
            parts = re.split(r"\s+as\s+", item)
            source_name = parts[0].strip()
            local = parts[-1].strip()
            if not re.fullmatch(r"[A-Za-z_$][\w$]*", source_name) or not re.fullmatch(r"[A-Za-z_$][\w$]*", local):
                raise ValidationError("unsupported_export_syntax", f"cannot parse TypeScript export item {raw!r}")
            names.append(local)
            if local in targets:
                raise ValidationError("unsupported_export_syntax", f"duplicate TypeScript package export {local}")
            targets[local] = f"{module}:{source_name}"
            kinds[local] = "type_only" if is_type_only else "value"
    return names, targets, kinds


def validate_typescript_type_exports(kinds: dict[str, str]) -> None:
    require({name for name, kind in kinds.items() if kind == "type_only"} == {"FilterCoefficients", "FirwinOptions"}, "source_completeness", "TypeScript type-only export set differs from root source")


def rust_names(lib: str) -> tuple[set[str], set[str], dict[str, str], dict[str, set[str]]]:
    modules = set(re.findall(r"(?m)^\s*pub\s+mod\s+(\w+)\s*;", lib))
    root_names = set(re.findall(r"(?m)^\s*pub\s+(?:const|enum|struct|fn)\s+(\w+)", lib))
    targets: dict[str, str] = {}
    reexports: dict[str, set[str]] = {}
    use_pattern = re.compile(r"(?m)^\s*pub\s+use\s+([\w:]+)::\{([^}]*)\}\s*;", re.S)
    for match in use_pattern.finditer(lib):
        module = match.group(1).split("::")[-1]
        items = [item.strip() for item in match.group(2).split(",") if item.strip()]
        reexports[module] = set(items)
        for item in items:
            targets[item] = f"crate::{module}::{item}"
            root_names.add(item)
    single_use = re.compile(r"(?m)^\s*pub\s+use\s+([\w:]+)::(\w+)\s*;")
    for match in single_use.finditer(lib):
        path, name = match.group(1), match.group(2)
        targets[name] = f"crate::{path}::{name}"
        root_names.add(name)
    unparsed_uses = use_pattern.sub("", lib)
    unparsed_uses = single_use.sub("", unparsed_uses)
    if re.search(r"(?m)^\s*pub\s+use\b", unparsed_uses):
        raise ValidationError("unsupported_export_syntax", "Rust package-root pub use syntax is unsupported")
    for name in re.findall(r"(?m)^\s*pub\s+(?:const|enum|struct|fn)\s+(\w+)", lib):
        targets[name] = f"crate::{name}"
    for name in ("Biquad", "sosfilt"):
        if name in targets and targets[name] == f"crate::biquad::{name}":
            targets[name] += " (private module reexport)"
    return root_names, modules, targets, reexports


def python_class_info(repo: Path, class_name: str) -> tuple[set[str], set[str], set[str]]:
    root = f"code/packages/python/dsp-filters/src/dsp_filters/"
    for name in ("biquad.py", "checked_biquad.py", "design.py", "fir.py", "iir.py"):
        _, source = pinned_blob(repo, root + name)
        tree = ast.parse(source)
        for node in tree.body:
            if isinstance(node, ast.ClassDef) and node.name == class_name:
                methods = {n.name for n in node.body if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and (not n.name.startswith("_") or n.name == "__init__")}
                slots: set[str] = set()
                enum_members: set[str] = set()
                for child in node.body:
                    if isinstance(child, ast.Assign):
                        if any(isinstance(target, ast.Name) and target.id == "__slots__" for target in child.targets):
                            value = ast.literal_eval(child.value)
                            slots = {slot for slot in (value if isinstance(value, (tuple, list)) else [value]) if isinstance(slot, str)}
                        elif child.lineno < (node.lineno + 20) and isinstance(child.value, ast.Constant):
                            for target in child.targets:
                                if isinstance(target, ast.Name) and target.id.isupper():
                                    enum_members.add(target.id)
                if class_name == "WindowType":
                    enum_members = {n.targets[0].id for n in node.body if isinstance(n, ast.Assign) and n.targets and isinstance(n.targets[0], ast.Name)}
                return methods, slots, enum_members
    raise ValidationError("source_completeness", f"Python class {class_name} not found in pinned source")


def typescript_class_members(repo: Path, filename: str, class_name: str) -> tuple[set[str], set[str], set[str]]:
    _, source = pinned_blob(repo, filename)
    match = re.search(rf"export\s+class\s+{re.escape(class_name)}\s*\{{([\s\S]*?)^\}}", source, re.M)
    if not match:
        raise ValidationError("source_completeness", f"TypeScript class {class_name} not found")
    body = match.group(1)
    methods: set[str] = set()
    for line in body.splitlines():
        match_method = re.match(r"^ {2}((?:(?:public|private|protected|static|override|async)\s+)*)(constructor|[A-Za-z_$][\w$]*)\s*\(", line)
        if match_method and "private" not in match_method.group(1) and not match_method.group(2).startswith("_"):
            methods.add(match_method.group(2))
    public_fields = set(re.findall(r"(?m)^\s*readonly\s+([A-Za-z_$][\w$]*)\s*:", body))
    private_fields = set(re.findall(r"(?m)^\s*private\s+(#?[A-Za-z_$][\w$]*)\s*[:=]", body))
    private_fields.update(re.findall(r"(?m)^ {2}(?:(?:private|readonly|protected)\s+)*(#[A-Za-z_$][\w$]*)\s*[:=]", body))
    return methods, public_fields, private_fields


def extract_aliases(repo: Path, lane: str) -> dict[str, Any]:
    if lane == "typescript":
        _, source = pinned_blob(repo, "code/packages/typescript/dsp-filters/src/biquad.ts")
        _, fir_source = pinned_blob(repo, "code/packages/typescript/dsp-filters/src/fir.ts")
        found = dict(re.findall(r"export\s+const\s+(\w+)\s*=\s*(\w+)\s*;", source + "\n" + fir_source))
        return {name: found[name] for name in ("sosCascade", "sos_cascade", "fir_fft") if name in found}
    if lane == "python":
        _, source = pinned_blob(repo, "code/packages/python/dsp-filters/src/dsp_filters/biquad.py")
        tree = ast.parse(source)
        for node in tree.body:
            if isinstance(node, ast.FunctionDef) and node.name == "sosfilt":
                args = [a.arg for a in node.args.args]
                returned = next((n.value for n in node.body if isinstance(n, ast.Return)), None)
                if isinstance(returned, ast.Call) and isinstance(returned.func, ast.Name):
                    return {"sosfilt": {"target": returned.func.id, "argument_adapter": f"{returned.func.id}({', '.join(ast.unparse(a) for a in returned.args)})", "parameters": args}}
        raise ValidationError("alias_target", "Python sosfilt wrapper not recognized")
    return {}


def strip_warrant_comments(source: str) -> str:
    return re.sub(r"(?m)^\s*--.*$", "", source)


def warrant_sections(source: str, keyword: str) -> dict[str, list[str]]:
    pattern = re.compile(rf"(?ms)^  {re.escape(keyword)} ([^:\n]+):\n(.*?)(?=^  (?:input|bound|labels|assume)\b|\Z)")
    return {name: [line.strip() for line in body.splitlines() if line.strip()] for name, body in pattern.findall(source)}


def evidence_artifact_candidates(paths: set[str]) -> set[str]:
    explicit = {
        "code/packages/rust/dsp-filters/evidence.bindings", "code/packages/rust/dsp-filters/tests/generated/warrant_evidence.rs",
        "code/packages/python/dsp-filters/evidence.bindings", "code/packages/python/dsp-filters/tests/warrant_evidence.py",
        "code/packages/typescript/dsp-filters/evidence.bindings", "code/packages/typescript/dsp-filters/tests/warrant-evidence.ts",
    }
    return {path for path in paths if path in explicit or "/evidence/" in path or path.endswith("assurance_report.json") or path.endswith("assurance_report.md")}


def validate_inventory(repo: Path, doc: dict[str, Any], altered_sources: dict[str, str] | None = None, altered_current_oids: dict[str, str] | None = None) -> None:
    exact_keys(doc, {"schema_version", "inventory_id", "subject", "lanes", "warrant_copies", "warrant_projection", "binding_and_report_tree_audit", "producer_sources", "evidence_records", "evidence_rungs", "provenance_policy"}, "schema", "inventory")
    require(doc.get("schema_version") == 1, "schema", "schema_version must be 1")
    require(doc.get("inventory_id") == "AF04A-DSP-FILTERS-API-ASSURANCE-v1", "schema", "inventory_id mismatch")
    subject = doc.get("subject", {})
    exact_keys(subject, {"repository", "commit", "tree", "scope"}, "schema", "subject")
    require(subject.get("commit") == PINNED_COMMIT, "source_identity", "subject commit is not the immutable baseline")
    actual_tree = git_text(repo, "rev-parse", f"{PINNED_COMMIT}^{{tree}}").strip()
    require(subject.get("tree") == PINNED_TREE == actual_tree, "source_identity", "subject tree mismatch")

    lanes = doc.get("lanes")
    require(isinstance(lanes, dict) and set(lanes) == set(EXPECTED_ROOT_EXPORTS), "schema", "lane set mismatch")
    all_blobs: dict[str, str] = {}
    for lane, data in lanes.items():
        exact_keys(data, LANE_SCHEMA_KEYS[lane], "schema", f"{lane} lane")
        require(isinstance(data, dict), "schema", f"lane {lane} must be an object")
        require(isinstance(data.get("parameter_and_behavior_notes"), list) and data["parameter_and_behavior_notes"], "source_completeness", f"{lane} parameter/behavior notes missing")
        require(set(data.get("members", {})) == EXPECTED_MEMBER_KEYS[lane], "source_completeness", f"{lane} member/type inventory incomplete")
        exports = data.get("root_exports")
        require(isinstance(exports, list) and len(exports) == len(set(exports)), "schema", f"{lane} root_exports must be unique strings")
        require(set(exports) == EXPECTED_ROOT_EXPORTS[lane], "source_completeness", f"{lane} package-root export set differs from independent baseline")
        require(data.get("export_targets", {}) and set(data["export_targets"]) == set(exports), "alias_target", f"{lane} source-qualified export target set mismatch")
        for mapping_key, expected_paths in (("source_blobs", EXPECTED_SOURCE_PATHS[lane]), ("test_sources", EXPECTED_TEST_PATHS[lane])):
            files = data.get(mapping_key)
            require(isinstance(files, dict) and set(files) == expected_paths, "source_completeness", f"{lane}.{mapping_key} path set differs from independent inventory boundary")
            for path, oid in files.items():
                require(isinstance(path, str) and isinstance(oid, str), "schema", f"invalid {lane}.{mapping_key} entry")
                previous = all_blobs.get(path)
                require(previous is None or previous == oid, "source_identity", f"conflicting pins for {path}")
                all_blobs[path] = oid
        targets = data["export_targets"]
        for name, target in targets.items():
            require(isinstance(target, str) and target, "alias_target", f"{lane} export {name} has no qualified target")
        entries = data.get("root_export_entries")
        require(isinstance(entries, list) and entries, "schema", f"{lane} root_export_entries missing")
        entry_pairs = {(entry.get("name"), entry.get("namespace"), entry.get("kind")) for entry in entries if isinstance(entry, dict)}
        require(all(isinstance(entry, dict) and entry.get("name") and entry.get("namespace") and entry.get("kind") for entry in entries), "schema", f"{lane} export entry missing namespace/kind")
        require({name for name, _, _ in entry_pairs} == set(exports), "source_completeness", f"{lane} typed export entries do not cover root exports")
        require(entry_pairs == EXPECTED_EXPORT_ENTRY_KINDS[lane], "source_completeness", f"{lane} export namespace/kind classification differs from the reviewed source census")
        if lane == "rust":
            require(("iir", "value", "function") in entry_pairs and ("iir", "module_path", "public_module") in entry_pairs and ("design", "value", "module") in entry_pairs, "source_completeness", "Rust iir module/value namespace collision or design module lost")
        if lane == "typescript":
            require({(name, namespace, kind) for name, namespace, kind in entry_pairs if name in {"FilterCoefficients", "FirwinOptions"}} == {("FilterCoefficients", "type_only", "interface"), ("FirwinOptions", "type_only", "interface")}, "source_completeness", "TypeScript type-only exports were not kept separate from runtime values")
        signatures = data.get("member_signatures", {})
        separator = "::" if lane == "rust" else "."
        expected_signature_keys = {f"{cls}{separator}{method}" for cls, methods in EXPECTED_METHODS[lane].items() for method in methods}
        require(set(signatures) == expected_signature_keys and all(isinstance(sig, str) and sig for sig in signatures.values()), "source_completeness", f"{lane} public method signature inventory incomplete")
        require(signatures == EXPECTED_MEMBER_SIGNATURES[lane], "source_completeness", f"{lane} public method signatures differ from the pinned source census")
        function_signatures = data.get("function_signatures")
        require(isinstance(function_signatures, dict) and set(function_signatures) == EXPECTED_FUNCTION_EXPORTS[lane], "source_completeness", f"{lane} exported function signature set incomplete")
        require(data.get("constant_values") == source_constant_values(repo, lane), "source_completeness", f"{lane} exported cap constants differ from pinned source")
        validate_parameter_notes(lane, data["parameter_and_behavior_notes"], data["constant_values"])
        errors = data.get("operation_errors")
        require(isinstance(errors, dict) and set(errors) == EXPECTED_ERROR_OPERATION_KEYS[lane] and errors == EXPECTED_OPERATION_ERRORS[lane], "source_completeness", f"{lane} operation error map differs from the reviewed source census")
        if lane == "rust":
            require(set(data["members"]["Biquad"]) == {"methods", "public_fields", "private_fields"} and set(data["members"]["WindowType"]) == {"variants"} and set(data["members"]["FilterError"]) == {"variants"}, "schema", "Rust member detail shape differs")
        elif lane == "python":
            require(set(data["members"]["Biquad"]) == {"methods", "public_fields", "legacy_prefix_commit"} and set(data["members"]["CheckedBiquad"]) == {"methods", "public_fields", "failure_atomic_for_filter_owned_state", "private_fields"} and set(data["members"]["WindowType"]) == {"variants"}, "schema", "Python member detail shape differs")
            require(data["members"]["Biquad"]["legacy_prefix_commit"] is True and data["members"]["CheckedBiquad"]["failure_atomic_for_filter_owned_state"] is True, "source_completeness", "Python legacy/checked state disposition missing")
        else:
            require(set(data["members"]["Biquad"]) == {"methods", "public_fields", "typescript_private_fields", "runtime_privacy", "runtime_readonly_note"} and set(data["members"]["CheckedBiquad"]) == {"methods", "public_fields", "runtime_private_fields"} and set(data["members"]["FirwinOptions"]) == {"fields"} and set(data["members"]["FilterCoefficients"]) == {"fields"}, "schema", "TypeScript member detail shape differs")

    for path, expected_oid in all_blobs.items():
        actual_oid, source = pinned_blob(repo, path)
        supplied = (altered_sources or {}).get(path, source)
        require(actual_oid == expected_oid, "source_identity", f"pinned blob differs for {path}")
        require(supplied == source, "source_identity", f"loaded source identity differs for {path}")
        current_oid = (altered_current_oids or {}).get(path, git_text(repo, "rev-parse", f"HEAD:{path}").strip())
        require(current_oid == expected_oid, "source_drift", f"current checkout differs from the pinned audit subject at {path}; refresh the reviewed subject/inventory")

    producers = doc.get("producer_sources", [])
    require({row.get("lane") for row in producers} == set(EXPECTED_PRODUCER_PATHS) and len(producers) == 3, "source_completeness", "producer lane inventory incomplete")
    for row in producers:
        lane = row["lane"]
        require((row.get("feedback"), row.get("outcome_accounting")) == EXPECTED_PRODUCER_CLAIMS[lane], "source_completeness", f"{lane} producer capability/disposition claim differs from source audit")
        require({row.get("fuzz_path"), row.get("pbt_path")} == EXPECTED_PRODUCER_PATHS[lane], "source_completeness", f"{lane} producer paths incomplete")
        for path_key, oid_key in (("fuzz_path", "fuzz_blob_oid"), ("pbt_path", "pbt_blob_oid")):
            actual_oid, _ = pinned_blob(repo, row[path_key])
            require(actual_oid == row.get(oid_key), "source_identity", f"{lane} producer blob mismatch at {row[path_key]}")
            current_oid = (altered_current_oids or {}).get(row[path_key], git_text(repo, "rev-parse", f"HEAD:{row[path_key]}").strip())
            require(current_oid == row.get(oid_key), "source_drift", f"current producer differs from the pinned audit subject at {row[path_key]}")

    pyroot = "code/packages/python/dsp-filters/src/dsp_filters/__init__.py"
    py_exports, py_targets = exported_python_names((altered_sources or {}).get(pyroot, pinned_blob(repo, pyroot)[1]))
    py = lanes["python"]
    require(set(py_exports) == set(py["root_exports"]) and len(py_exports) == len(set(py_exports)), "source_completeness", "Python __all__ differs from inventory")
    require({key: py_targets.get(key, "") for key in py["root_exports"]} == py["export_targets"], "alias_target", "Python root import targets differ")
    require(python_public_function_signatures(repo, py_targets) == py["function_signatures"], "source_completeness", "Python exported function signatures differ from pinned source")
    require(py.get("aliases") == EXPECTED_ALIASES["python"], "alias_target", "Python alias target/argument adapter differs from baseline")
    for cls in ("Biquad", "CheckedBiquad"):
        methods, slots, _ = python_class_info(repo, cls)
        member = py["members"].get(cls, {})
        require(methods == set(member.get("methods", [])) == EXPECTED_METHODS["python"][cls], "source_completeness", f"Python {cls} method set differs")
        if cls == "Biquad":
            require(slots == set(member.get("public_fields", [])), "source_completeness", f"Python {cls} public field/slot set differs")
        else:
            require(slots == set(member.get("private_fields", [])), "source_completeness", f"Python {cls} private slot set differs")
    _, _, py_windows = python_class_info(repo, "WindowType")
    require(py_windows == set(py["members"]["WindowType"]["variants"]), "source_completeness", "Python WindowType values differ")

    tsroot = "code/packages/typescript/dsp-filters/src/index.ts"
    ts_exports, ts_targets, ts_kinds = exported_typescript((altered_sources or {}).get(tsroot, pinned_blob(repo, tsroot)[1]))
    ts = lanes["typescript"]
    require(set(ts_exports) == set(ts["root_exports"]) and len(ts_exports) == len(set(ts_exports)), "source_completeness", "TypeScript index export set differs")
    require(ts_targets == ts["export_targets"], "alias_target", "TypeScript source-qualified export targets differ")
    validate_typescript_type_exports(ts_kinds)
    require(typescript_public_function_signatures(repo, ts_targets, EXPECTED_ALIASES["typescript"]) == ts["function_signatures"], "source_completeness", "TypeScript exported function signatures differ from pinned source")
    require(ts.get("aliases") == EXPECTED_ALIASES["typescript"], "alias_target", "TypeScript alias map differs")
    actual_ts_aliases = extract_aliases(repo, "typescript")
    require(actual_ts_aliases == EXPECTED_ALIASES["typescript"], "alias_target", "TypeScript alias assignment differs from baseline")
    for cls in ("Biquad", "CheckedBiquad"):
        filename = "code/packages/typescript/dsp-filters/src/biquad.ts" if cls == "Biquad" else "code/packages/typescript/dsp-filters/src/checked-biquad.ts"
        methods, public_fields, private_fields = typescript_class_members(repo, filename, cls)
        member = ts["members"].get(cls, {})
        require(methods == set(member.get("methods", [])) == EXPECTED_METHODS["typescript"][cls], "source_completeness", f"TypeScript {cls} method set differs")
        require(public_fields == set(member.get("public_fields", [])), "source_completeness", f"TypeScript {cls} public fields differ")
        wanted_private = set(member.get("typescript_private_fields", member.get("runtime_private_fields", [])))
        require(private_fields == wanted_private, "source_completeness", f"TypeScript {cls} private field set differs")
    for interface in ("FilterCoefficients", "FirwinOptions"):
        _, source = pinned_blob(repo, "code/packages/typescript/dsp-filters/src/design.ts")
        match = re.search(rf"export\s+interface\s+{interface}\s*\{{([^}}]*)\}}", source, re.S)
        require(match is not None, "source_completeness", f"TypeScript interface {interface} missing")
        fields = set(re.findall(r"(?m)^\s*([A-Za-z_]\w*)\??\s*:", match.group(1)))
        require(fields == set(ts["members"][interface]["fields"]), "source_completeness", f"TypeScript interface {interface} fields differ")

    rustlib_path = "code/packages/rust/dsp-filters/src/lib.rs"
    rustlib = (altered_sources or {}).get(rustlib_path, pinned_blob(repo, rustlib_path)[1])
    root_names, modules, rust_targets, reexports = rust_names(rustlib)
    rust = lanes["rust"]
    # Rust has a value/type namespace collision: crate::iir is both the module
    # path and the crate-root re-exported function. Keep module paths explicit.
    rust_expected = set(rust["root_exports"]) - {"design"}
    require(root_names == rust_expected, "source_completeness", "Rust crate-root item set differs")
    require(modules == EXPECTED_PUBLIC_MODULES, "source_completeness", "Rust public module set differs")
    require(reexports == EXPECTED_PRIVATE_REEXPORTS | {"design": set(rust["module_exports"]["design"])}, "source_completeness", "Rust module re-export shape changed")
    require(rust.get("private_module_reexports") == {"biquad": ["Biquad", "sosfilt"]}, "alias_target", "Rust private-module reexports differ")
    rust_targets["design"] = "crate::design public module"
    require(rust["export_targets"] == rust_targets, "alias_target", "Rust source-qualified root target map differs")
    require(rust_public_function_signatures(repo, {path for path in rust["source_blobs"] if path.endswith(".rs")}) == rust["function_signatures"], "source_completeness", "Rust exported function signatures differ from pinned source")
    require(rust.get("export_targets", {}).get("Biquad") == "crate::biquad::Biquad (private module reexport)", "alias_target", "Rust Biquad route changed")
    methods = rust_class_methods(repo)
    require(methods == set(rust["members"]["Biquad"]["methods"]) == EXPECTED_METHODS["rust"]["Biquad"], "source_completeness", "Rust Biquad method set differs")
    require(rust_struct_fields(repo, "Biquad") == set(rust["members"]["Biquad"]["private_fields"]) | set(rust["members"]["Biquad"]["public_fields"]), "source_completeness", "Rust Biquad field set differs")
    require(not rust["members"]["Biquad"]["public_fields"], "source_completeness", "Rust Biquad declares public fields unexpectedly")
    require(rust_enum_variants(repo, "code/packages/rust/dsp-filters/src/design.rs", "WindowType") == set(rust["members"]["WindowType"]["variants"]), "source_completeness", "Rust WindowType variants differ")
    require(rust_enum_variants(repo, "code/packages/rust/dsp-filters/src/lib.rs", "FilterError") == set(rust["members"]["FilterError"]["variants"]), "source_completeness", "Rust FilterError variants differ")
    require(set(re.findall(r"(?m)^\s*pub\s+(?:fn|enum|struct)\s+(\w+)", pinned_blob(repo, "code/packages/rust/dsp-filters/src/design.rs")[1])) == set(rust["module_exports"]["design"]), "source_completeness", "Rust public design-module API differs")
    require(set(re.findall(r"(?m)^\s*pub\s+fn\s+(\w+)", pinned_blob(repo, "code/packages/rust/dsp-filters/src/iir.rs")[1])) == set(rust["module_exports"]["iir"]), "source_completeness", "Rust public iir-module API differs")

    for lane, data in lanes.items():
        identity_map = data.get("test_identities_by_path", {})
        require(set(identity_map) == EXPECTED_TEST_PATHS[lane], "source_completeness", f"{lane} named-test map incomplete")
        require(data.get("test_identity_status") == "source_configured_not_run_by_this_inventory", "evidence_provenance", f"{lane} test identities overstated")
        for path, expected_ids in identity_map.items():
            _, source = pinned_blob(repo, path)
            if lane == "python":
                tree = ast.parse(source)
                actual_ids = [node.name for node in tree.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name.startswith("test_")]
            elif lane == "typescript":
                actual_ids = re.findall(r"\bit\s*\(\s*['\"]([^'\"]+)['\"]", source) + re.findall(r"\btest\s*\(\s*['\"]([^'\"]+)['\"]", source)
            else:
                actual_ids = re.findall(r"#\[test\]\s*fn\s+(\w+)\s*\(", source)
            require(actual_ids == expected_ids, "source_completeness", f"{lane} test identities differ at {path}")
            require(len(expected_ids) > 0, "source_completeness", f"no test identities inventoried at {path}")
        if lane == "rust":
            inline = data.get("inline_unit_test_identities", {})
            inline_paths = {path for path in EXPECTED_SOURCE_PATHS["rust"] if path.endswith(".rs")}
            require(set(inline) == inline_paths, "source_completeness", "Rust inline test source set incomplete")
            for path, ids in inline.items():
                _, source = pinned_blob(repo, path)
                actual_ids = re.findall(r"#\[test\]\s*fn\s+(\w+)\s*\(", source)
                require(actual_ids == ids, "source_completeness", f"Rust inline test identities differ at {path}")

    warrant_paths = {
        "canonical": "code/specs/dsp-filters.warrant",
        "rust": "code/packages/rust/dsp-filters/dsp-filters.warrant",
        "python": "code/packages/python/dsp-filters/dsp-filters.warrant",
        "typescript": "code/packages/typescript/dsp-filters/dsp-filters.warrant",
    }
    warrants = doc.get("warrant_copies", [])
    require(len(warrants) == 4 and {row.get("lane"): row.get("path") for row in warrants} == warrant_paths, "source_completeness", "Warrant copy tree set/lane map differs")
    projection = doc.get("warrant_projection")
    exact_keys(projection, {"declared_operations", "all_arguments", "functional_claims", "examples", "biquad_or_sos_operations", "resource_rows", "operation_signatures_by_lane", "input_rows_by_lane", "label_rows_by_lane", "bound_rows_by_lane"}, "schema", "Warrant projection")
    require(projection.get("declared_operations") == ["fir", "fir_fft", "iir", "design_low_pass", "design_high_pass", "design_band_pass"] and projection.get("all_arguments") == "Bytes or Nat" and projection.get("functional_claims") == 0 and projection.get("examples") == 0 and projection.get("biquad_or_sos_operations") == 0, "warrant_declaration", "Warrant operation/claim projection differs")
    for projection_key in ("operation_signatures_by_lane", "input_rows_by_lane", "label_rows_by_lane", "bound_rows_by_lane"):
        require(set(projection.get(projection_key, {})) == set(warrant_paths), "warrant_declaration", f"Warrant {projection_key} lane set incomplete")
    for row in warrants:
        exact_keys(row, {"path", "blob_oid", "lane"}, "schema", "Warrant copy")
        oid, source = pinned_blob(repo, row["path"])
        require(oid == row.get("blob_oid"), "source_identity", f"Warrant source blob differs at {row['path']}")
        current_oid = (altered_current_oids or {}).get(row["path"], git_text(repo, "rev-parse", f"HEAD:{row['path']}").strip())
        require(current_oid == oid, "source_drift", f"current Warrant differs from the pinned audit subject at {row['path']}")
        parsed = strip_warrant_comments((altered_sources or {}).get(row["path"], source))
        ops = set(re.findall(r"(?m)^\s*op\s+(\w+)\s*\(", parsed))
        require(ops == EXPECTED_WARRANT_OPS, "warrant_declaration", f"Warrant op set differs at {row['path']}")
        declared_full = {name: f"{args.strip()}) -> {ret.strip()}" for name, args, ret in re.findall(r"(?m)^\s*op\s+(\w+)\s*\(([^)]*)\)\s*->\s*([^\n]+)", parsed)}
        expected_declared = {
            "fir": "signal: Bytes, kernel: Bytes) -> Result<Bytes, FilterError>",
            "fir_fft": "signal: Bytes, kernel: Bytes) -> Result<Bytes, FilterError>",
            "iir": "signal: Bytes, b: Bytes, a: Bytes) -> Result<Bytes, FilterError>",
            "design_low_pass": "cutoff_norm: Nat, num_taps: Nat, window: Nat) -> Bytes",
            "design_high_pass": "cutoff_norm: Nat, num_taps: Nat, window: Nat) -> Bytes",
            "design_band_pass": "low_norm: Nat, high_norm: Nat, num_taps: Nat, window: Nat) -> Bytes",
        }
        require(declared_full == expected_declared, "warrant_declaration", f"Warrant argument/return projection differs at {row['path']}")
        lane = row["lane"]
        require(projection["operation_signatures_by_lane"][lane] == declared_full, "warrant_declaration", f"Warrant signature projection is stale at {row['path']}")
        for key, keyword in (("input_rows_by_lane", "input"), ("label_rows_by_lane", "labels"), ("bound_rows_by_lane", "bound")):
            observed = warrant_sections(parsed, keyword)
            require(projection[key][lane] == observed, "warrant_declaration", f"Warrant {keyword} projection differs at {row['path']}")
        claims = len(re.findall(r"(?m)^\s*(?:claim|example)\b", parsed))
        require(claims == 0 and doc["warrant_projection"]["functional_claims"] == 0 and doc["warrant_projection"]["examples"] == 0, "warrant_declaration", "unexpected functional claim/example")

    audit = doc["binding_and_report_tree_audit"]
    exact_keys(audit, {"pinned_package_tree_walked", "rust_evidence_bindings_path", "generated_evidence_harness_path", "assurance_report_path", "native_refinement", "warrant_binding", "host_float_and_state_binding", "note"}, "schema", "tree audit")
    for status_key in ("native_refinement", "warrant_binding", "host_float_and_state_binding"):
        require(audit.get(status_key) == "not_live", "unsupported_status", f"tree audit cannot promote {status_key}")
    tree_paths = set(git_text(repo, "ls-tree", "-r", "--name-only", PINNED_COMMIT, "code/packages/rust/dsp-filters", "code/packages/python/dsp-filters", "code/packages/typescript/dsp-filters").splitlines())
    require(audit.get("pinned_package_tree_walked") is True, "status", "package tree audit is not asserted")
    forbidden = sorted(evidence_artifact_candidates(tree_paths))
    require(not forbidden, "status", f"pinned package tree contains evidence artifacts not represented by the inventory: {forbidden}")
    for status_key in ("rust_evidence_bindings_path", "generated_evidence_harness_path", "assurance_report_path"):
        require(doc["binding_and_report_tree_audit"].get(status_key) == "absent", "status", f"unsupported artifact presence claim: {status_key}")

    expected_aliases = doc.get("lanes", {})
    for lane in expected_aliases:
        require(expected_aliases[lane].get("aliases", EXPECTED_ALIASES[lane]) == EXPECTED_ALIASES[lane], "alias_target", f"{lane} aliases differ from independent baseline")
    evidence_rows = doc.get("evidence_records")
    require(isinstance(evidence_rows, list) and evidence_rows, "evidence_provenance", "evidence record section missing/empty")
    for evidence in evidence_rows:
        require(set(evidence) in ({"lane", "operation", "kind", "status", "test_path", "test_blob_oid", "evidence_domain", "notes", "test_identities"}, {"lane", "operation", "kind", "status", "test_path", "test_blob_oid", "evidence_domain", "test_identities"}), "schema", "evidence record keys differ")
        require(evidence.get("status") == "configured", "unsupported_status", "no authenticated run receipts exist in this inventory; evidence must remain configured")
        require(evidence.get("lane") in EXPECTED_ROOT_EXPORTS and evidence.get("operation") and evidence.get("kind") and evidence.get("evidence_domain"), "evidence_provenance", "evidence record key is incomplete")
        require(evidence.get("test_path") in lanes[evidence["lane"]].get("test_sources", {}), "evidence_provenance", "evidence references an unpinned test source")
        require(evidence.get("test_blob_oid") == lanes[evidence["lane"]]["test_sources"].get(evidence["test_path"]), "evidence_provenance", "test blob does not match evidence subject")
        evkey = (evidence["lane"], evidence["operation"], evidence["kind"], evidence["test_path"], frozenset(evidence["test_identities"]))
        require(evkey in EXPECTED_EVIDENCE, "evidence_provenance", "evidence tuple is not an independently reviewed test/operation mapping")
        require(evidence.get("evidence_domain") == EXPECTED_EVIDENCE_DOMAINS[(evidence["lane"], evidence["operation"], evidence["kind"])], "evidence_provenance", "evidence domain was altered or widened")
    expected_evkeys = {(lane, operation, kind, path, ids, EXPECTED_EVIDENCE_DOMAINS[(lane, operation, kind)]) for lane, operation, kind, path, ids in EXPECTED_EVIDENCE}
    actual_evkeys = {(r["lane"], r["operation"], r["kind"], r["test_path"], frozenset(r["test_identities"]), r["evidence_domain"]) for r in evidence_rows}
    require(len(actual_evkeys) == len(evidence_rows) and actual_evkeys == expected_evkeys, "evidence_provenance", "evidence records are duplicated, missing, or mismapped")
    require(doc.get("provenance_policy", {}).get("evidence_key") == ["subject_commit", "lane", "operation", "kind", "domain"], "evidence_provenance", "evidence key does not bind subject/lane/operation/kind/domain")
    rungs = doc.get("evidence_rungs")
    require(isinstance(rungs, list) and rungs, "evidence_provenance", "evidence rung section missing/empty")
    rung_keys = []
    for rung in rungs:
        exact_keys(rung, {"lane", "operation", "source", "public_execution", "mathematical_model", "warrant_binding", "native_refinement"}, "schema", "evidence rung")
        require(rung.get("lane") in EXPECTED_ROOT_EXPORTS and rung.get("operation"), "evidence_provenance", "rung is not operation/lane qualified")
        rung_keys.append((rung["lane"], rung["operation"]))
        expected_values = EXPECTED_RUNG_VALUES[(rung["lane"], rung["operation"])]
        actual_values = tuple(rung[key] for key in ("source", "public_execution", "mathematical_model", "warrant_binding", "native_refinement"))
        require(actual_values == expected_values, "unsupported_status", f"{rung.get('lane')} {rung.get('operation')} evidence rung differs from admitted baseline")
    require(len(rung_keys) == len(set(rung_keys)) and set(rung_keys) == EXPECTED_RUNG_KEYS, "evidence_provenance", "evidence rung scope is incomplete or duplicated")


def rust_class_methods(repo: Path) -> set[str]:
    _, source = pinned_blob(repo, "code/packages/rust/dsp-filters/src/biquad.rs")
    match = re.search(r"impl\s+Biquad\s*\{([\s\S]*?)^\}", source, re.M)
    if not match:
        raise ValidationError("source_completeness", "Rust impl Biquad block not found")
    return set(re.findall(r"(?m)^\s*pub\s+fn\s+(\w+)\s*\(", match.group(1)))


def rust_struct_fields(repo: Path, name: str) -> set[str]:
    _, source = pinned_blob(repo, "code/packages/rust/dsp-filters/src/biquad.rs")
    match = re.search(rf"pub\s+struct\s+{re.escape(name)}\s*\{{([^}}]*)\}}", source, re.S)
    if not match:
        raise ValidationError("source_completeness", f"Rust struct {name} not found")
    return set(re.findall(r"(?m)^\s*(?:pub\s+)?([A-Za-z_]\w*)\s*:", match.group(1)))


def rust_enum_variants(repo: Path, path: str, name: str) -> set[str]:
    _, source = pinned_blob(repo, path)
    match = re.search(rf"pub\s+enum\s+{re.escape(name)}\s*\{{([^}}]*)\}}", source, re.S)
    if not match:
        raise ValidationError("source_completeness", f"Rust enum {name} not found")
    variants = set()
    enum_body = re.sub(r"(?m)^\s*///.*$", "", match.group(1))
    for item in enum_body.split(","):
        item = item.strip()
        if not item:
            continue
        variant = re.match(r"([A-Za-z_]\w*)(?:\(([^)]*)\))?$", item, re.S)
        if not variant:
            raise ValidationError("unsupported_source_syntax", f"unrecognized Rust {name} variant {item!r}")
        payload = re.sub(r"\s+", "", variant.group(2) or "")
        variants.add(variant.group(1) + (f"({payload})" if variant.group(2) else ""))
    return variants


def source_constant_values(repo: Path, lane: str) -> dict[str, int]:
    if lane == "rust":
        _, source = pinned_blob(repo, "code/packages/rust/dsp-filters/src/lib.rs")
        raw = dict(re.findall(r"(?m)^pub const (MAX_\w+): usize = ([^;]+);", source))
        values = {}
        for name, expression in raw.items():
            shift = re.fullmatch(r"1\s*<<\s*(\d+)", expression.strip())
            if shift:
                values[name] = 1 << int(shift.group(1))
            elif expression.strip().isdigit():
                values[name] = int(expression.strip())
            else:
                raise ValidationError("unsupported_source_syntax", f"unrecognized Rust constant expression {name}={expression}")
        return values
    if lane == "python":
        _, source = pinned_blob(repo, "code/packages/python/dsp-filters/src/dsp_filters/fir.py")
        tree = ast.parse(source)
        values = {}
        for node in tree.body:
            targets = node.targets if isinstance(node, ast.Assign) else [node.target] if isinstance(node, ast.AnnAssign) else []
            for target in targets:
                if isinstance(target, ast.Name) and target.id in {"MAX_SAMPLES", "MAX_TAPS", "MAX_ORDER"}:
                    try:
                        values[target.id] = int(ast.literal_eval(node.value))
                    except (ValueError, TypeError):
                        if isinstance(node.value, ast.BinOp) and isinstance(node.value.op, ast.LShift) and isinstance(node.value.left, ast.Constant) and isinstance(node.value.right, ast.Constant):
                            values[target.id] = int(node.value.left.value) << int(node.value.right.value)
                        else:
                            raise ValidationError("unsupported_source_syntax", f"unrecognized Python constant expression {target.id}")
        return values
    _, source = pinned_blob(repo, "code/packages/typescript/dsp-filters/src/constants.ts")
    raw = dict(re.findall(r"(?m)^export const (MAX_\w+): number = ([^;]+);", source))
    values: dict[str, int] = {}
    for _ in range(len(raw) + 1):
        for name, expression in raw.items():
            expr = expression.strip()
            shift = re.fullmatch(r"1\s*<<\s*(\d+)", expr)
            if shift:
                values[name] = 1 << int(shift.group(1))
            elif expr.isdigit():
                values[name] = int(expr)
            elif expr in values:
                values[name] = values[expr]
    require(set(values) == set(raw), "unsupported_source_syntax", "unresolved TypeScript constant expression")
    return values


def validate_parameter_notes(lane: str, notes: list[dict[str, Any]], constants: dict[str, int]) -> None:
    require({note.get("operation") for note in notes} == EXPECTED_NOTE_OPERATIONS[lane], "source_completeness", f"{lane} operation behavior notes incomplete")
    expected_caps = {
        "signal_samples": constants["MAX_SAMPLES"],
        "output_samples": constants["MAX_SAMPLES"],
        "process_samples": constants["MAX_SAMPLES"],
        "fft_length": constants["MAX_SAMPLES"],
        "kernel_taps": constants["MAX_TAPS"],
        "coefficient_vector_preflight": constants["MAX_TAPS"],
        "polynomial_order": constants["MAX_ORDER"],
        "sections": constants["MAX_ORDER"],
        "sos_sections": constants["MAX_ORDER"],
        "maximum_accepted_coefficient_vector_length": constants["MAX_ORDER"] + 1,
        "designer_taps_max": constants["MAX_TAPS"] - 1,
        "coefficient_count_per_section": 6,
        "designer_taps_must_be_odd": True,
        "cutoff_cycles_per_sample": [0.0, 0.5],
        "padding": "3*max(coefficient lengths) when signal is longer than pad length; padded total <= 2^24",
    }
    for note in notes:
        caps = note.get("caps")
        if caps is None:
            continue
        require(isinstance(caps, dict), "source_completeness", f"{lane} {note.get('operation')} caps must be a map")
        for name, value in caps.items():
            require(name in expected_caps, "unsupported_source_syntax", f"unrecognized cap field {name} in {lane} {note.get('operation')}")
            require(value == expected_caps[name], "source_completeness", f"{lane} {note.get('operation')} cap {name} is not derived from the pinned constants")


def rust_public_function_signatures(repo: Path, source_paths: set[str]) -> dict[str, str]:
    signatures: dict[str, str] = {}
    for path in source_paths:
        _, source = pinned_blob(repo, path)
        for match in re.finditer(r"(?m)^pub\s+fn\s+(\w+)\s*\(", source):
            name = match.group(1)
            opening = source.find("(", match.start())
            depth = 0
            closing = None
            for index in range(opening, len(source)):
                if source[index] == "(":
                    depth += 1
                elif source[index] == ")":
                    depth -= 1
                    if depth == 0:
                        closing = index
                        break
            if closing is None:
                raise ValidationError("unsupported_source_syntax", f"unterminated Rust signature {name} in {path}")
            brace = source.find("{", closing)
            semicolon = source.find(";", closing)
            end = min((value for value in (brace, semicolon) if value >= 0), default=-1)
            if end < 0:
                raise ValidationError("unsupported_source_syntax", f"unterminated Rust declaration {name} in {path}")
            signature = re.sub(r"\s+", " ", source[match.start():end].strip())
            if name in signatures:
                raise ValidationError("source_completeness", f"duplicate public Rust function name {name}")
            signatures[name] = signature
    return signatures


def python_public_function_signatures(repo: Path, root_targets: dict[str, str]) -> dict[str, str]:
    signatures: dict[str, str] = {}
    prefix = "dsp_filters."
    for exported, target in root_targets.items():
        if not target.startswith(prefix):
            continue
        module, name = target[len(prefix):].rsplit(".", 1)
        path = f"code/packages/python/dsp-filters/src/dsp_filters/{module}.py"
        _, source = pinned_blob(repo, path)
        tree = ast.parse(source)
        function = next((node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == name), None)
        if function is None:
            continue
        args = ast.unparse(function.args)
        returns = ast.unparse(function.returns) if function.returns else "None"
        signatures[exported] = f"def {name}({args}) -> {returns}"
    return signatures


def typescript_public_function_signatures(repo: Path, root_targets: dict[str, str], aliases: dict[str, str]) -> dict[str, str]:
    signatures: dict[str, str] = {}
    for exported, target in root_targets.items():
        module, name = target.split(":", 1)
        canonical = aliases.get(exported, name)
        path = f"code/packages/typescript/dsp-filters/src/{module.removeprefix('./').removesuffix('.js')}.ts"
        _, source = pinned_blob(repo, path)
        match = re.search(rf"export\s+function\s+{re.escape(canonical)}\s*\(", source)
        if not match:
            continue
        opening = source.find("(", match.start())
        depth = 0
        closing = None
        for index in range(opening, len(source)):
            if source[index] == "(":
                depth += 1
            elif source[index] == ")":
                depth -= 1
                if depth == 0:
                    closing = index
                    break
        if closing is None:
            raise ValidationError("unsupported_source_syntax", f"unterminated TypeScript signature {canonical}")
        brace = source.find("{", closing)
        if brace < 0:
            raise ValidationError("unsupported_source_syntax", f"unterminated TypeScript declaration {canonical}")
        signature = re.sub(r"\s+", " ", source[match.start():brace].strip())
        signatures[exported] = signature
    return signatures


def self_test(repo: Path, doc: dict[str, Any]) -> tuple[int, int]:
    """Run semantic controls; every injected fault must hit its intended class."""
    # Establish that every subsequent mutation starts from an accepted baseline.
    validate_inventory(repo, doc)
    controls: list[tuple[str, Callable[[dict[str, Any]], None], str]] = []

    def malformed(candidate: dict[str, Any]) -> None:
        candidate["schema_version"] = 99
    controls.append(("malformed schema", malformed, "schema"))

    def remove_export(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["rust"]["root_exports"].remove("fir")
    controls.append(("missing root export", remove_export, "source_completeness"))

    def change_pin(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["rust"]["source_blobs"]["code/packages/rust/dsp-filters/src/lib.rs"] = "0" * 40
    controls.append(("mutated source digest", change_pin, "source_identity"))

    def promote_proof(candidate: dict[str, Any]) -> None:
        candidate["evidence_rungs"][0]["native_refinement"] = "live"
    controls.append(("unsupported proof promotion", promote_proof, "unsupported_status"))

    def remove_method(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["python"]["members"]["Biquad"]["methods"].remove("process_sample")
    controls.append(("missing public method", remove_method, "source_completeness"))

    def remove_field(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["rust"]["members"]["Biquad"]["private_fields"].remove("z1")
    controls.append(("missing private state field", remove_field, "source_completeness"))

    def change_alias(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["typescript"]["aliases"]["sosCascade"] = "Biquad"
    controls.append(("changed alias target", change_alias, "alias_target"))

    def cross_lane(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["python"]["export_targets"]["Biquad"] = "./biquad.js:Biquad"
    controls.append(("cross-lane export target", cross_lane, "alias_target"))

    def mismatched_receipt(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"][0]["test_blob_oid"] = "0" * 40
    controls.append(("unmatched historical receipt", mismatched_receipt, "evidence_provenance"))

    def promote_configured_to_executed(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"][0]["status"] = "executed"
    controls.append(("unreceipted execution promotion", promote_configured_to_executed, "unsupported_status"))

    def unrelated_receipt_does_not_prove_binding(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"][0]["operation"] = "fir"
    controls.append(("unrelated receipt cannot bind op", unrelated_receipt_does_not_prove_binding, "evidence_provenance"))

    def unrelated_valid_evidence_cannot_promote_warrant(candidate: dict[str, Any]) -> None:
        # The accepted Biquad/SOS records remain intact; they cannot promote
        # the separate aggregate FIR/IIR/designer Warrant rung.
        candidate["evidence_rungs"][3]["warrant_binding"] = "live"
    controls.append(("unrelated valid evidence cannot promote Warrant rung", unrelated_valid_evidence_cannot_promote_warrant, "unsupported_status"))

    def erase_evidence(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"] = []
    controls.append(("empty evidence records", erase_evidence, "evidence_provenance"))

    def erase_rungs(candidate: dict[str, Any]) -> None:
        candidate["evidence_rungs"] = []
    controls.append(("empty proof rung coverage", erase_rungs, "evidence_provenance"))

    def erase_producers(candidate: dict[str, Any]) -> None:
        candidate["producer_sources"] = []
    controls.append(("missing producer inventory", erase_producers, "source_completeness"))

    def erase_entries_and_signatures(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["rust"]["root_export_entries"] = []
        candidate["lanes"]["rust"]["member_signatures"] = {}
        candidate["lanes"]["rust"]["operation_errors"] = {}
    controls.append(("missing typed exports signatures and errors", erase_entries_and_signatures, "schema"))

    def promote_tree_proof(candidate: dict[str, Any]) -> None:
        candidate["binding_and_report_tree_audit"]["native_refinement"] = "live"
    controls.append(("tree audit proof promotion", promote_tree_proof, "unsupported_status"))

    def substitute_other_lane_sources(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["python"]["source_blobs"] = copy.deepcopy(candidate["lanes"]["rust"]["source_blobs"])
    controls.append(("cross-lane source path substitution", substitute_other_lane_sources, "source_completeness"))

    def fake_producer_digest(candidate: dict[str, Any]) -> None:
        next(row for row in candidate["producer_sources"] if row["lane"] == "python")["fuzz_blob_oid"] = "0" * 40
    controls.append(("fake producer identity", fake_producer_digest, "source_identity"))

    def retarget_rust_iir(candidate: dict[str, Any]) -> None:
        candidate["lanes"]["rust"]["export_targets"]["iir"] = "crate::iir::fir"
    controls.append(("wrong Rust target", retarget_rust_iir, "alias_target"))

    def duplicate_evidence(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"].append(copy.deepcopy(candidate["evidence_records"][0]))
    controls.append(("duplicate evidence tuple", duplicate_evidence, "evidence_provenance"))

    def add_unreviewed_proof_field(candidate: dict[str, Any]) -> None:
        candidate["full_verification"] = "proved"
    controls.append(("unknown top-level proof field", add_unreviewed_proof_field, "schema"))

    def attach_wrong_named_test(candidate: dict[str, Any]) -> None:
        candidate["evidence_records"][0]["test_identities"] = ["does_not_exist"]
    controls.append(("unreachable named-test evidence", attach_wrong_named_test, "evidence_provenance"))

    for name, mutate, expected_category in controls:
        candidate = copy.deepcopy(doc)
        mutate(candidate)
        try:
            validate_inventory(repo, candidate)
        except ValidationError as error:
            if error.category != expected_category:
                raise ValidationError("negative_control", f"{name} failed as {error.category}, expected {expected_category}: {error}") from error
        else:
            raise ValidationError("negative_control", f"{name} escaped validation")
    drift_path = "code/packages/rust/dsp-filters/src/lib.rs"
    try:
        validate_inventory(repo, doc, altered_current_oids={drift_path: "0" * 40})
    except ValidationError as error:
        if error.category != "source_drift":
            raise ValidationError("negative_control", f"current-source drift failed as {error.category}, expected source_drift") from error
    else:
        raise ValidationError("negative_control", "current-HEAD source drift escaped validation")
    try:
        exported_typescript("export * from './untracked';")
    except ValidationError as error:
        if error.category != "unsupported_export_syntax":
            raise ValidationError("negative_control", f"TS export-star rejected as {error.category}, expected unsupported_export_syntax") from error
    else:
        raise ValidationError("negative_control", "TS export-star syntax escaped checked parser")
    candidates = {
        "code/packages/rust/dsp-filters/evidence.bindings",
        "code/packages/rust/dsp-filters/tests/generated/warrant_evidence.rs",
        "code/packages/python/dsp-filters/evidence.bindings",
        "code/packages/python/dsp-filters/tests/warrant_evidence.py",
        "code/packages/typescript/dsp-filters/evidence.bindings",
        "code/packages/typescript/dsp-filters/tests/warrant-evidence.ts",
    }
    require(evidence_artifact_candidates(candidates) == candidates, "negative_control", "positive binding/harness path controls did not classify all known candidates")
    # Source-side declaration controls verify the fail-closed TS type export
    # discriminator and the comment-aware Warrant operation parser.
    ts_path = "code/packages/typescript/dsp-filters/src/index.ts"
    type_mutation = pinned_blob(repo, ts_path)[1].replace("type FilterCoefficients", "FilterCoefficients", 1)
    try:
        _, _, altered_kinds = exported_typescript(type_mutation)
        validate_typescript_type_exports(altered_kinds)
    except ValidationError as error:
        if error.category != "source_completeness":
            raise ValidationError("negative_control", f"TS type-kind mutation failed as {error.category}") from error
    else:
        raise ValidationError("negative_control", "TS type-kind mutation escaped validation")
    warrant_path = "code/packages/rust/dsp-filters/dsp-filters.warrant"
    warrant_mutation = pinned_blob(repo, warrant_path)[1].replace("op fir(signal: Bytes", "op fir_x(signal: Bytes", 1)
    try:
        validate_inventory(repo, doc, {warrant_path: warrant_mutation})
    except ValidationError as error:
        if error.category != "warrant_declaration":
            raise ValidationError("negative_control", f"Warrant declaration mutation failed as {error.category}") from error
    else:
        raise ValidationError("negative_control", "Warrant declaration mutation escaped validation")
    return len(controls) + 4, 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--inventory", type=Path, default=None)
    parser.add_argument("--self-test", action="store_true", help="run semantic negative controls before validating the committed inventory")
    args = parser.parse_args()
    repo = args.repo_root.resolve()
    inventory_path = args.inventory or repo / INVENTORY_PATH
    try:
        document = json.loads(inventory_path.read_text(encoding="utf-8"))
        control_count, positive_count = self_test(repo, document) if args.self_test else (0, 0)
        validate_inventory(repo, document)
    except (OSError, json.JSONDecodeError, ValidationError) as error:
        print(f"DSP filter assurance inventory: FAIL: {error}", file=sys.stderr)
        return 1
    suffix = f"; {control_count} negative and {positive_count} positive semantic controls passed" if args.self_test else ""
    print(f"DSP filter assurance inventory: PASS: {len(document['lanes'])} lanes, pinned subject {PINNED_COMMIT}{suffix}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
