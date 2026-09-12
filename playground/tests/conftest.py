import pathlib
import sys

import pytest

PLAYGROUND_DIR = pathlib.Path(__file__).resolve().parents[1]
REPO_ROOT = PLAYGROUND_DIR.parent
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(PLAYGROUND_DIR))

from playground.app import create_app  # noqa: E402

FIXTURES = REPO_ROOT / "fixtures"


@pytest.fixture()
def app():
    return create_app()


@pytest.fixture()
def client(app):
    return app.test_client()


@pytest.fixture()
def multi_obx_message():
    return (FIXTURES / "messages" / "multi-obx.hl7").read_text()


@pytest.fixture()
def basic_hierarchy_message():
    return (FIXTURES / "messages" / "basic-hierarchy.hl7").read_text()


@pytest.fixture()
def basic_two_level_profile_bytes():
    return (FIXTURES / "profiles" / "basic-two-level.json").read_bytes()
