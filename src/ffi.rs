//! PyO3 bindings for the AAP apply engine.
//!
//! Exposes `resolve_envelope` to Python, which takes an operation envelope
//! JSON string and an optional base artifact envelope JSON string, and
//! returns the resolved artifact as a JSON envelope string.

use pyo3::prelude::*;

use crate::aap::Envelope;
use crate::apply;

/// Resolve an AAP operation against an optional base artifact.
///
/// Args:
///     operation_json: JSON string of the operation envelope.
///     artifact_json: JSON string of the base artifact envelope (a `name:"full"` envelope).
///         Required for diff/section ops, ignored for full/template.
///
/// Returns:
///     Resolved artifact as a JSON envelope string (`name:"full"`).
///
/// Raises:
///     ValueError: If the envelope is malformed or the operation fails.
#[pyfunction]
fn resolve_envelope(operation_json: &str, artifact_json: Option<&str>) -> PyResult<String> {
    let operation: Envelope = serde_json::from_str(operation_json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("invalid operation envelope JSON: {e}")))?;

    let artifact = artifact_json
        .map(|json| {
            serde_json::from_str::<Envelope>(json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("invalid artifact envelope JSON: {e}")))
        })
        .transpose()?;

    let result = apply::apply(artifact.as_ref(), &operation)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("apply failed: {e}")))?;

    serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("serialization failed: {e}")))
}

/// AAP apply engine Python module.
#[pymodule]
fn aap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(resolve_envelope, m)?)?;
    Ok(())
}
