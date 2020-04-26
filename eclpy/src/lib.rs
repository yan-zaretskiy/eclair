use eclair::eclipse_binary::BinFile;
use eclair::eclipse_summary::Summary;
use eclair::errors::FileError;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use rmp_serde as rmps;
use serde::Serialize;

use std::path::Path;

#[pyfunction]
/// Load Eclipse's summary data in the MessegaPack format.
fn as_msgpack(input_path: &str) -> PyResult<PyObject> {
    let input_path = Path::new(input_path);
    // If there is no stem, bail early
    if input_path.file_stem().is_none() {
        return Err(exceptions::ValueError::py_err(
            FileError::InvalidFilePath.to_string(),
        ));
    }

    // we allow either extension or no extension at all
    if let Some(ext) = input_path.extension() {
        let ext = ext.to_str();
        if ext != Some("SMSPEC") && ext != Some("UNSMRY") {
            return Err(exceptions::ValueError::py_err(
                FileError::InvalidFileExt.to_string(),
            ));
        }
    }

    let smspec = BinFile::new(input_path.with_extension("SMSPEC"))
        .map_err(|err| exceptions::IOError::py_err(err.to_string()))?;
    let unsmry = BinFile::new(input_path.with_extension("UNSMRY"))
        .map_err(|err| exceptions::IOError::py_err(err.to_string()))?;

    let summary =
        Summary::new(smspec, unsmry).map_err(|err| exceptions::IOError::py_err(err.to_string()))?;

    // serialize summary data in the MessagePack format
    let mut wr = Vec::with_capacity(128);
    let mut se = rmps::encode::Serializer::new(&mut wr)
        .with_struct_map()
        .with_string_variants();
    summary
        .serialize(&mut se)
        .map_err(|err| exceptions::IOError::py_err(err.to_string()))?;

    let gil = Python::acquire_gil();
    let py = gil.python();
    let b = PyBytes::new(py, &wr);
    Ok(b.to_object(py))
}

#[pymodule]
/// A Python module implemented in Rust.
fn eclpy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(as_msgpack))?;

    Ok(())
}
