"""Contract tests for POST /api/extract against
specs/9000-playground-webapp/contracts/playground-api.md."""

import io
import pathlib

FIXTURES = pathlib.Path(__file__).resolve().parents[2] / "fixtures"


# --- User Story 1 (P1): flat PATH with line numbers ---


def test_non_hierarchy_path_returns_located_results(client, multi_obx_message):
    resp = client.post("/api/extract", data={"message": multi_obx_message, "path": "OBX-5"})

    assert resp.status_code == 200
    body = resp.get_json()
    assert body["status"] == "results"
    assert body["hierarchy"] is False
    assert body["results"] == [
        {"value": "Positive", "line": 4},
        {"value": "Negative", "line": 5},
        {"value": "Equivocal", "line": 6},
    ]


def test_non_matching_path_returns_no_results(client, multi_obx_message):
    resp = client.post("/api/extract", data={"message": multi_obx_message, "path": "ZZZ-1"})

    assert resp.status_code == 200
    assert resp.get_json() == {"status": "no_results"}


# --- User Story 2 (P2): hierarchy PATH with a profile ---


def test_hierarchy_path_without_profile_requires_one(client, basic_hierarchy_message):
    resp = client.post(
        "/api/extract", data={"message": basic_hierarchy_message, "path": "OBR -> OBX-5"}
    )

    assert resp.status_code == 200
    body = resp.get_json()
    assert body["status"] == "profile_required"
    assert "profile" in body["message"].lower()


def test_hierarchy_path_with_profile_returns_values_without_line_numbers(
    client, basic_hierarchy_message, basic_two_level_profile_bytes
):
    resp = client.post(
        "/api/extract",
        data={
            "message": basic_hierarchy_message,
            "path": "OBR -> OBX-5",
            "profile": (io.BytesIO(basic_two_level_profile_bytes), "profile.json"),
        },
    )

    assert resp.status_code == 200
    body = resp.get_json()
    assert body["status"] == "results"
    assert body["hierarchy"] is True
    assert body["results"] == ["POS", "NEG", "POS", "NEG"]
    assert all(isinstance(v, str) for v in body["results"])


# --- User Story 3 (P3): bad input handling ---


def test_invalid_path_syntax_returns_path_error(client, multi_obx_message):
    resp = client.post("/api/extract", data={"message": multi_obx_message, "path": "OBX[[1]-5"})

    assert resp.status_code == 200
    assert resp.get_json()["status"] == "path_error"


def test_message_missing_msh_returns_scan_error(client):
    resp = client.post("/api/extract", data={"message": "OBX|1|ST|foo", "path": "OBX-1"})

    assert resp.status_code == 200
    assert resp.get_json()["status"] == "scan_error"


def test_invalid_profile_file_returns_profile_error_and_app_stays_usable(
    client, basic_hierarchy_message, multi_obx_message
):
    resp = client.post(
        "/api/extract",
        data={
            "message": basic_hierarchy_message,
            "path": "OBR -> OBX-5",
            "profile": (io.BytesIO(b"not json"), "profile.json"),
        },
    )
    assert resp.status_code == 200
    assert resp.get_json()["status"] == "profile_error"

    # A follow-up non-hierarchy request in the same session still works normally.
    resp2 = client.post("/api/extract", data={"message": multi_obx_message, "path": "OBX-5"})
    assert resp2.status_code == 200
    assert resp2.get_json()["status"] == "results"


def test_oversized_request_returns_too_large(app):
    app.config["MAX_CONTENT_LENGTH"] = 10  # shrink the cap just for this test
    client = app.test_client()

    resp = client.post("/api/extract", data={"message": "x" * 100, "path": "OBX-1"})

    assert resp.status_code == 413
    assert resp.get_json()["status"] == "too_large"


# --- Edge Case: non-standard delimiters (FR-012) ---


def test_non_standard_delimiters_report_correct_value_and_line(client):
    message = (FIXTURES / "messages" / "scanner-non-standard-delimiters.hl7").read_text()

    resp = client.post("/api/extract", data={"message": message, "path": "PID-5.1"})

    assert resp.status_code == 200
    body = resp.get_json()
    assert body["status"] == "results"
    assert body["results"] == [
        {"value": "Doe", "line": 2},
        {"value": "Smith", "line": 2},
    ]
