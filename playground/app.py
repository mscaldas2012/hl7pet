"""Flask app factory for the HL7-PET playground (spec 9000).

Run with: flask --app playground.app run
"""

from flask import Flask, jsonify

from .hl7_playground.routes import bp as extract_bp

MAX_CONTENT_LENGTH = 5 * 1024 * 1024  # research.md #6


def create_app() -> Flask:
    app = Flask(__name__)
    app.config["MAX_CONTENT_LENGTH"] = MAX_CONTENT_LENGTH
    app.register_blueprint(extract_bp)

    @app.errorhandler(413)
    def too_large(_error):
        return (
            jsonify(
                status="too_large",
                message="Request too large: message + profile must be under 5 MB combined.",
            ),
            413,
        )

    return app


app = create_app()
