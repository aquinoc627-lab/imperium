"""Contract tests: fixtures must match schema + Pydantic IR."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from imperium_intent.models import IntentIR

REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA_PATH = REPO_ROOT / "schemas" / "intent_ir.schema.json"
FIXTURE_DIR = REPO_ROOT / "tests" / "contract" / "intent_ir"

# (fixture, expected protocol version)
FIXTURES = [
    ("echo_v0.json", 1),
    ("simple_rest_api.json", 1),
    ("write_v2.json", 2),
]


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    "name,expected_version",
    FIXTURES,
)
def test_fixtures_match_schema_and_models(
    name: str, expected_version: int, schema: dict
) -> None:
    payload = json.loads((FIXTURE_DIR / name).read_text(encoding="utf-8"))
    Draft202012Validator(schema).validate(payload)
    ir = IntentIR.model_validate(payload)
    ir.validate_structure()
    assert ir.version == expected_version
    assert ir.tasks


def test_echo_v0_is_the_slice_intent() -> None:
    payload = json.loads((FIXTURE_DIR / "echo_v0.json").read_text(encoding="utf-8"))
    ir = IntentIR.model_validate(payload)
    assert ir.tasks[0].capabilities == ["cap.echo"]
    assert ir.requires_approval is True
    assert ir.risk_score == 0.0


def test_write_v2_declares_effects() -> None:
    payload = json.loads((FIXTURE_DIR / "write_v2.json").read_text(encoding="utf-8"))
    ir = IntentIR.model_validate(payload)
    assert ir.version == 2
    effects = ir.tasks[0].effects
    assert len(effects) == 1
    assert effects[0].type == "write"
    assert effects[0].path == "scratch/notes.txt"
    assert effects[0].size_bytes == 5


def test_unknown_version_is_rejected() -> None:
    payload = json.loads((FIXTURE_DIR / "write_v2.json").read_text(encoding="utf-8"))
    payload["version"] = 3
    ir = IntentIR.model_validate(payload)
    with pytest.raises(ValueError, match="Version mismatch"):
        ir.validate_structure()
