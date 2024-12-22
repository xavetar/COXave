/*
 * Copyright 2024 Stanislav Mikhailov (xavetar)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

use crate::encodings::{
    ASCII,
    UTF8, UTF16, UTF32
};

use pyo3::{
    PyResult, Bound,
    pymodule, pyclass, pymethods,
    types::{
        PyModule,
        PyModuleMethods,
        PyBytes,
        PyBytesMethods,
    }
};

#[pyclass(name="ASCII")]
struct ASCIIWrapper;

#[pymethods]
impl ASCIIWrapper {

    #[staticmethod]
    #[pyo3(name = "is_ascii")]
    pub fn is_ascii_ffi(bytes: &Bound<'_, PyBytes>) -> bool {
        return ASCII::is_ascii(bytes.as_bytes());
    }
}

#[pyclass(name="UTF8")]
struct UTF8Wrapper;

#[pymethods]
impl UTF8Wrapper {

    #[staticmethod]
    #[pyo3(name = "is_utf8")]
    pub fn is_utf8_ffi(bytes: &Bound<'_, PyBytes>) -> bool {
        return UTF8::is_utf8(bytes.as_bytes());
    }
}

#[pyclass(name="UTF16")]
struct UTF16Wrapper;

#[pymethods]
impl UTF16Wrapper {

    #[staticmethod]
    #[pyo3(name = "is_utf16")]
    pub fn is_utf16_ffi(bytes: &Bound<'_, PyBytes>, endian: bool, omp: bool, only: bool) -> bool {
        return UTF16::is_utf16_from_byte_array(bytes.as_bytes(), endian, omp, only);
    }
}

#[pyclass(name="UTF32")]
struct UTF32Wrapper;

#[pymethods]
impl UTF32Wrapper {

    #[staticmethod]
    #[pyo3(name = "is_utf32")]
    pub fn is_utf32_ffi(bytes: &Bound<'_, PyBytes>, endian: bool) -> bool {
        return UTF32::is_utf32_from_byte_array(bytes.as_bytes(), endian);
    }
}

#[pymodule]
fn COXave(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ASCIIWrapper>().expect("Class ASCII cannot be added!");
    module.add_class::<UTF8Wrapper>().expect("Class UTF8 cannot be added!");
    module.add_class::<UTF16Wrapper>().expect("Class UTF16 cannot be added!");
    module.add_class::<UTF32Wrapper>().expect("Class UTF32 cannot be added!");

    return Ok(());
}