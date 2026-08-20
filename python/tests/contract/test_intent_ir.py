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


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    "name",
    ["echo_v0.json", "simple_rest_api.json"],
)
def test_fixtures_match_schema_and_models(name: str, schema: dict) -> None:
    payload = json.loads((FIXTURE_DIR / name).read_text(encoding="utf-8"))
    Draft202012Validator(schema).validate(payload)
    ir = IntentIR.model_validate(payload)
    ir.validate_structure()
    assert ir.version == 1
    assert ir.tasks


def test_echo_v0_is_the_slice_intent() -> None:
    payload = json.loads((FIXTURE_DIR / "echo_v0.json").read_text(encoding="utf-8"))
    ir = IntentIR.model_validate(payload)
    assert ir.tasks[0].capabilities == ["cap.echo"]
    assert ir.requires_approval is True
    assert ir.risk_score == 0.0
