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

pub struct ASCII;

impl ASCII {
    const fn is_not_ascii(code: u128) -> bool {
        return if (code & 0x80808080808080808080808080808080) != 0 { true } else { false };
    }

    pub const fn is_ascii(array: &[u128]) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        if length == 0_usize { return false; }

        while index < length { if ASCII::is_not_ascii(array[index]) { return false; } else { index += 1_usize; } }

        return true;
    }

    pub const fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        let (mut index, mut indivisible_code_array): (usize, [u8; 16_usize]) = (0_usize, [0_u8; 16_usize]);

        return if length == 0_usize { false }
        else if length < 16_usize {
            while index < length { indivisible_code_array[index] = array[index]; index += 1_usize; }

            if ASCII::is_not_ascii(u128::from_ne_bytes(indivisible_code_array)) { false } else { true }
        } else {
            let indivisible: usize = length % 16_usize;

            if indivisible != 0_usize {
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }

                if ASCII::is_not_ascii(u128::from_ne_bytes(indivisible_code_array)) { false }
                else { ASCII::is_ascii(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr().add(indivisible) as *const u128, (length - indivisible) / 16_usize) }) }
            } else { ASCII::is_ascii(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr() as *const u128, length / 16_usize) }) }
        }
    }

    pub fn search_pattern(array: &[u8], pattern: &[u8], all_matches: bool, limit: Option<usize>) -> Vec<usize> {
        let mut search_result: Vec<usize> = Vec::<usize>::new();

        let (array_length, pattern_length): (usize, usize) = match limit {
            Some(limit) => {
                let (mut array_length, pattern_length): (usize, usize) = (array.len(), pattern.len());

                if (pattern_length == 0_usize) || (array_length == 0_usize) { return search_result; }
                else if limit > 0_usize {
                    if limit >= array_length { (array_length, pattern_length) }
                    else {
                        array_length -= limit;

                        if pattern_length > array_length { return search_result; }
                        else { (array_length, pattern_length) }
                    }
                } else { return search_result; }
            }
            None => {
                let (array_length, pattern_length): (usize, usize) = (array.len(), pattern.len());

                if pattern_length > array_length { return search_result; }
                else if (pattern_length == 0_usize) || (array_length == 0_usize) { return search_result; }

                (array.len(), pattern.len())
            }
        };

        return if ASCII::is_ascii_from_byte_array(array) && ASCII::is_ascii_from_byte_array(pattern) {
            let (mut index, mut next_index, mut matches, mut start_index, last_pattern_index)
            :
            (usize, usize, usize, usize, usize)
            =
            (0_usize, 0_usize, 0_usize, 0_usize, pattern_length - 1_usize);

            if pattern_length == 1_usize {
                while index < array_length {
                    if array[index] != pattern[0_usize] { index += 1_usize }
                    else { search_result.push(index); index += 1_usize; if !all_matches { return search_result; }}
                }
            } else if pattern_length == 2_usize {
                while index < array_length {
                    if array[index] != pattern[0_usize] { index += 1_usize; continue; }
                    else {
                        next_index = index + 1_usize;

                        if next_index >= array_length { return search_result; }
                        else if array[next_index] != pattern[last_pattern_index] { if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                        else { search_result.push(index); index += 2_usize; if !all_matches { return search_result; } else { continue; } }
                    }
                }
            } else if pattern_length >= 3_usize {
                let penultimate_pattern_index: usize = pattern_length - 2_usize;

                while index < array_length {
                    if matches != 0_usize {
                        if start_index + last_pattern_index >= array_length { return search_result; }
                        else if array[start_index + last_pattern_index] == pattern[last_pattern_index] {
                            while matches < last_pattern_index {
                                if array[index] == pattern[matches] {
                                    if matches != penultimate_pattern_index { matches += 1_usize; index += 1_usize; }
                                    else { search_result.push(start_index); matches = 0_usize; if all_matches { break; } else { return search_result; } }
                                } else {
                                    if next_index > 0_usize { start_index = next_index; index = next_index + 1_usize; matches = 1_usize; next_index = 0_usize; break; }
                                    else { matches = 0_usize; break; }
                                }

                                if next_index == 0_usize { if array[index] == pattern[0_usize] { next_index = index; } }
                            }
                        } else { matches = 0_usize; continue; }
                    } else {
                        while index < array_length {
                            if array[index] != pattern[0_usize] { index += 1_usize }
                            else { start_index = index; index += 1_usize; matches = 1_usize; break }
                        }
                    }
                }
            }
            search_result
        } else { search_result }
    }
}
