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

#[cfg(feature = "universal")]
use crate::functors::{
    codings::{
        ASCII,
        UTF8, UTF16, UTF32
    }
};

#[cfg(not(feature = "universal"))]
use crate::functors::{
    codings::{
        ASCII,
        UTF16, UTF32
    },
    non_simd_codings::{
        *
    }
};

use pyo3::{
    PyResult, Bound,
    pymodule, pyclass, pymethods,
    types::{
        PyModule,
        PyModuleMethods,
        PyBytes,
        PyBytesMethods,
        PyBool,
        PyInt,
        PyNone,
        PyAny,
        PyAnyMethods
    }
};

#[pyclass(name="ASCII")]
struct ASCIIWrapper;

#[pymethods]
impl ASCIIWrapper {

    #[staticmethod]
    #[pyo3(name = "is_ascii")]
    pub fn is_ascii_ffi(bytes: &Bound<'_, PyBytes>) -> bool {
        return ASCII::is_ascii_from_byte_array(bytes.as_bytes());
    }

    #[staticmethod]
    #[pyo3(name = "search_pattern")]
    pub fn search_pattern_ffi(bytes: &Bound<'_, PyBytes>, pattern_bytes: &Bound<'_, PyBytes>, overlapping: &Bound<'_, PyBool>, all_matches: &Bound<'_, PyBool>, limit: &Bound<'_, PyAny>) -> Vec<usize> {
        return ASCII::search_pattern(
            bytes.as_bytes(),
            pattern_bytes.as_bytes(),
            overlapping.extract::<bool>().expect("[ASCII | search_pattern_ffi | ERROR]: Can't extract overlapping"),
            all_matches.extract::<bool>().expect("[ASCII | search_pattern_ffi | ERROR]: Can't extract all_matches"),
            if limit.is_instance_of::<PyNone>() { None }
            else { if limit.is_instance_of::<PyInt>() { Some(limit.extract::<usize>().expect("[ASCII | search_pattern_ffi | ERROR]: Can't extract limit")) } else { None } }
        )
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

    #[staticmethod]
    #[pyo3(name = "search_pattern")]
    pub fn search_pattern_ffi(bytes: &Bound<'_, PyBytes>, pattern_bytes: &Bound<'_, PyBytes>, overlapping: &Bound<'_, PyBool>, all_matches: &Bound<'_, PyBool>, limit: &Bound<'_, PyAny>) -> Vec<usize> {
        return UTF8::search_pattern(
            bytes.as_bytes(),
            pattern_bytes.as_bytes(),
            overlapping.extract::<bool>().expect("[UTF-8 | search_pattern_ffi | ERROR]: Can't extract overlapping"),
            all_matches.extract::<bool>().expect("[UTF-8 | search_pattern_ffi | ERROR]: Can't extract all_matches"),
            if limit.is_instance_of::<PyNone>() { None }
            else { if limit.is_instance_of::<PyInt>() { Some(limit.extract::<usize>().expect("[UTF-8 | search_pattern_ffi | ERROR]: Can't extract limit")) } else { None } }
        )
    }
}

#[pyclass(name="UTF16")]
struct UTF16Wrapper;

#[pymethods]
impl UTF16Wrapper {

    #[staticmethod]
    #[pyo3(name = "is_utf16")]
    pub fn is_utf16_ffi(bytes: &Bound<'_, PyBytes>, endian: &Bound<'_, PyBool>, omp: &Bound<'_, PyBool>, only: &Bound<'_, PyBool>) -> bool {
        return UTF16::is_utf16_from_byte_array(
            bytes.as_bytes(),
            endian.extract::<bool>().expect("[UTF-16 | is_utf16_ffi | ERROR]: Can't extract endian"),
            omp.extract::<bool>().expect("[UTF-16 | is_utf16_ffi | ERROR]: Can't extract omp"),
            only.extract::<bool>().expect("[UTF-16 | is_utf16_ffi | ERROR]: Can't extract only"));
    }

    #[staticmethod]
    #[pyo3(name = "search_pattern")]
    pub fn search_pattern_ffi(bytes: &Bound<'_, PyBytes>, pattern_bytes: &Bound<'_, PyBytes>, omp: &Bound<'_, PyBool>, only: &Bound<'_, PyBool>, overlapping: &Bound<'_, PyBool>, all_matches: &Bound<'_, PyBool>, limit: &Bound<'_, PyAny>) -> Vec<usize> {
        return UTF16::search_pattern(
            bytes.as_bytes(),
            pattern_bytes.as_bytes(),
            omp.extract::<bool>().expect("[UTF-16 | is_utf16_ffi | ERROR]: Can't extract omp"),
            only.extract::<bool>().expect("[UTF-16 | is_utf16_ffi | ERROR]: Can't extract only"),
            overlapping.extract::<bool>().expect("[UTF-16 | search_pattern_ffi | ERROR]: Can't extract overlapping"),
            all_matches.extract::<bool>().expect("[UTF-16 | search_pattern_ffi | ERROR]: Can't extract all_matches"),
            if limit.is_instance_of::<PyNone>() { None }
            else { if limit.is_instance_of::<PyInt>() { Some(limit.extract::<usize>().expect("[UTF-16 | search_pattern_ffi | ERROR]: Can't extract limit")) } else { None } }
        )
    }
}

#[pyclass(name="UTF32")]
struct UTF32Wrapper;

#[pymethods]
impl UTF32Wrapper {

    #[staticmethod]
    #[pyo3(name = "is_utf32")]
    pub fn is_utf32_ffi(bytes: &Bound<'_, PyBytes>, endian: &Bound<'_, PyBool>) -> bool {
        return UTF32::is_utf32_from_byte_array(
            bytes.as_bytes(),
            endian.extract::<bool>().expect("[UTF-32 | is_utf32_ffi | ERROR]: Can't extract endian")
        );
    }

    #[staticmethod]
    #[pyo3(name = "search_pattern")]
    pub fn search_pattern_ffi(bytes: &Bound<'_, PyBytes>, pattern_bytes: &Bound<'_, PyBytes>, overlapping: &Bound<'_, PyBool>, all_matches: &Bound<'_, PyBool>, limit: &Bound<'_, PyAny>) -> Vec<usize> {
        return UTF32::search_pattern(
            bytes.as_bytes(),
            pattern_bytes.as_bytes(),
            overlapping.extract::<bool>().expect("[UTF-32 | search_pattern_ffi | ERROR]: Can't extract overlapping"),
            all_matches.extract::<bool>().expect("[UTF-32 | search_pattern_ffi | ERROR]: Can't extract all_matches"),
            if limit.is_instance_of::<PyNone>() { None }
            else { if limit.is_instance_of::<PyInt>() { Some(limit.extract::<usize>().expect("[UTF-32 | search_pattern_ffi | ERROR]: Can't extract limit")) } else { None } }
        )
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