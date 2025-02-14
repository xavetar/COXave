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
        ASCII
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
            _mm512_set1_epi8,
            _mm512_and_si512,
            _mm512_movepi8_mask
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
            _mm256_set1_epi8,
            _mm256_and_si256,
            _mm256_movemask_epi8
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
            _mm_set1_epi8,
            _mm_and_si128,
            _mm_movemask_epi8
        }
    }
};

impl ASCII {

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    fn is_ascii_8x64(array: &[__m512i]) -> bool {
        let (mut index, length, mask): (usize, usize, __m512i) = (0_usize, array.len(), unsafe { _mm512_set1_epi8(-0x80) });

        while index < length { if unsafe { _mm512_movepi8_mask(_mm512_and_si512(_mm512_loadu_si512(black_box(&array[index] as *const __m512i as *const i32)), mask)) } != 0_u64 { return false; } else { index += 1_usize; } }

        return true;
    }

    #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
    pub fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        if length == 0_usize { return false; }

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 64_usize, true);

        if indivisible != 0_usize {
            let indivisible_code_array: __m512i = {
                let mut indivisible_code_array: [u8; 64_usize] = [0_u8; 64_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 64_usize], __m512i>(indivisible_code_array) }
            };

            result &= ASCII::is_ascii_8x64(&[indivisible_code_array]);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= ASCII::is_ascii_8x64(unsafe { std::slice::from_raw_parts::<__m512i>(transmute::<*const u8, *const __m512i>(array.as_ptr().add(indivisible)), remains_length / 64_usize) });
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_ascii_8x32(array: &[__m256i]) -> bool {
        let (mut index, length, mask): (usize, usize, __m256i) = (0_usize, array.len(), unsafe { _mm256_set1_epi8(-0x80) });

        while index < length { if unsafe { _mm256_movemask_epi8(_mm256_and_si256(_mm256_loadu_si256(black_box(&array[index])), mask)) } != 0_i32 { return false; } else { index += 1_usize; } }

        return true;
    }

    #[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        if length == 0_usize { return false; }

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 32_usize, true);

        if indivisible != 0_usize {
            let indivisible_code_array: __m256i = {
                let mut indivisible_code_array: [u8; 32_usize] = [0_u8; 32_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 32_usize], __m256i>(indivisible_code_array) }
            };

            result &= ASCII::is_ascii_8x32(&[indivisible_code_array]);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= ASCII::is_ascii_8x32(unsafe { std::slice::from_raw_parts::<__m256i>(transmute::<*const u8, *const __m256i>(array.as_ptr().add(indivisible)), remains_length / 32_usize) });
            }
        }

        return result;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    fn is_ascii_8x16(array: &[__m128i]) -> bool {
        let (mut index, length, mask): (usize, usize, __m128i) = (0_usize, array.len(), unsafe { _mm_set1_epi8(-0x80) });

        while index < length { if unsafe { _mm_movemask_epi8(_mm_and_si128(_mm_loadu_si128(black_box(&array[index])), mask)) } != 0_i32 { return false; } else { index += 1_usize; } }

        return true;
    }

    #[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
    pub fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        if length == 0_usize { return false; }

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 16_usize, true);

        if indivisible != 0_usize {
            let indivisible_code_array: __m128i = {
                let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                unsafe { transmute::<[u8; 16_usize], __m128i>(indivisible_code_array) }
            };

            result &= ASCII::is_ascii_8x16(&[indivisible_code_array]);
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= ASCII::is_ascii_8x16(unsafe { std::slice::from_raw_parts::<__m128i>(transmute::<*const u8, *const __m128i>(array.as_ptr().add(indivisible)), remains_length / 16_usize) });
            }
        }

        return result;
    }
}
