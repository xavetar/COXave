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

pub struct UTF32;

impl UTF32 {
    const __ENCODING_BYTES: usize = 4_usize;

    const fn is_not_utf32(code: u128) -> bool {
        return if (code & 0xFFFF0000FFFF0000FFFF0000FFFF0000) == 0x00000000000000000000000000000000  {

            let mask_result: u128 = !(code ^ 0xFFFFD8FFFFFFD8FFFFFFD8FFFFFFD8FF) | 0x00000700000007000000070000000700;

            if (((mask_result >> 8) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 40) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 72) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 104) & 0xFF) as u8 == 0xFF) { true } else { false }
        }
        else {
            if (((code >> 16) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 48) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 80) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 112) & 0xFFFF) as u16 > 0x0010) { true } else { false }
        }
    }

    pub const fn is_utf32(array: &[u128], endian: bool) -> bool {
        const fn swap_endian(value: u128) -> u128 {
            return ((value & 0xFF000000000000000000000000000000) >> 24)
                 | ((value & 0x00FF0000000000000000000000000000) >> 8)
                 | ((value & 0x0000FF00000000000000000000000000) << 8)
                 | ((value & 0x000000FF000000000000000000000000) << 24)
                 | ((value & 0x00000000FF0000000000000000000000) >> 24)
                 | ((value & 0x0000000000FF00000000000000000000) >> 8)
                 | ((value & 0x000000000000FF000000000000000000) << 8)
                 | ((value & 0x00000000000000FF0000000000000000) << 24)
                 | ((value & 0x0000000000000000FF00000000000000) >> 24)
                 | ((value & 0x000000000000000000FF000000000000) >> 8)
                 | ((value & 0x00000000000000000000FF0000000000) << 8)
                 | ((value & 0x0000000000000000000000FF00000000) << 24)
                 | ((value & 0x000000000000000000000000FF000000) >> 24)
                 | ((value & 0x00000000000000000000000000FF0000) >> 8)
                 | ((value & 0x0000000000000000000000000000FF00) << 8)
                 | ((value & 0x000000000000000000000000000000FF) << 24);
        }

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        #[cfg(target_endian = "big")]
        if endian {
            while index < length { if UTF32::is_not_utf32(swap_endian(array[index])) { return false; }; index += 1_usize; }
        } else {
            while index < length { if UTF32::is_not_utf32(array[index]) { return false; }; index += 1_usize; }
        }

        #[cfg(target_endian = "little")]
        if endian {
            while index < length { if UTF32::is_not_utf32(array[index]) { return false; }; index += 1_usize; }
        } else {
            while index < length { if UTF32::is_not_utf32(swap_endian(array[index])) { return false; }; index += 1_usize; }
        }

        return true;
    }

    pub const fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {
        let length: usize = array.len();

        let (mut index, mut indivisible_code_array): (usize, [u8; 16_usize]) = (0_usize, [0_u8; 16_usize]);

        return if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { false }
        else if length < 16_usize {
            while index < length { indivisible_code_array[index] = array[index]; index += 1_usize; }

            UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(indivisible_code_array.as_ptr() as *const u128, 1_usize) }, endian)
        } else {
            let indivisible: usize = length % 16_usize;

            if indivisible != 0_usize {
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1; }

                if !UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(indivisible_code_array.as_ptr() as *const u128, 1_usize) }, endian) { false }
                else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr().add(indivisible) as *const u128, (length - indivisible) / 16_usize) }, endian) }
            } else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr() as *const u128, length / 16_usize) }, endian) }
        }
    }

    pub fn search_pattern(array_ptr: &[u8], pattern_ptr: &[u8], all_matches: bool, limit: Option<usize>, endian: bool) -> Vec<usize> {

        let (array, pattern): (&[u32], &[u32]) = (
            unsafe { std::slice::from_raw_parts::<u32>(array_ptr.as_ptr() as *const u32, array_ptr.len() / size_of::<u32>()) },
            unsafe { std::slice::from_raw_parts::<u32>(pattern_ptr.as_ptr() as *const u32, pattern_ptr.len() / size_of::<u32>()) }
        );

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

        return if UTF32::is_utf32_from_byte_array(array_ptr, endian) && UTF32::is_utf32_from_byte_array(pattern_ptr, endian) {
            let (
                mut index, mut next_index, mut matches, mut start_index, last_pattern_index, penultimate_pattern_index
            ): (
                usize, usize, usize, usize, usize, usize
            ) = (
                0_usize, 0_usize, 0_usize, 0_usize, pattern_length - 1_usize, pattern_length - 2_usize
            );

            if pattern_length == 1_usize {
                while index < array_length {
                    if array[index] != pattern[0_usize] { index += 1_usize }
                    else { search_result.push(index * size_of::<u32>()); index += 1_usize; if !all_matches { return search_result; } }
                }
            } else if pattern_length == 2_usize {
                while index < array_length {
                    if array[index] != pattern[0_usize] { index += 1_usize; continue; }
                    else {
                        next_index = index + 1_usize;

                        if next_index >= array_length { return search_result; }
                        else if array[next_index] != pattern[last_pattern_index] { if array[next_index] != pattern[0_usize] { index += 2_usize; continue; } else { index += 1_usize; continue; } }
                        else { search_result.push(index * size_of::<u32>()); index += 2_usize; if !all_matches { return search_result; } else { continue; } }
                    }
                }
            } else if pattern_length >= 3_usize {
                while index < array_length {
                    if matches != 0_usize {
                        if start_index + last_pattern_index >= array_length { return search_result; }
                        else if array[start_index + last_pattern_index] == pattern[last_pattern_index] {
                            while matches < last_pattern_index {
                                if array[index] == pattern[matches] {
                                    if matches != penultimate_pattern_index { matches += 1_usize; index += 1_usize; }
                                    else { search_result.push(start_index * size_of::<u32>()); matches = 0_usize; if all_matches { break; } else { return search_result; } }
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
