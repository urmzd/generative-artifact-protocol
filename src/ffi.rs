//! PyO3 bindings for the AAP apply engine.
//!
//! Exposes `resolve_envelope` to Python, which takes an envelope JSON string
//! and an optional base content string, and returns the resolved artifact.

use pyo3::prelude::*;
use std::collections::HashMap;

use crate::aap::Envelope;
use crate::apply;

/// Resolve an AAP envelope against optional base content.
///
/// Args:
///     envelope_json: JSON string of the envelope.
///     base_content: Base artifact content (required for diff/section ops, ignored for full/template).
///
/// Returns:
///     Resolved artifact content as a string.
///
/// Raises:
///     ValueError: If the envelope is malformed or the operation fails.
#[pyfunction]
fn resolve_envelope(envelope_json: &str, base_content: Option<&str>) -> PyResult<String> {
    let envelope: Envelope = serde_json::from_str(envelope_json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("invalid envelope JSON: {e}")))?;

    let mut store = HashMap::new();
    if let Some(base) = base_content {
        store.insert(envelope.id.clone(), base.to_string());
    }

    apply::resolve(&envelope, &store)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("apply failed: {e}")))
}

/// AAP apply engine Python module.
#[pymodule]
fn aap(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(resolve_envelope, m)?)?;
    Ok(())
}
