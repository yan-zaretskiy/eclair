use eclair::summary::Summary;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use rmp_serde as rmps;
use serde::Serialize;

use std::{mem, path::Path};

#[pyfunction]
/// Load Eclipse's summary data in the MessegaPack format.
fn as_msgpack<'a: 'p, 'p>(py: Python<'p>, input_path: &'a str) -> PyResult<&'p PyBytes> {
    let input_path = Path::new(input_path);

    // Parse SMSPEC & UNSMRY files.
    let summary = Summary::from_path(input_path)
        .map_err(|err| exceptions::IOError::py_err(err.to_string()))?;

    // Serialize summary data to vector in the MessagePack format.
    let mut vec = Vec::with_capacity(128);
    let mut se = rmps::encode::Serializer::new(&mut vec)
        .with_struct_map()
        .with_string_variants();

    summary
        .serialize(&mut se)
        .map_err(|err| exceptions::IOError::py_err(err.to_string()))?;

    // Make Python bytes object from the vector. Forget it so that Rust does not clear its contents.
    let ptr = vec.as_ptr();
    let len = vec.len();
    mem::forget(vec);

    let b = unsafe { PyBytes::from_ptr(py, ptr, len) };
    Ok(b)
}

#[pymodule]
/// A Python module implemented in Rust.
fn eclpy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(as_msgpack))?;

    Ok(())
}
