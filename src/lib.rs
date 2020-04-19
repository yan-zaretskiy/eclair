pub mod eclipse_binary;
pub mod eclipse_summary;
pub mod errors;

// use pyo3::prelude::*;
// use pyo3::wrap_pyfunction;
//
// #[pyfunction]
// /// Formats the sum of two numbers as string.
// fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
//     Ok((a + b).to_string())
// }
//
// #[pymodule]
// /// A Python module implemented in Rust.
// fn string_sum(py: Python, m: &PyModule) -> PyResult<()> {
//     m.add_wrapped(wrap_pyfunction!(sum_as_string))?;
//
//     Ok(())
// }
