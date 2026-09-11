//! PyO3 extension module wrapping `hl7pet-core` for Python (spec
//! 6000-python-bindings-automation). Every function here scans/parses/
//! executes against `hl7pet-core` directly (no eager hierarchy build, no
//! intermediate object model — Constitution Principle II) and converts to
//! a Python-owned value exactly once, at this outer boundary. Absence is
//! always `None` (Constitution Principle III); the four structural
//! failures `hl7pet-core` itself defines are mapped to typed exceptions in
//! `errors` — see contracts/python-api.md for the full behavioral contract.

mod errors;
mod located_value;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use located_value::LocatedValue;

/// Shared scan+parse pipeline every entry point below starts with.
fn compile<'m, 'p>(
    message: &'m str,
    path: &'p str,
) -> PyResult<(hl7pet_core::ScanResult<'m>, hl7pet_core::CompiledPath<'p>)> {
    let scan = hl7pet_core::scan(message).map_err(errors::scan_error)?;
    let compiled = hl7pet_core::parse(path).map_err(errors::parse_error)?;
    Ok((scan, compiled))
}

fn reject_hierarchy(compiled: &hl7pet_core::CompiledPath<'_>, method: &str) -> PyResult<()> {
    if compiled.child.is_some() {
        return Err(errors::Hl7PathError::new_err(format!(
            "{method} does not support hierarchy PATHs (the '->' operator); use get_value_hierarchy with a profile"
        )));
    }
    Ok(())
}

fn owned_rows(rows: Vec<Vec<std::borrow::Cow<'_, str>>>) -> Option<Vec<Vec<String>>> {
    if rows.is_empty() {
        None
    } else {
        Some(
            rows.into_iter()
                .map(|inner| inner.into_iter().map(|s| s.into_owned()).collect())
                .collect(),
        )
    }
}

/// Scala `getValue(msg, path)` counterpart (FR-003). Non-hierarchy PATHs
/// only — a hierarchy PATH raises `Hl7PathError` (no profile to navigate
/// with here; see `get_value_hierarchy`). `None` when nothing matches;
/// never raises for "no match" (FR-005).
#[pyfunction]
fn get_value(message: &str, path: &str) -> PyResult<Option<Vec<Vec<String>>>> {
    let (scan, compiled) = compile(message, path)?;
    reject_hierarchy(&compiled, "get_value")?;
    let result = hl7pet_core::execute(&scan, &compiled).map_err(errors::query_error)?;
    Ok(owned_rows(result))
}

/// Hierarchy counterpart of `get_value` (spec 008's `execute_hierarchy`).
/// `profile` is the same `segmentDefinition` JSON shape `hl7pet-core`
/// already parses, passed as a Python dict; re-serialized via the stdlib
/// `json` module rather than a second FFI conversion crate (research.md).
///
/// `build_hierarchy=False` mirrors the Scala engine's static-mode toggle
/// (fixtures `flags.buildHierarchy`) by passing `None` as the profile to
/// `execute_hierarchy` instead of parsing/using `profile` at all — the
/// same "don't consult a profile" behavior `crates/core/tests/
/// hierarchy_vectors.rs` already exercises directly against the core.
#[pyfunction]
#[pyo3(signature = (message, path, profile, build_hierarchy=true))]
fn get_value_hierarchy(
    py: Python<'_>,
    message: &str,
    path: &str,
    profile: &Bound<'_, PyAny>,
    build_hierarchy: bool,
) -> PyResult<Option<Vec<Vec<String>>>> {
    let hierarchy_profile = if build_hierarchy {
        let json_mod = py.import("json")?;
        let profile_json: String = json_mod.call_method1("dumps", (profile,))?.extract()?;
        Some(
            hl7pet_core::HierarchyProfile::from_json(&profile_json)
                .map_err(errors::profile_error)?,
        )
    } else {
        None
    };

    let (scan, compiled) = compile(message, path)?;
    let result = hl7pet_core::execute_hierarchy(&scan, &compiled, hierarchy_profile.as_ref())
        .map_err(errors::query_error)?;
    Ok(owned_rows(result))
}

/// Scala `getFirstValue(msg, path)` counterpart (FR-003). `None` when
/// nothing matches. Non-hierarchy PATHs only, same restriction as
/// `get_value`.
#[pyfunction]
fn get_first_value(message: &str, path: &str) -> PyResult<Option<String>> {
    let (scan, compiled) = compile(message, path)?;
    reject_hierarchy(&compiled, "get_first_value")?;
    let result = hl7pet_core::execute(&scan, &compiled).map_err(errors::query_error)?;
    Ok(result.first().and_then(|reps| reps.first()).map(|s| s.to_string()))
}

/// Spec 1000 `execute_located` counterpart. Non-hierarchy PATHs only, the
/// same scope `hl7pet-core` itself enforces for this method (FR-009).
#[pyfunction]
fn get_value_located(message: &str, path: &str) -> PyResult<Option<Vec<Vec<LocatedValue>>>> {
    let (scan, compiled) = compile(message, path)?;
    reject_hierarchy(&compiled, "get_value_located")?;
    let result = hl7pet_core::execute_located(&scan, &compiled).map_err(errors::query_error)?;
    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            result
                .into_iter()
                .map(|inner| inner.into_iter().map(LocatedValue::from).collect())
                .collect(),
        ))
    }
}

/// Spec 1000 `first_located` counterpart.
#[pyfunction]
fn get_first_value_located(message: &str, path: &str) -> PyResult<Option<LocatedValue>> {
    let (scan, compiled) = compile(message, path)?;
    reject_hierarchy(&compiled, "get_first_value_located")?;
    let result = hl7pet_core::first_located(&scan, &compiled).map_err(errors::query_error)?;
    Ok(result.map(LocatedValue::from))
}

/// Batched extraction (FR-004): one scan of `message`, then one `execute`
/// per path in `paths`, in order — amortizes per-call FFI overhead for hot
/// loops. A path that fails to parse or is a hierarchy PATH raises
/// immediately, aborting the whole batch (research.md #4) — the same
/// error contract as the single-call methods, not a per-element marker.
#[pyfunction]
fn get_values(message: &str, paths: Vec<String>) -> PyResult<Vec<Option<Vec<Vec<String>>>>> {
    let scan = hl7pet_core::scan(message).map_err(errors::scan_error)?;
    let mut out = Vec::with_capacity(paths.len());
    for path in &paths {
        let compiled = hl7pet_core::parse(path).map_err(errors::parse_error)?;
        reject_hierarchy(&compiled, "get_values")?;
        let result = hl7pet_core::execute(&scan, &compiled).map_err(errors::query_error)?;
        out.push(owned_rows(result));
    }
    Ok(out)
}

#[pymodule]
fn _hl7pet(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<LocatedValue>()?;
    m.add_function(wrap_pyfunction!(get_value, m)?)?;
    m.add_function(wrap_pyfunction!(get_value_hierarchy, m)?)?;
    m.add_function(wrap_pyfunction!(get_first_value, m)?)?;
    m.add_function(wrap_pyfunction!(get_value_located, m)?)?;
    m.add_function(wrap_pyfunction!(get_first_value_located, m)?)?;
    m.add_function(wrap_pyfunction!(get_values, m)?)?;
    errors::register(py, m)?;
    Ok(())
}
