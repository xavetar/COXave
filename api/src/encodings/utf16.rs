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

pub struct UTF16;

impl UTF16 {
    const __ENCODING_REGULAR_PAIR_BYTES:   usize = 2_usize;
    const __ENCODING_SURROGATE_PAIR_BYTES: usize = 4_usize;

    const fn is_lead_part_of_surrogate(code: u16) -> bool {
        return if code >= 0xD800 && code <= 0xDBFF { true } else { false };
    }

    const fn is_not_lead_part_of_surrogate(code: u16) -> bool {
        return if code < 0xD800 && code > 0xDBFF { true } else { false };
    }

    const fn is_part_of_surrogate(code: u16) -> bool {
        return if code >= 0xDC00 && code <= 0xDFFF { true } else { false };
    }

    const fn is_bmp(code: u16) -> bool {
        return if code <= 0xD7FF || code >= 0xE000 { true } else { false };
    }

    const fn is_omp(first_byte: u16, second_byte: u16) -> bool {
        return if (first_byte >= 0xD800 && first_byte <= 0xDBFF) && (second_byte >= 0xDC00 && second_byte <= 0xDFFF) { true } else { false };
    }

    const fn is_not_bmp(code: u16) -> bool {
        return if code > 0xD7FF && code < 0xE000 { true } else { false };
    }

    const fn is_not_omp(first_byte: u16, second_byte: u16) -> bool {
        return if (first_byte < 0xD800 || first_byte > 0xDBFF) && (second_byte < 0xDC00 || second_byte > 0xDFFF) { true } else { false };
    }

    pub const fn is_utf16(array: &[u16], endian: bool, omp: bool, only: bool) -> bool {
        const fn swap_endian(value: u16) -> u16 { return ((value & 0xFF00) >> 8) | ((value & 0x00FF) << 8); }

        let (mut index, length): (usize, usize) = (
            0_usize,
            if omp && only || only { array.len() }
            else {
                let mut length: usize = array.len();

                if length == 0_usize {
                    return false;
                } else if length == 1_usize {
                    #[cfg(target_endian = "big")]
                    if endian {
                        return if UTF16::is_bmp(swap_endian(array[length - 1_usize])) { true } else { false };
                    } else {
                        return if UTF16::is_bmp(array[length - 1_usize]) { true } else { false };
                    }

                    #[cfg(target_endian = "little")]
                    if endian {
                        return if UTF16::is_bmp(array[length - 1_usize]) { true } else { false };
                    } else {
                        return if UTF16::is_bmp(swap_endian(array[length - 1_usize])) { true } else { false };
                    }
                } else if length > 1_usize {
                    #[cfg(target_endian = "big")]
                    let (first, second): (u16, u16) = if endian {
                        (swap_endian(array[length - 2_usize]), swap_endian(array[length - 1_usize]))
                    } else {
                        (array[length - 2_usize], array[length - 1_usize])
                    };

                    #[cfg(target_endian = "little")]
                    let (first, second): (u16, u16) = if endian {
                        (array[length - 2_usize], array[length - 1_usize])
                    } else {
                        (swap_endian(array[length - 2_usize]), swap_endian(array[length - 1_usize]))
                    };

                    if length == 2_usize {
                        return if UTF16::is_bmp(first) && UTF16::is_bmp(second) { true }
                        else if UTF16::is_omp(first, second) { true }
                        else { false };
                    } else {
                        if UTF16::is_bmp(first) && UTF16::is_bmp(second) { length -= 2_usize; }
                        else if UTF16::is_omp(first, second) { length -= 2_usize; }
                        else if UTF16::is_part_of_surrogate(first) && UTF16::is_bmp(second) { length -= 1_usize; }
                        else { return false; }
                    }
                }

                length
            }
        );

        #[cfg(target_endian = "big")]
        if endian {
            if omp {
                if only {
                    while index < length { if UTF16::is_not_omp(swap_endian(array[index]), swap_endian(array[index + 1])) {return false; }; index += 2_usize; }
                } else {
                    let mut swapped: u16;

                    while index < length {
                        swapped = swap_endian(array[index]);

                        if UTF16::is_bmp(swapped) { index += 1_usize; }
                        else if UTF16::is_omp(swapped, swap_endian(array[index + 1])) { index += 2_usize; }
                        else { return false; }
                    }
                }
            } else {
                while index < length { if UTF16::is_not_bmp(swap_endian(array[index])) { return false; }; index += 1_usize; }
            }
        } else {
            if omp {
                if only {
                    while index < length { if UTF16::is_not_omp(array[index], array[index + 1]) { return false; }; index += 2_usize; }
                } else {
                    while index < length {
                        if UTF16::is_bmp(array[index]) { index += 1_usize }
                        else if UTF16::is_omp(array[index], array[index + 1]) { index += 2_usize }
                        else { return false; }
                    }
                }
            } else {
                while index < length { if UTF16::is_not_bmp(array[index]) { return false; }; index += 1_usize; }
            }
        }

        #[cfg(target_endian = "little")]
        if endian {
            if omp {
                if only {
                    while index < length { if UTF16::is_not_omp(array[index], array[index + 1]) { return false; }; index += 2_usize; }
                } else {
                    while index < length {
                        if UTF16::is_bmp(array[index]) { index += 1_usize }
                        else if UTF16::is_omp(array[index], array[index + 1]) { index += 2_usize }
                        else { return false; }
                    }
                }
            } else {
                while index < length { if UTF16::is_not_bmp(array[index]) { return false; }; index += 1_usize; }
            }
        } else {
            if omp {
                if only {
                    while index < length { if UTF16::is_not_omp(swap_endian(array[index]), swap_endian(array[index + 1])) { return false; }; index += 2_usize; }
                } else {
                    let mut swapped: u16;

                    while index < length {
                        swapped = swap_endian(array[index]);

                        if UTF16::is_bmp(swapped) { index += 1_usize; }
                        else if UTF16::is_omp(swapped, swap_endian(array[index + 1])) { index += 2_usize; }
                        else { return false; }
                    }
                }
            } else {
                while index < length { if UTF16::is_not_bmp(swap_endian(array[index])) { return false; }; index += 1_usize; }
            }
        }

        return true;
    }

    pub const fn is_utf16_from_byte_array(array: &[u8], endian: bool, omp: bool, only: bool) -> bool {

        let length: usize = array.len();

        if length == 0 { return false; }
        else if omp && only {
            if length % UTF16::__ENCODING_SURROGATE_PAIR_BYTES != 0_usize { return false; }
        } else {
            if length % UTF16::__ENCODING_REGULAR_PAIR_BYTES != 0_usize { return false; }
        }

        return UTF16::is_utf16(unsafe { std::slice::from_raw_parts::<u16>(array.as_ptr() as *const u16, length / UTF16::__ENCODING_REGULAR_PAIR_BYTES) }, endian, omp, only);
    }
}
