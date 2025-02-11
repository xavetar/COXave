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

use std::{
    mem::{
        transmute
    }
};

pub use crate::{
    essence::{
        ByteSearch
    }
};

impl<T: std::cmp::PartialEq> ByteSearch<T> {

    const fn is_search_possible(array: &[T], pattern: &[T], limit: Option<usize>) -> (usize, usize, bool) {
        return match limit {
            Some(limit) => {
                let (mut array_length, pattern_length): (usize, usize) = (array.len(), pattern.len());

                if (pattern_length == 0_usize) || (array_length == 0_usize) || (pattern_length > array_length) {
                    return (0_usize, 0_usize, false);
                } else if limit > 0_usize {
                    if limit >= array_length { (array_length - (pattern_length - 1_usize), pattern_length, true) }
                    else {
                        array_length -= limit;

                        if pattern_length > array_length { return (0_usize, 0_usize, false); }
                        else { (array_length - (pattern_length - 1_usize), pattern_length, true) }
                    }
                } else { return (0_usize, 0_usize, false); }
            }
            None => {
                let (array_length, pattern_length): (usize, usize) = (array.len(), pattern.len());

                if (pattern_length == 0_usize) || (array_length == 0_usize) || (pattern_length > array_length) {
                    return (0_usize, 0_usize, false);
                }

                (array_length - (pattern_length - 1_usize), pattern_length, true)
            }
        };
    }

    pub fn search_single(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize> {
        let mut search_result: Vec<usize> = Vec::<usize>::new();

        let (array, pattern): (&[T], &[T]) = (
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(array_ptr.as_ptr()), array_ptr.len() / size_of::<T>()) },
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(pattern_ptr.as_ptr()), pattern_ptr.len() / size_of::<T>()) }
        );

        let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<T>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

        let (mut index, mut next_index, mut matches, mut start_index, last_pattern_index): (usize, usize, usize, usize, usize) = (0_usize, 0_usize, 0_usize, 0_usize, pattern_length - 1_usize);

        if pattern_length == 1_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize }
                else { search_result.push(index * size_of::<T>()); return search_result; }
            }
        } else if pattern_length == 2_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize; continue; }
                else {
                    next_index = index + 1_usize;

                    if array[next_index] != pattern[last_pattern_index] { if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                    else { search_result.push(index * size_of::<T>()); return search_result; }
                }
            }
        } else if pattern_length >= 3_usize {
            let penultimate_pattern_index: usize = pattern_length - 2_usize;

            while start_index < array_length {
                if matches != 0_usize {
                    if array[start_index + last_pattern_index] == pattern[last_pattern_index] {
                        while matches < last_pattern_index {
                            if array[index] == pattern[matches] {
                                if matches != penultimate_pattern_index { matches += 1_usize; index += 1_usize; }
                                else { search_result.push(start_index * size_of::<T>()); return search_result; }
                            } else {
                                if next_index > 0_usize { start_index = next_index; index = next_index + 1_usize; matches = 1_usize; next_index = 0_usize; break; }
                                else { matches = 0_usize; break; }
                            }

                            if next_index == 0_usize { if array[index] == pattern[0_usize] { next_index = index; } }
                        }
                    } else { matches = 0_usize; continue; }
                } else {
                    while start_index < array_length {
                        if array[index] != pattern[0_usize] { index += 1_usize; start_index = index; }
                        else { start_index = index; index += 1_usize; matches = 1_usize; break }
                    }
                }
            }
        }

        return search_result;
    }

    pub fn search_all(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize> {
        let mut search_result: Vec<usize> = Vec::<usize>::new();

        let (array, pattern): (&[T], &[T]) = (
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(array_ptr.as_ptr()), array_ptr.len() / size_of::<T>()) },
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(pattern_ptr.as_ptr()), pattern_ptr.len() / size_of::<T>()) }
        );

        let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<T>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

        let (mut index, mut next_index, mut matches, mut start_index, last_pattern_index): (usize, usize, usize, usize, usize) = (0_usize, 0_usize, 0_usize, 0_usize, pattern_length - 1_usize);

        if pattern_length == 1_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize }
                else { search_result.push(index * size_of::<T>()); index += 1_usize; continue; }
            }
        } else if pattern_length == 2_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize; continue; }
                else {
                    next_index = index + 1_usize;

                    if array[next_index] != pattern[last_pattern_index] { if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                    else { search_result.push(index * size_of::<T>()); index += 2_usize; continue; }
                }
            }
        } else if pattern_length >= 3_usize {
            let penultimate_pattern_index: usize = pattern_length - 2_usize;

            while start_index < array_length {
                if matches != 0_usize {
                    if array[start_index + last_pattern_index] == pattern[last_pattern_index] {
                        while matches < last_pattern_index {
                            if array[index] == pattern[matches] {
                                if matches != penultimate_pattern_index { matches += 1_usize; index += 1_usize; }
                                else { search_result.push(start_index * size_of::<T>()); matches = 0_usize; index = start_index + pattern_length; break; }
                            } else {
                                if next_index > 0_usize { start_index = next_index; index = next_index + 1_usize; matches = 1_usize; next_index = 0_usize; break; }
                                else { matches = 0_usize; break; }
                            }

                            if next_index == 0_usize { if array[index] == pattern[0_usize] { next_index = index; } }
                        }
                    } else { matches = 0_usize; continue; }
                } else {
                    while start_index < array_length {
                        if array[index] != pattern[0_usize] { index += 1_usize; start_index = index; }
                        else { start_index = index; index += 1_usize; matches = 1_usize; break }
                    }
                }
            }
        }

        return search_result;
    }

    pub fn search_all_overlapping(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize> {
        let mut search_result: Vec<usize> = Vec::<usize>::new();

        let (array, pattern): (&[T], &[T]) = (
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(array_ptr.as_ptr()), array_ptr.len() / size_of::<T>()) },
            unsafe { std::slice::from_raw_parts::<T>(transmute::<*const u8, *const T>(pattern_ptr.as_ptr()), pattern_ptr.len() / size_of::<T>()) }
        );

        let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<T>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

        let (mut index, mut next_index, mut matches, mut start_index, last_pattern_index): (usize, usize, usize, usize, usize) = (0_usize, 0_usize, 0_usize, 0_usize, pattern_length - 1_usize);

        if pattern_length == 1_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize }
                else { search_result.push(index * size_of::<T>()); index += 1_usize; continue; }
            }
        } else if pattern_length == 2_usize {
            while index < array_length {
                if array[index] != pattern[0_usize] { index += 1_usize; continue; }
                else {
                    next_index = index + 1_usize;

                    if array[next_index] != pattern[last_pattern_index] { if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                    else { search_result.push(index * size_of::<T>()); if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                }
            }
        } else if pattern_length >= 3_usize {
            let penultimate_pattern_index: usize = pattern_length - 2_usize;

            while start_index < array_length {
                if matches != 0_usize {
                    if array[start_index + last_pattern_index] == pattern[last_pattern_index] {
                        while matches < last_pattern_index {
                            if array[index] == pattern[matches] {
                                if matches != penultimate_pattern_index {
                                    matches += 1_usize; index += 1_usize;
                                } else {
                                    search_result.push(start_index * size_of::<T>());

                                    if next_index > 0_usize { start_index = next_index; index = next_index + 1_usize; matches = 1_usize; next_index = 0_usize; break; }
                                    else { matches = 0_usize; break; }
                                }
                            } else {
                                if next_index > 0_usize { start_index = next_index; index = next_index + 1_usize; matches = 1_usize; next_index = 0_usize; break; }
                                else { matches = 0_usize; break; }
                            }

                            if next_index == 0_usize { if array[index] == pattern[0_usize] { next_index = index; } }
                        }
                    } else { matches = 0_usize; continue; }
                } else {
                    while start_index < array_length {
                        if array[index] != pattern[0_usize] { index += 1_usize; start_index = index; }
                        else { start_index = index; index += 1_usize; matches = 1_usize; break }
                    }
                }
            }
        }

        return search_result;
    }
}
