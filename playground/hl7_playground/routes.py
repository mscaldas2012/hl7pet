"""HTTP surface: the single page and the single /api/extract endpoint."""

from flask import Blueprint, jsonify, render_template, request

from .extraction import extract

bp = Blueprint("playground", __name__)


@bp.get("/")
def index():
    return render_template("index.html")


@bp.post("/api/extract")
def api_extract():
    message = request.form.get("message", "")
    path = request.form.get("path", "")
    profile_file = request.files.get("profile") or None
    return jsonify(extract(message, path, profile_file))
