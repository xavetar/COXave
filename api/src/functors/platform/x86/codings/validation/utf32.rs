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
        UTF32
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
            _mm512_loadu_si512,
            _mm512_set1_epi32,
            _mm512_and_si512,
            _mm512_set_epi8,
            _mm512_shuffle_epi8,
            _mm512_cmplt_epi32_mask,
            _mm512_cmpgt_epi32_mask,
            _mm512_cmpeq_epi32_mask,
            _mm512_maskz_mov_epi32
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
            _mm256_set1_epi32,
            _mm256_and_si256,
            _mm256_movemask_epi8,
            _mm256_set_epi8,
            _mm256_shuffle_epi8,
            _mm256_cmpgt_epi32,
            _mm256_cmpeq_epi32,
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
            _mm_set1_epi32,
            _mm_cmplt_epi32,
            _mm_cmpgt_epi32,
            _mm_and_si128,
            _mm_cmpeq_epi32,
            _mm_movemask_epi8,
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
            _mm_set_epi32,
            _mm_sra_epi32,
            _mm_sll_epi32,
            _mm_or_si128
        }
    }
};

impl UTF32 {

    const __ENCODING_BYTES: usize = 4_usize;

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    fn is_utf32_32x16(array: &[__m512i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (max_signed_mask, max_unsigned_mask, range_mask, bad_range_mask, bad_result_mask, mask_to_vector): (__m512i, __m512i, __m512i, __m512i, __m512i, __m512i) =
        unsafe {
            (
                _mm512_set1_epi32(0x00000000), _mm512_set1_epi32(0x0010FFFF),
                _mm512_set1_epi32(0x00010000), _mm512_set1_epi32(0x0000F800),
                _mm512_set1_epi32(0x0000D800), _mm512_set1_epi32(-0x00000001)
            )
        };

        let swap_endian: __m512i = unsafe { _mm512_set_epi8(
            60, 61, 62, 63, 56, 57, 58, 59, 52, 53, 54, 55, 48, 49, 50, 51,
            44, 45, 46, 47, 40, 41, 42, 43, 36, 37, 38, 39, 32, 33, 34, 35,
            28, 29, 30, 31, 24, 25, 26, 27, 20, 21, 22, 23, 16, 17, 18, 19,
            12, 13, 14, 15,  8,  9, 10, 11,  4,  5,  6,  7,  0,  1,  2,  3
        ) };

        if endian {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                } else {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                };

                if unsafe { _mm512_cmplt_epi32_mask(value, max_signed_mask) } != 0_u16 { return false; }
                else if unsafe { _mm512_cmpgt_epi32_mask(value, max_unsigned_mask) } != 0_u16 { return false; }
                else if unsafe { _mm512_cmpeq_epi32_mask(_mm512_and_si512(_mm512_and_si512(value, _mm512_maskz_mov_epi32(_mm512_cmplt_epi32_mask(value, range_mask), mask_to_vector)), bad_range_mask), bad_result_mask) } > 0_u16 { return false; }

                index += 1;
            }
        } else {
            while index < length {
                let value: __m512i = if cfg!(target_endian = "big") {
                    unsafe { _mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)) }
                } else {
                    unsafe { _mm512_shuffle_epi8(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), swap_endian) }
                };

                if unsafe { _mm512_cmplt_epi32_mask(value, max_signed_mask) } != 0_u16 { return false; }
                else if unsafe { _mm512_cmpgt_epi32_mask(value, max_unsigned_mask) } != 0_u16 { return false; }
                else if unsafe { _mm512_cmpeq_epi32_mask(_mm512_and_si512(_mm512_and_si512(value, _mm512_maskz_mov_epi32(_mm512_cmplt_epi32_mask(value, range_mask), mask_to_vector)), bad_range_mask), bad_result_mask) } > 0_u16 { return false; }

                index += 1;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    pub fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 64_usize, true);

        if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { return false; }

        if indivisible != 0_usize {
            let indivisible_code_array: __m512i = {
                let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
            };

            result &= UTF32::is_utf32_32x16(&[indivisible_code_array], endian);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= UTF32::is_utf32_32x16(unsafe { std::slice::from_raw_parts::<__m512i>(transmute::<*const u8, *const __m512i>(array.as_ptr().add(indivisible)), remains_length / 64_usize) }, endian);
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf32_32x8(array: &[__m256i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (min_mask, max_unsigned_mask, range_mask, bad_range_mask, bad_result_mask): (__m256i, __m256i, __m256i, __m256i, __m256i) =
        unsafe { (_mm256_set1_epi32(0x0000000), _mm256_set1_epi32(0x0010FFFF), _mm256_set1_epi32(0x0000FFFF), _mm256_set1_epi32(0x0000F800), _mm256_set1_epi32(0x0000D800)) };

        let swap_endian: __m256i = unsafe { _mm256_set_epi8(
            28, 29, 30, 31, 24, 25, 26, 27, 20, 21, 22, 23, 16, 17, 18, 19,
            12, 13, 14, 15,  8,  9, 10, 11,  4,  5,  6,  7,  0,  1,  2,  3
        ) };

        if endian {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                } else {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi32(min_mask, value)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi32(value, max_unsigned_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi32(_mm256_and_si256(_mm256_and_si256(value, _mm256_cmpgt_epi32(range_mask, value)), bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1;
            }
        } else {
            while index < length {
                let value: __m256i = if cfg!(target_endian = "big") {
                    unsafe { _mm256_loadu_si256(black_box(&array[index])) }
                } else {
                    unsafe { _mm256_shuffle_epi8(_mm256_loadu_si256(black_box(&array[index])), swap_endian) }
                };

                if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi32(min_mask, value)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_cmpgt_epi32(value, max_unsigned_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm256_movemask_epi8(_mm256_cmpeq_epi32(_mm256_and_si256(_mm256_and_si256(value, _mm256_cmpgt_epi32(range_mask, value)), bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {
        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 32_usize, true);

        if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { return false; }

        if indivisible != 0_usize {
            let indivisible_code_array: __m256i = {
                let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
            };

            result &= UTF32::is_utf32_32x8(&[indivisible_code_array], endian);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= UTF32::is_utf32_32x8(unsafe { std::slice::from_raw_parts::<__m256i>(transmute::<*const u8, *const __m256i>(array.as_ptr().add(indivisible)), remains_length / 32_usize) }, endian);
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_utf32_32x4(array: &[__m128i], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (max_signed_mask, max_unsigned_mask, range_mask, bad_range_mask, bad_result_mask): (__m128i, __m128i, __m128i, __m128i, __m128i) =
        unsafe { (_mm_set1_epi32(0x00000000), _mm_set1_epi32(0x0010FFFF), _mm_set1_epi32(0x00010000), _mm_set1_epi32(0x0000F800), _mm_set1_epi32(0x0000D800)) };

        #[cfg(target_feature = "ssse3")]
        let swap_endian: __m128i = unsafe { _mm_set_epi8(12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3) };

        #[cfg(all(target_feature = "sse2", not(target_feature = "ssse3")))]
        fn swap_endian(value: __m128i) -> __m128i {
            let (shift_sides, shift_middle): (__m128i, __m128i) = unsafe {
                (_mm_set_epi32(0, 0, 0, 24), _mm_set_epi32(0, 0, 0, 8))
            };

            let (zero_left, zero_right, zero_left_side, zero_right_side): (__m128i, __m128i, __m128i, __m128i) = unsafe {
                (
                    _mm_set_epi32(0x000000FF, 0x000000FF, 0x000000FF, 0x000000FF),
                    _mm_set_epi32(-0x1000000, -0x1000000, -0x1000000, -0x1000000), // 0xFF000000
                    _mm_set_epi32(0x0000FF00, 0x0000FF00, 0x0000FF00, 0x0000FF00),
                    _mm_set_epi32(0x00FF0000, 0x00FF0000, 0x00FF0000, 0x00FF0000)
                )
            };

            unsafe {
                _mm_or_si128(
                    _mm_or_si128(
                        _mm_and_si128(_mm_sra_epi32(value, shift_middle), zero_left_side),
                        _mm_and_si128(_mm_sll_epi32(value, shift_middle), zero_right_side)
                    ),
                    _mm_or_si128(
                        _mm_and_si128(_mm_sra_epi32(value, shift_sides), zero_left),
                        _mm_and_si128(_mm_sll_epi32(value, shift_sides), zero_right)
                    )
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

                if unsafe { _mm_movemask_epi8(_mm_cmplt_epi32(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_cmpgt_epi32(value, max_unsigned_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_cmpeq_epi32(_mm_and_si128(_mm_and_si128(value, _mm_cmplt_epi32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1;
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

                if unsafe { _mm_movemask_epi8(_mm_cmplt_epi32(value, max_signed_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_cmpgt_epi32(value, max_unsigned_mask)) } != 0_i32 { return false; }
                else if unsafe { _mm_movemask_epi8(_mm_cmpeq_epi32(_mm_and_si128(_mm_and_si128(value, _mm_cmplt_epi32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_i32 { return false; }

                index += 1;
            }
        }

        return true;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 16_usize, true);

        if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { return false; }

        if indivisible != 0_usize {
            let indivisible_code_array: __m128i = {
                let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
            };

            result &= UTF32::is_utf32_32x4(&[indivisible_code_array], endian);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= UTF32::is_utf32_32x4(unsafe { std::slice::from_raw_parts::<__m128i>(transmute::<*const u8, *const __m128i>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian);
            }
        }

        return result;
    }
}