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

pub use crate::{
    essence::{
        UTF16
    }
};

#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
use std::{
    hint::{
        black_box
    },
    mem::{
        transmute
    },
    arch::{
        x86::{
            __m512i,
            __mmask32,
            _mm512_loadu_si512,
            _mm512_set_epi16,
            _mm512_set1_epi16,
            _mm512_cmplt_epi16_mask,
            _mm512_cmpgt_epi16_mask,
            _mm512_cmpeq_epi16_mask,
            _mm512_and_si512,
            _mm512_set_epi8,
            _mm512_shuffle_epi8,
        }
    }
};

#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    hint::{
        black_box
    },
    mem::{
        transmute
    },
    arch::{
        x86::{
            __m256i,
            _mm256_loadu_si256,
            _mm256_set_epi16,
            _mm256_set1_epi16,
            _mm256_cmpgt_epi16,
            _mm256_cmpeq_epi16,
            _mm256_or_si256,
            _mm256_and_si256,
            _mm256_andnot_si256,
            _mm256_movemask_epi8,
            _mm256_set_epi8,
            _mm256_shuffle_epi8,
            _mm256_permute2x128_si256,
            _mm256_alignr_epi8
        }
    }
};

#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    hint::{
        black_box
    },
    mem::{
        transmute
    },
    arch::{
        x86::{
            __m128i,
            _mm_loadu_si128,
            _mm_set_epi16,
            _mm_set1_epi16,
            _mm_cmplt_epi16,
            _mm_cmpgt_epi16,
            _mm_cmpeq_epi16,
            _mm_or_si128,
            _mm_and_si128,
            _mm_andnot_si128,
            _mm_movemask_epi8,
            _mm_bslli_si128,
            _mm_bsrli_si128
        }
    }
};

#[cfg(all(target_feature = "sse2", target_feature = "ssse3", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    arch::{
        x86::{
            _mm_set_epi8,
            _mm_shuffle_epi8
        }
    }
};

#[cfg(all(target_feature = "sse2", not(target_feature = "ssse3"), not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    arch::{
        x86::{
            _mm_sra_epi16,
            _mm_sll_epi16,
        }
    }
};

impl UTF16 {

    const __ENCODING_REGULAR_PAIR_BYTES:   usize = 2_usize;
    const __ENCODING_SURROGATE_PAIR_BYTES: usize = 4_usize;

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    fn is_utf16_bmp_16x32(array: &[__m512i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (bad_range_mask, bad_result_mask): (__m512i, __m512i) =
            unsafe { (_mm512_set1_epi16(-0x0800), _mm512_set1_epi16(-0x2800)) }; // 0xF800, 0xD800

        let swap_endian: __m512i = unsafe { _mm512_set_epi8(
            62, 63, 60, 61, 58, 59, 56, 57, 54, 55, 52, 53, 50, 51, 48, 49,
            46, 47, 44, 45, 42, 43, 40, 41, 38, 39, 36, 37, 34, 35, 32, 33,
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        if endian {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                } else {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                };

                if unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, bad_range_mask), bad_result_mask) } > 0_u32 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                } else {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                };

                if unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, bad_range_mask), bad_result_mask) } > 0_u32 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    fn is_utf16_omp_16x32(array: &[__m512i], endian: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (max_signed_mask, restricted_less_than_mask, restricted_big_than_mask): (__m512i, __m512i, __m512i) =
            // 0xFFFF: -0x0001
            // 0xD800: -0x2800, 0xDC00: -0x2400
            // 0xDBFF: -0x2401, 0xDFFF: -0x2001
            unsafe {
                (
                    _mm512_set1_epi16(-0x0001),
                    _mm512_set_epi16(
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800,
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800,
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800,
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800
                    ),
                    _mm512_set_epi16(
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401,
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401,
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401,
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401
                    ),
                )
            };

        let swap_endian: __m512i = unsafe { _mm512_set_epi8(
            62, 63, 60, 61, 58, 59, 56, 57, 54, 55, 52, 53, 50, 51, 48, 49,
            46, 47, 44, 45, 42, 43, 40, 41, 38, 39, 36, 37, 34, 35, 32, 33,
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        if endian {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                } else {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                };

                if unsafe { _mm512_cmpgt_epi16_mask(value, max_signed_mask) } != 0_u32 { return false; }
                else if unsafe { _mm512_cmpgt_epi16_mask(restricted_less_than_mask, value) | _mm512_cmplt_epi16_mask(restricted_big_than_mask, value) } != 0_u32 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                } else {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                };

                if unsafe { _mm512_cmpgt_epi16_mask(value, max_signed_mask) } != 0_u32 { return false; }
                else if unsafe { _mm512_cmpgt_epi16_mask(restricted_less_than_mask, value) | _mm512_cmplt_epi16_mask(restricted_big_than_mask, value) } != 0_u32 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    fn is_utf16_mixed_16x32(array: &[__m512i], endian: bool, mut continuation: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (any_part_surrogate_detect_mask, following_surrogate_detect_mask): (__m512i, __m512i) = unsafe {
            (_mm512_set1_epi16(-0x0800), _mm512_set1_epi16(-0x0400)) // 0xF800, 0xFC00
        };

        let (high_surrogate_detect_mask, low_surrogate_detect_mask): (__m512i, __m512i) = unsafe {
            (_mm512_set1_epi16(-0x2800), _mm512_set1_epi16(-0x2400)) // 0xD800, 0xDC00
        };

        let (test_continuation_mask, test_following_continuation_mask, ignore_leading_continuation_mask): (__mmask32, __mmask32, __mmask32) = (0x80000000, 0x00000001, 0x7FFFFFFF);

        let swap_endian: __m512i = unsafe { _mm512_set_epi8(
            62, 63, 60, 61, 58, 59, 56, 57, 54, 55, 52, 53, 50, 51, 48, 49,
            46, 47, 44, 45, 42, 43, 40, 41, 38, 39, 36, 37, 34, 35, 32, 33,
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        if endian {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                } else {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                };

                let any_surrogate_mask: __mmask32 = unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if any_surrogate_mask != 0_u32 {
                    let following_surrogate_mask: __mmask32 = unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if following_surrogate_mask != 0_u32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__mmask32, __mmask32) = if (following_surrogate_mask & test_following_continuation_mask) != 0_u32 {
                                if (any_surrogate_mask & test_continuation_mask) == 0_u32 { continuation = false; }
                                else if (following_surrogate_mask & test_continuation_mask) != 0_u32 { continuation = false; }

                                if continuation {
                                    ((any_surrogate_mask & ignore_leading_continuation_mask) >> 1, following_surrogate_mask >> 1)
                                } else {
                                    (any_surrogate_mask >> 1, following_surrogate_mask >> 1)
                                }
                            } else {
                                return false;
                            };

                            if following_surrogate_mask != 0_u32 {
                                let potential_high_surrogates_mask: __mmask32 = following_surrogate_mask >> 1;

                                if (following_surrogate_mask & potential_high_surrogates_mask) == 0_u32 {
                                    let high_surrogates_mask: __mmask32 = any_surrogate_mask & !following_surrogate_mask;

                                    let potential_following_surrogates_mask: __mmask32 = high_surrogates_mask << 1;

                                    if high_surrogates_mask == 0_u32 { return false; }
                                    else if potential_high_surrogates_mask != high_surrogates_mask { return false; }
                                    else if potential_following_surrogates_mask != following_surrogate_mask { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if any_surrogate_mask != 0_u32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__mmask32, __mmask32) = if (any_surrogate_mask & test_continuation_mask) != 0_u32 {
                            if (following_surrogate_mask & test_continuation_mask) == 0_u32 { continuation = true; }

                            if continuation {
                                ((any_surrogate_mask & ignore_leading_continuation_mask), following_surrogate_mask)
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if following_surrogate_mask != 0_u32 {
                            let potential_high_surrogates_mask: __mmask32 = following_surrogate_mask >> 1;

                            if (following_surrogate_mask & potential_high_surrogates_mask) == 0_u32 {

                                let high_surrogates_mask: __mmask32 = any_surrogate_mask & !following_surrogate_mask;

                                let potential_following_surrogates_mask: __mmask32 = high_surrogates_mask << 1;

                                if high_surrogates_mask == 0_u32 { return false; }
                                else if potential_high_surrogates_mask != high_surrogates_mask {return false; }
                                else if potential_following_surrogates_mask != following_surrogate_mask { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if any_surrogate_mask != 0_u32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        } else {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                } else {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                };

                let any_surrogate_mask: __mmask32 = unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if any_surrogate_mask != 0_u32 {
                    let following_surrogate_mask: __mmask32 = unsafe { _mm512_cmpeq_epi16_mask(_mm512_and_si512(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if following_surrogate_mask != 0_u32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__mmask32, __mmask32) = if (following_surrogate_mask & test_following_continuation_mask) != 0_u32 {
                                if (any_surrogate_mask & test_continuation_mask) == 0_u32 { continuation = false; }
                                else if (following_surrogate_mask & test_continuation_mask) != 0_u32 { continuation = false; }

                                if continuation {
                                    ((any_surrogate_mask & ignore_leading_continuation_mask) >> 1, following_surrogate_mask >> 1)
                                } else {
                                    (any_surrogate_mask >> 1, following_surrogate_mask >> 1)
                                }
                            } else {
                                return false;
                            };

                            if following_surrogate_mask != 0_u32 {
                                let potential_high_surrogates_mask: __mmask32 = following_surrogate_mask >> 1;

                                if (following_surrogate_mask & potential_high_surrogates_mask) == 0_u32 {
                                    let high_surrogates_mask: __mmask32 = any_surrogate_mask & !following_surrogate_mask;

                                    let potential_following_surrogates_mask: __mmask32 = high_surrogates_mask << 1;

                                    if high_surrogates_mask == 0_u32 { return false; }
                                    else if potential_high_surrogates_mask != high_surrogates_mask { return false; }
                                    else if potential_following_surrogates_mask != following_surrogate_mask { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if any_surrogate_mask != 0_u32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__mmask32, __mmask32) = if (any_surrogate_mask & test_continuation_mask) != 0_u32 {
                            if (following_surrogate_mask & test_continuation_mask) == 0_u32 { continuation = true; }

                            if continuation {
                                ((any_surrogate_mask & ignore_leading_continuation_mask), following_surrogate_mask)
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if following_surrogate_mask != 0_u32 {
                            let potential_high_surrogates_mask: __mmask32 = following_surrogate_mask >> 1;

                            if (following_surrogate_mask & potential_high_surrogates_mask) == 0_u32 {

                                let high_surrogates_mask: __mmask32 = any_surrogate_mask & !following_surrogate_mask;

                                let potential_following_surrogates_mask: __mmask32 = high_surrogates_mask << 1;

                                if high_surrogates_mask == 0_u32 { return false; }
                                else if potential_high_surrogates_mask != high_surrogates_mask {return false; }
                                else if potential_following_surrogates_mask != following_surrogate_mask { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if any_surrogate_mask != 0_u32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    pub fn is_utf16_from_byte_array(array: &[u8], endian: bool, omp: bool, only: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut continuation, mut result): (usize, usize, bool, bool) = (0_usize, length % 64_usize, false, true);

        if length == 0_usize { return false; }

        if omp && only {
            if length % UTF16::__ENCODING_SURROGATE_PAIR_BYTES != 0_usize { return false; }
        } else {
            if length % UTF16::__ENCODING_REGULAR_PAIR_BYTES != 0_usize { return false; }
        }

        if indivisible != 0_usize {
            if only {
                if omp {
                    let indivisible_code_array: __m512i = {
                        if endian {
                            let mut indivisible_code_array: [u8; 64_usize] = [
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC,
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC,
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC,
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC
                            ];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                        } else {
                            let mut indivisible_code_array: [u8; 64_usize] = [
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00,
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00,
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00,
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00
                            ];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                        }
                    };

                    result &= UTF16::is_utf16_omp_16x32(&[indivisible_code_array], endian);
                } else {
                    let indivisible_code_array: __m512i = {
                        let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_bmp_16x32(&[indivisible_code_array], endian);
                }
            } else {
                if length < 65_usize {
                    let indivisible_code_array: __m512i = {
                        let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_mixed_16x32(&[indivisible_code_array], endian, false);
                } else {
                    let align_indivisible: usize = indivisible + indivisible % 2_usize;

                    let potentially_surrogate_index: usize = if cfg!(target_endian = "big") {
                        if endian { align_indivisible - 2_usize } else { align_indivisible - 1_usize }
                    } else {
                        if endian { align_indivisible - 1_usize } else { align_indivisible - 2_usize }
                    };

                    if (array[potentially_surrogate_index] & 0xFC) != 0xD8 {
                        let indivisible_code_array: __m512i = {
                            let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                            while index < align_indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_mixed_16x32(&[indivisible_code_array], endian, false);
                    } else {
                        continuation = true;

                        if potentially_surrogate_index >= 2_usize {
                            let indivisible_code_array: __m512i = {
                                let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                                while index < align_indivisible - 2_usize { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
                            };

                            result &= UTF16::is_utf16_mixed_16x32(&[indivisible_code_array], endian, false);
                        }
                    }
                }
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                if only {
                    if omp {
                        result &= UTF16::is_utf16_omp_16x32(unsafe { std::slice::from_raw_parts::<__m512i>(transmute::<*const u8, *const __m512i>(array.as_ptr().add(indivisible)), remains_length / 64_usize) }, endian);
                    } else {
                        result &= UTF16::is_utf16_bmp_16x32(unsafe { std::slice::from_raw_parts::<__m512i>(transmute::<*const u8, *const __m512i>(array.as_ptr().add(indivisible)), remains_length / 64_usize) }, endian);
                    }
                } else {
                    result &= UTF16::is_utf16_mixed_16x32(unsafe { std::slice::from_raw_parts::<__m512i>(transmute::<*const u8, *const __m512i>(array.as_ptr().add(indivisible)), remains_length / 64_usize) }, endian, continuation);
                }
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_bmp_16x16(array: &[__m256i], endian: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (bad_range_mask, bad_result_mask): (__m256i, __m256i) =
            unsafe { (_mm256_set1_epi16(-0x0800), _mm256_set1_epi16(-0x2800)) }; // 0xF800, 0xD800

        let swap_endian: __m256i = unsafe { _mm256_set_epi8(
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        if endian {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                } else {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi16(_mm256_and_si256(value, bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                } else {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi16(_mm256_and_si256(value, bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_omp_16x16(array: &[__m256i], endian: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (max_signed_mask, restricted_less_than_mask, restricted_big_than_mask): (__m256i, __m256i, __m256i) =
            // 0xFFFF: -0x0001
            // 0xD800: -0x2800, 0xDC00: -0x2400
            // 0xDBFF: -0x2401, 0xDFFF: -0x2001
            unsafe {
                (
                    _mm256_set1_epi16(-0x0001),
                    _mm256_set_epi16(
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800,
                        -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800
                    ),
                    _mm256_set_epi16(
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401,
                        -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401
                    ),
                )
            };

        let swap_endian: __m256i = unsafe { _mm256_set_epi8(
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        if endian {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                } else {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi16(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_or_si256(_mm256_cmpgt_epi16(restricted_less_than_mask, value), _mm256_cmpgt_epi16(value, restricted_big_than_mask))) } != 0_i32 { return false; }


                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                } else {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi16(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_or_si256(_mm256_cmpgt_epi16(restricted_less_than_mask, value), _mm256_cmpgt_epi16(value, restricted_big_than_mask))) } != 0_i32 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_mixed_16x16(array: &[__m256i], endian: bool, mut continuation: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (any_part_surrogate_detect_mask, following_surrogate_detect_mask): (__m256i, __m256i) = unsafe {
            (_mm256_set1_epi16(-0x0800), _mm256_set1_epi16(-0x0400)) // 0xF800, 0xFC00
        };

        let (high_surrogate_detect_mask, low_surrogate_detect_mask): (__m256i, __m256i) = unsafe {
            (_mm256_set1_epi16(-0x2800), _mm256_set1_epi16(-0x2400)) // 0xD800, 0xDC00
        };

        let (test_continuation_mask, test_following_continuation_mask, ignore_leading_continuation_mask): (__m256i, __m256i, __m256i) = unsafe {
            // 0xFFFF: -0x0001
            (
                _mm256_set_epi16(
                    -0x0001, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
                     0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000
                ),
                _mm256_set_epi16(
                    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,  0x0000,
                    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, -0x0001
                ),
                _mm256_set_epi16(
                     0x0000, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001,
                    -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001
                )
            )
        };

        let and_not_mask: __m256i = unsafe { _mm256_set1_epi16(-0x0001) }; // 0xFFFF: -0x0001

        let swap_endian: __m256i = unsafe { _mm256_set_epi8(
            30, 31, 28, 29, 26, 27, 24, 25, 22, 23, 20, 21, 18, 19, 16, 17,
            14, 15, 12, 13, 10, 11,  8,  9,  6,  7,  4,  5,  2,  3,  0,  1
        ) };

        let (zero_left, zero_right): (__m256i, __m256i) = unsafe {
            (
                _mm256_set_epi8(
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,  0x00,  0x00
                ),
                _mm256_set_epi8(
                     0x00,  0x00, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01,
                    -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01, -0x01
                )
            )
        };

        let (shift_left, shift_right): (fn(value: __m256i, zero: __m256i) -> __m256i, fn(value: __m256i, zero: __m256i) -> __m256i) =
            (
                |value: __m256i, zero: __m256i| -> __m256i { unsafe { _mm256_and_si256(_mm256_alignr_epi8::<14>(value, _mm256_permute2x128_si256::<1>(value, value)), zero) } },
                |value: __m256i, zero: __m256i| -> __m256i { unsafe { _mm256_and_si256(_mm256_alignr_epi8::<2>(_mm256_permute2x128_si256::<1>(value, value), value), zero) } }
            );

        if endian {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                } else {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                };

                let any_surrogate_mask: __m256i = unsafe { _mm256_cmpeq_epi16(_mm256_and_si256(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 {
                    let following_surrogate_mask: __m256i = unsafe { _mm256_cmpeq_epi16(_mm256_and_si256(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__m256i, __m256i) = if unsafe {
                                _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_i32 {
                                if unsafe { _mm256_movemask_epi8(_mm256_and_si256(any_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = false; }
                                else if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_continuation_mask)) } != 0_i32 { continuation = false; }

                                if continuation {
                                    (shift_right(unsafe { _mm256_and_si256(any_surrogate_mask, ignore_leading_continuation_mask) }, zero_right), shift_right(following_surrogate_mask, zero_right))
                                } else {
                                    (shift_right(any_surrogate_mask, zero_right), shift_right(following_surrogate_mask, zero_right))
                                }
                            } else {
                                return false;
                            };

                            if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                                let potential_high_surrogates_mask: __m256i = shift_right(following_surrogate_mask, zero_right);

                                if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {
                                    let high_surrogates_mask: __m256i = unsafe { _mm256_and_si256(any_surrogate_mask, _mm256_andnot_si256(following_surrogate_mask, and_not_mask)) };

                                    let potential_following_surrogates_mask: __m256i = shift_left(high_surrogates_mask, zero_left);

                                    if unsafe { _mm256_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                    else if unsafe { _mm256_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm256_movemask_epi8(high_surrogates_mask) } { return false; }
                                    else if unsafe { _mm256_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm256_movemask_epi8(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__m256i, __m256i) = if unsafe {
                            _mm256_movemask_epi8(_mm256_and_si256(any_surrogate_mask, test_continuation_mask))
                        } != 0_i32 {
                            if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = true; }

                            if continuation {
                                unsafe { (_mm256_and_si256(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let potential_high_surrogates_mask: __m256i = shift_right(following_surrogate_mask, zero_right);

                            if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {

                                let high_surrogates_mask: __m256i = unsafe { _mm256_and_si256(any_surrogate_mask, _mm256_andnot_si256(following_surrogate_mask, and_not_mask)) };

                                let potential_following_surrogates_mask: __m256i = shift_left(high_surrogates_mask, zero_left);

                                if unsafe { _mm256_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                else if unsafe { _mm256_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm256_movemask_epi8(high_surrogates_mask) } {return false; }
                                else if unsafe {_mm256_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm256_movemask_epi8(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        } else {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                } else {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                };

                let any_surrogate_mask: __m256i = unsafe { _mm256_cmpeq_epi16(_mm256_and_si256(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 {
                    let following_surrogate_mask: __m256i = unsafe { _mm256_cmpeq_epi16(_mm256_and_si256(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__m256i, __m256i) = if unsafe {
                                _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_i32 {
                                if unsafe { _mm256_movemask_epi8(_mm256_and_si256(any_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = false; }
                                else if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_continuation_mask)) } != 0_i32 { continuation = false; }

                                if continuation {
                                    (shift_right(unsafe { _mm256_and_si256(any_surrogate_mask, ignore_leading_continuation_mask) }, zero_right), shift_right(following_surrogate_mask, zero_right))
                                } else {
                                    (shift_right(any_surrogate_mask, zero_right), shift_right(following_surrogate_mask, zero_right))
                                }
                            } else {
                                return false;
                            };

                            if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                                let potential_high_surrogates_mask: __m256i = shift_right(following_surrogate_mask, zero_right);

                                if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {
                                    let high_surrogates_mask: __m256i = unsafe { _mm256_and_si256(any_surrogate_mask, _mm256_andnot_si256(following_surrogate_mask, and_not_mask)) };

                                    let potential_following_surrogates_mask: __m256i = shift_left(high_surrogates_mask, zero_left);

                                    if unsafe { _mm256_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                    else if unsafe { _mm256_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm256_movemask_epi8(high_surrogates_mask) } { return false; }
                                    else if unsafe { _mm256_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm256_movemask_epi8(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__m256i, __m256i) = if unsafe {
                            _mm256_movemask_epi8(_mm256_and_si256(any_surrogate_mask, test_continuation_mask))
                        } != 0_i32 {
                            if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = true; }

                            if continuation {
                                unsafe { (_mm256_and_si256(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { _mm256_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let potential_high_surrogates_mask: __m256i = shift_right(following_surrogate_mask, zero_right);

                            if unsafe { _mm256_movemask_epi8(_mm256_and_si256(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {

                                let high_surrogates_mask: __m256i = unsafe { _mm256_and_si256(any_surrogate_mask, _mm256_andnot_si256(following_surrogate_mask, and_not_mask)) };

                                let potential_following_surrogates_mask: __m256i = shift_left(high_surrogates_mask, zero_left);

                                if unsafe { _mm256_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                else if unsafe { _mm256_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm256_movemask_epi8(high_surrogates_mask) } {return false; }
                                else if unsafe {_mm256_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm256_movemask_epi8(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { _mm256_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_utf16_from_byte_array(array: &[u8], endian: bool, omp: bool, only: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut continuation, mut result): (usize, usize, bool, bool) = (0_usize, length % 32_usize, false, true);

        if length == 0_usize { return false; }

        if omp && only {
            if length % UTF16::__ENCODING_SURROGATE_PAIR_BYTES != 0_usize { return false; }
        } else {
            if length % UTF16::__ENCODING_REGULAR_PAIR_BYTES != 0_usize { return false; }
        }

        if indivisible != 0_usize {
            if only {
                if omp {
                    let indivisible_code_array: __m256i = {
                        if endian {
                            let mut indivisible_code_array: [u8; 32_usize] = [
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC,
                                0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC
                            ];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                        } else {
                            let mut indivisible_code_array: [u8; 32_usize] = [
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00,
                                0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00
                            ];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                        }
                    };

                    result &= UTF16::is_utf16_omp_16x16(&[indivisible_code_array], endian);
                } else {
                    let indivisible_code_array: __m256i = {
                        let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_bmp_16x16(&[indivisible_code_array], endian);
                }
            } else {
                if length < 33_usize {
                    let indivisible_code_array: __m256i = {
                        let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_mixed_16x16(&[indivisible_code_array], endian, false);
                } else {
                    let align_indivisible: usize = indivisible + indivisible % 2_usize;

                    let potentially_surrogate_index: usize = if cfg!(target_endian = "big") {
                        if endian { align_indivisible - 2_usize } else { align_indivisible - 1_usize }
                    } else {
                        if endian { align_indivisible - 1_usize } else { align_indivisible - 2_usize }
                    };

                    if (array[potentially_surrogate_index] & 0xFC) != 0xD8 {
                        let indivisible_code_array: __m256i = {
                            let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                            while index < align_indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_mixed_16x16(&[indivisible_code_array], endian, false);
                    } else {
                        continuation = true;

                        if potentially_surrogate_index >= 2_usize {
                            let indivisible_code_array: __m256i = {
                                let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                                while index < align_indivisible - 2_usize { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
                            };

                            result &= UTF16::is_utf16_mixed_16x16(&[indivisible_code_array], endian, false);
                        }
                    }
                }
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                if only {
                    if omp {
                        result &= UTF16::is_utf16_omp_16x16(unsafe { std::slice::from_raw_parts::<__m256i>(transmute::<*const u8, *const __m256i>(array.as_ptr().add(indivisible)), remains_length / 32_usize) }, endian);
                    } else {
                        result &= UTF16::is_utf16_bmp_16x16(unsafe { std::slice::from_raw_parts::<__m256i>(transmute::<*const u8, *const __m256i>(array.as_ptr().add(indivisible)), remains_length / 32_usize) }, endian);
                    }
                } else {
                    result &= UTF16::is_utf16_mixed_16x16(unsafe { std::slice::from_raw_parts::<__m256i>(transmute::<*const u8, *const __m256i>(array.as_ptr().add(indivisible)), remains_length / 32_usize) }, endian, continuation);
                }
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_bmp_16x8(array: &[__m128i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (bad_range_mask, bad_result_mask): (__m128i, __m128i) =
        unsafe { (_mm_set1_epi16(-0x0800), _mm_set1_epi16(-0x2800)) }; // 0xF800, 0xD800

        #[cfg(target_feature = "ssse3")]
        let swap_endian: __m128i = unsafe { _mm_set_epi8(14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1) };

        #[cfg(all(target_feature = "sse2", not(target_feature = "ssse3")))]
        fn swap_endian(value: __m128i) -> __m128i {
            let shift_sides: __m128i = unsafe { _mm_set_epi16(0, 0, 0, 0, 0, 0, 0, 8) };

            let (zero_left, zero_right): (__m128i, __m128i) = unsafe {
                (
                    _mm_set_epi16(0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF),
                    _mm_set_epi16(-0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100) // 0xFF00
                )
            };

            unsafe {
                _mm_or_si128(
                    _mm_and_si128(_mm_sra_epi16(value, shift_sides), zero_left),
                    _mm_and_si128(_mm_sll_epi16(value, shift_sides), zero_right)
                )
            }
        }

        if endian {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                } else {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                };

                if unsafe { _mm_movemask_epi8(_mm_cmpeq_epi16(_mm_and_si128(value, bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                } else {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                };

                if unsafe { _mm_movemask_epi8(_mm_cmpeq_epi16(_mm_and_si128(value, bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_omp_16x8(array: &[__m128i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (max_signed_mask, restricted_less_than_mask, restricted_big_than_mask): (__m128i, __m128i, __m128i) =
            unsafe {
                (
                    _mm_set1_epi16(-0x0001),                                                               // 0xFFFF: -0x0001
                    _mm_set_epi16(-0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800, -0x2400, -0x2800), // 0xD800: -0x2800, 0xDC00: -0x2400
                    _mm_set_epi16(-0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401, -0x2001, -0x2401), // 0xDBFF: -0x2401, 0xDFFF: -0x2001
                )
            };

        #[cfg(target_feature = "ssse3")]
        let swap_endian: __m128i = unsafe { _mm_set_epi8(14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1) };

        #[cfg(all(target_feature = "sse2", not(target_feature = "ssse3")))]
        fn swap_endian(value: __m128i) -> __m128i {
            let shift_sides: __m128i = unsafe { _mm_set_epi16(0, 0, 0, 0, 0, 0, 0, 8) };

            let (zero_left, zero_right): (__m128i, __m128i) = unsafe {
                (
                    _mm_set_epi16(0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF),
                    _mm_set_epi16(-0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100), // 0xFF00
                )
            };

            unsafe {
                _mm_or_si128(
                    _mm_and_si128(_mm_sra_epi16(value, shift_sides), zero_left),
                    _mm_and_si128(_mm_sll_epi16(value, shift_sides), zero_right),
                )
            }
        }

        if endian {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                } else {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                };

                if unsafe { _mm_movemask_epi8(_mm_cmpgt_epi16(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_or_si128(_mm_cmpgt_epi16(restricted_less_than_mask, value), _mm_cmplt_epi16(restricted_big_than_mask, value))) } != 0_i32 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                } else {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                };

                if unsafe { _mm_movemask_epi8(_mm_cmpgt_epi16(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_or_si128(_mm_cmpgt_epi16(restricted_less_than_mask, value), _mm_cmplt_epi16(restricted_big_than_mask, value))) } != 0_i32 { return false; }


                index += 1_usize;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf16_mixed_16x8(array: &[__m128i], endian: bool, mut continuation: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (any_part_surrogate_detect_mask, following_surrogate_detect_mask): (__m128i, __m128i) = unsafe {
            (_mm_set1_epi16(-0x0800), _mm_set1_epi16(-0x0400)) // 0xF800, 0xFC00
        };

        let (high_surrogate_detect_mask, low_surrogate_detect_mask): (__m128i, __m128i) = unsafe {
            (_mm_set1_epi16(-0x2800), _mm_set1_epi16(-0x2400)) // 0xD800, 0xDC00
        };

        let (test_continuation_mask, test_following_continuation_mask, ignore_leading_continuation_mask): (__m128i, __m128i, __m128i) = unsafe {
            (
                _mm_set_epi16(-0x0001, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000),      // 0xFFFF: -0x0001
                _mm_set_epi16(0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, -0x0001),      // 0xFFFF: -0x0001
                _mm_set_epi16(0x0000, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001, -0x0001) // 0xFFFF: -0x0001
            )
        };

        let and_not_mask: __m128i = unsafe { _mm_set1_epi16(-0x0001) }; // 0xFFFF: -0x0001

        #[cfg(target_feature = "ssse3")]
        let swap_endian: __m128i = unsafe { _mm_set_epi8(14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1) };

        #[cfg(all(target_feature = "sse2", not(target_feature = "ssse3")))]
        fn swap_endian(value: __m128i) -> __m128i {
            let shift_sides: __m128i = unsafe { _mm_set_epi16(0, 0, 0, 0, 0, 0, 0, 8) };

            let (zero_left, zero_right): (__m128i, __m128i) = unsafe {
                (
                    _mm_set_epi16(0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF, 0x00FF),
                    _mm_set_epi16(-0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100, -0x100), // 0xFF00
                )
            };

            unsafe {
                _mm_or_si128(
                    _mm_and_si128(_mm_sra_epi16(value, shift_sides), zero_left),
                    _mm_and_si128(_mm_sll_epi16(value, shift_sides), zero_right),
                )
            }
        }

        if endian {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                } else {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                };

                let any_surrogate_mask: __m128i = unsafe { _mm_cmpeq_epi16(_mm_and_si128(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 {
                    let following_surrogate_mask: __m128i = unsafe { _mm_cmpeq_epi16(_mm_and_si128(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__m128i, __m128i) = if unsafe {
                                _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_i32 {
                                if unsafe { _mm_movemask_epi8(_mm_and_si128(any_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = false; }
                                else if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_continuation_mask)) } != 0_i32 { continuation = false; }

                                if continuation {
                                    unsafe { (_mm_bsrli_si128::<2>(_mm_and_si128(any_surrogate_mask, ignore_leading_continuation_mask)), _mm_bsrli_si128::<2>(following_surrogate_mask)) }
                                } else {
                                    unsafe { (_mm_bsrli_si128::<2>(any_surrogate_mask), _mm_bsrli_si128::<2>(following_surrogate_mask)) }
                                }
                            } else {
                                return false;
                            };

                            if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                                let potential_high_surrogates_mask: __m128i = unsafe { _mm_bsrli_si128::<2>(following_surrogate_mask) };

                                if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {
                                    let high_surrogates_mask: __m128i = unsafe { _mm_and_si128(any_surrogate_mask, _mm_andnot_si128(following_surrogate_mask, and_not_mask)) };

                                    let potential_following_surrogates_mask: __m128i = unsafe { _mm_bslli_si128::<2>(high_surrogates_mask) };

                                    if unsafe { _mm_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                    else if unsafe { _mm_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm_movemask_epi8(high_surrogates_mask) } { return false; }
                                    else if unsafe { _mm_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm_movemask_epi8(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__m128i, __m128i) = if unsafe {
                            _mm_movemask_epi8(_mm_and_si128(any_surrogate_mask, test_continuation_mask))
                        } != 0_i32 {
                            if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = true; }

                            if continuation {
                                unsafe { (_mm_and_si128(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let potential_high_surrogates_mask: __m128i = unsafe { _mm_bsrli_si128::<2>(following_surrogate_mask) };

                            if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {

                                let high_surrogates_mask: __m128i = unsafe { _mm_and_si128(any_surrogate_mask, _mm_andnot_si128(following_surrogate_mask, and_not_mask)) };

                                let potential_following_surrogates_mask: __m128i = unsafe { _mm_bslli_si128::<2>(high_surrogates_mask) };

                                if unsafe { _mm_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                else if unsafe { _mm_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm_movemask_epi8(high_surrogates_mask) } {return false; }
                                else if unsafe {_mm_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm_movemask_epi8(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        } else {
            while index < length {
                let value: __m128i = if cfg!(target_endian = "big") {
                    unsafe { _mm_loadu_si128(black_box(&array[index])) }
                } else {
                    #[cfg(target_feature = "ssse3")]
                    unsafe { _mm_shuffle_epi8(_mm_loadu_si128(black_box(&array[index])), swap_endian) }

                    #[cfg(all(target_feature = "sse", target_feature = "sse2", not(target_feature = "ssse3")))]
                    unsafe { swap_endian(_mm_loadu_si128(black_box(&array[index]))) }
                };

                let any_surrogate_mask: __m128i = unsafe { _mm_cmpeq_epi16(_mm_and_si128(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 {
                    let following_surrogate_mask: __m128i = unsafe { _mm_cmpeq_epi16(_mm_and_si128(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let (any_surrogate_mask, following_surrogate_mask): (__m128i, __m128i) = if unsafe {
                                _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_i32 {
                                if unsafe { _mm_movemask_epi8(_mm_and_si128(any_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = false; }
                                else if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_continuation_mask)) } != 0_i32 { continuation = false; }

                                if continuation {
                                    unsafe { (_mm_bsrli_si128::<2>(_mm_and_si128(any_surrogate_mask, ignore_leading_continuation_mask)), _mm_bsrli_si128::<2>(following_surrogate_mask)) }
                                } else {
                                    unsafe { (_mm_bsrli_si128::<2>(any_surrogate_mask), _mm_bsrli_si128::<2>(following_surrogate_mask)) }
                                }
                            } else {
                                return false;
                            };

                            if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                                let potential_high_surrogates_mask: __m128i = unsafe { _mm_bsrli_si128::<2>(following_surrogate_mask) };

                                if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {
                                    let high_surrogates_mask: __m128i = unsafe { _mm_and_si128(any_surrogate_mask, _mm_andnot_si128(following_surrogate_mask, and_not_mask)) };

                                    let potential_following_surrogates_mask: __m128i = unsafe { _mm_bslli_si128::<2>(high_surrogates_mask) };

                                    if unsafe { _mm_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                    else if unsafe { _mm_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm_movemask_epi8(high_surrogates_mask) } { return false; }
                                    else if unsafe { _mm_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm_movemask_epi8(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (__m128i, __m128i) = if unsafe {
                            _mm_movemask_epi8(_mm_and_si128(any_surrogate_mask, test_continuation_mask))
                        } != 0_i32 {
                            if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, test_continuation_mask)) } == 0_i32 { continuation = true; }

                            if continuation {
                                unsafe { (_mm_and_si128(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { _mm_movemask_epi8(following_surrogate_mask) } != 0_i32 {
                            let potential_high_surrogates_mask: __m128i = unsafe { _mm_bsrli_si128::<2>(following_surrogate_mask) };

                            if unsafe { _mm_movemask_epi8(_mm_and_si128(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_i32 {

                                let high_surrogates_mask: __m128i = unsafe { _mm_and_si128(any_surrogate_mask, _mm_andnot_si128(following_surrogate_mask, and_not_mask)) };

                                let potential_following_surrogates_mask: __m128i = unsafe { _mm_bslli_si128::<2>(high_surrogates_mask) };

                                if unsafe { _mm_movemask_epi8(high_surrogates_mask) } == 0_i32 { return false; }
                                else if unsafe { _mm_movemask_epi8(potential_high_surrogates_mask) } != unsafe { _mm_movemask_epi8(high_surrogates_mask) } {return false; }
                                else if unsafe {_mm_movemask_epi8(potential_following_surrogates_mask) } != unsafe { _mm_movemask_epi8(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { _mm_movemask_epi8(any_surrogate_mask) } != 0_i32 { return false; }
                        }
                    }
                } else {
                    if continuation { return false; }
                }

                index += 1_usize;
            }

            if continuation { return false; }
        }

        return true;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_utf16_from_byte_array(array: &[u8], endian: bool, omp: bool, only: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut continuation, mut result): (usize, usize, bool, bool) = (0_usize, length % 16_usize, false, true);

        if length == 0_usize { return false; }

        if omp && only {
            if length % UTF16::__ENCODING_SURROGATE_PAIR_BYTES != 0_usize { return false; }
        } else {
            if length % UTF16::__ENCODING_REGULAR_PAIR_BYTES != 0_usize { return false; }
        }

        if indivisible != 0_usize {
            if only {
                if omp {
                    let indivisible_code_array: __m128i = {
                        if endian {
                            let mut indivisible_code_array: [u8; 16_usize] = [0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                        } else {
                            let mut indivisible_code_array: [u8; 16_usize] = [0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00];

                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                        }
                    };

                    result &= UTF16::is_utf16_omp_16x8(&[indivisible_code_array], endian);
                } else {
                    let indivisible_code_array: __m128i = {
                        let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_bmp_16x8(&[indivisible_code_array], endian);
                }
            } else {
                if length < 17_usize {
                    let indivisible_code_array: __m128i = {
                        let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                    };

                    result &= UTF16::is_utf16_mixed_16x8(&[indivisible_code_array], endian, false);
                } else {
                    let align_indivisible: usize = indivisible + indivisible % 2_usize;

                    let potentially_surrogate_index: usize = if cfg!(target_endian = "big") {
                        if endian { align_indivisible - 2_usize } else { align_indivisible - 1_usize }
                    } else {
                        if endian { align_indivisible - 1_usize } else { align_indivisible - 2_usize }
                    };

                    if (array[potentially_surrogate_index] & 0xFC) != 0xD8 {
                        let indivisible_code_array: __m128i = {
                            let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                            while index < align_indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_mixed_16x8(&[indivisible_code_array], endian, false);
                    } else {
                        continuation = true;

                        if potentially_surrogate_index >= 2_usize {
                            let indivisible_code_array: __m128i = {
                                let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                                while index < align_indivisible - 2_usize { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
                            };

                            result &= UTF16::is_utf16_mixed_16x8(&[indivisible_code_array], endian, false);
                        }
                    }
                }
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                if only {
                    if omp {
                        result &= UTF16::is_utf16_omp_16x8(unsafe { std::slice::from_raw_parts::<__m128i>(transmute::<*const u8, *const __m128i>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian);
                    } else {
                        result &= UTF16::is_utf16_bmp_16x8(unsafe { std::slice::from_raw_parts::<__m128i>(transmute::<*const u8, *const __m128i>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian);
                    }
                } else {
                    result &= UTF16::is_utf16_mixed_16x8(unsafe { std::slice::from_raw_parts::<__m128i>(transmute::<*const u8, *const __m128i>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian, continuation);
                }
            }
        }

        return result;
    }
}
