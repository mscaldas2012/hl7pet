"""Apache Arrow / PyArrow / PySpark integration for hl7pet-core (spec
6002-arrow-integration).

See specs/6002-arrow-integration/contracts/arrow-api.md for the full
contract. `hl7pet_arrow.spark` provides PySpark DataFrame column wiring
(Story 3) and is not imported here, since it requires `pyspark` to be
installed and standalone PyArrow usage should not.
"""

# T013/T019 add extract_value/extract_values re-exports here.
