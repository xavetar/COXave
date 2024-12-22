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

    const fn is_not_utf32(code: u32) -> bool {
        return if (code & 0xFFFF0000) == 0x00000000 && (code & 0x0000F800) == 0x0000D800 { true }
        else if (code & 0xFFFF0000) > 0x00100000 { true }
        else { false };
    }

    pub const fn is_utf32(array: &[u32], endian: bool) -> bool {
        const fn swap_endian(value: u32) -> u32 {
            return ((value & 0xFF000000) >> 24)
                 | ((value & 0x00FF0000) >> 8)
                 | ((value & 0x0000FF00) << 8)
                 | ((value & 0x000000FF) << 24);
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

    pub const fn is_utf32_from_byte_array(bytes: &[u8], endian: bool) -> bool {
        let length: usize = bytes.len();

        return if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { false }
        else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u32>(bytes.as_ptr() as *const u32, length / UTF32::__ENCODING_BYTES) }, endian) }
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
            let (mut index, mut matches, mut start_index): (usize, usize, usize) = (0_usize, 0_usize, 0_usize);

            while index < array_length {
                if array[index] == pattern[matches] {
                    if matches == 0_usize { matches += 1_usize; start_index = index; } else { matches += 1_usize }

                    if pattern_length == matches { search_result.push(start_index); matches = 0_usize; start_index = 0_usize; if !all_matches { return search_result; } }
                } else { matches = 0_usize; start_index = 0_usize; }

                index += 1;
            }

            search_result
        } else { search_result }
    }
}
