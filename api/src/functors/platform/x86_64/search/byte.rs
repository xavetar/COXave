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
        ByteSearch
    }
};

#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
use std::{
    mem::{
        transmute,
    },
    arch::{
        x86_64::{
            __m512i,
            __mmask16,
            __mmask32,
            __mmask64,
            _mm512_loadu_si512,
            _mm512_set1_epi8, _mm512_set1_epi16, _mm512_set1_epi32,
            _mm512_and_si512,
            _mm512_andnot_si512,
            _mm512_cmpeq_epi8_mask, _mm512_cmpeq_epi16_mask, _mm512_cmpeq_epi32_mask,
            _mm512_storeu_si512,
            _mm512_maskz_mov_epi8, _mm512_maskz_mov_epi16, _mm512_maskz_mov_epi32,
            _mm512_movepi8_mask, _mm512_movepi16_mask, _mm512_movepi32_mask
        }
    }
};

#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    mem::{
        transmute,
    },
    arch::{
        x86_64::{
            __m256i,
            _mm256_loadu_si256,
            _mm256_set1_epi8, _mm256_set1_epi16, _mm256_set1_epi32,
            _mm256_and_si256,
            _mm256_andnot_si256,
            _mm256_cmpeq_epi8, _mm256_cmpeq_epi16, _mm256_cmpeq_epi32,
            _mm256_storeu_si256,
            _mm256_movemask_epi8
        }
    }
};

#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
use std::{
    mem::{
        transmute,
    },
    arch::{
        x86_64::{
            __m128i,
            _mm_loadu_si128,
            _mm_set1_epi8, _mm_set1_epi16, _mm_set1_epi32,
            _mm_and_si128,
            _mm_andnot_si128,
            _mm_cmpeq_epi8, _mm_cmpeq_epi16, _mm_cmpeq_epi32,
            _mm_storeu_si128,
            _mm_movemask_epi8
        }
    }
};

#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
macro_rules! generate_search {
    ($t:ty, $t_default:expr, $not_ignore_mask:expr, $t_size:expr, $precision:ty, $register_size:expr, $mask:ty, $load:expr, $store:expr, $dup_one_t:expr, $eq_compare:expr, $bitwise_and:expr, $bitwise_not_and:expr, $vector_to_scalar:expr, $mask_to_vector:expr) => {
        impl ByteSearch<$t> {

            const fn is_search_possible(array: &[$t], pattern: &[$t], limit: Option<usize>) -> (usize, usize, bool) {
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

            pub fn search_single(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, mask_to_vector_mask, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, $precision, $mask) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, unsafe { $dup_one_t($not_ignore_mask) }, 0);

                let (load_match_ptr, write_match_ptr): ([*const i32; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const i32>(matches[0].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[1].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[2].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[3], $mask_to_vector($eq_compare(four, pattern_mask), mask_to_vector_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[3], $bitwise_and($mask_to_vector($eq_compare(s_four, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_four, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, mask_to_vector_mask, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, $precision, $mask) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, unsafe { $dup_one_t($not_ignore_mask) }, 0);

                let (load_match_ptr, write_match_ptr): ([*const i32; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const i32>(matches[0].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[1].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[2].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[3], $mask_to_vector($eq_compare(four, pattern_mask), mask_to_vector_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (mut next_pass, start_pattern_mask, end_pattern_mask): (bool, $precision, $precision) = (false, $dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[3], $bitwise_and($mask_to_vector($eq_compare(s_four, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_four, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..4 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..3 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..2 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                if next_pass { index_of_match += 1_usize; next_pass = false; }
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                }
                                if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                            if next_pass { index_of_match += 1_usize; }
                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        let (next_area, mut next_index_of_match): (usize, usize) = (pattern_length - (pattern_length % COUNT_OF_VALUES_IN_REGISTER), 0_usize);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                        let index: usize = array_index + index_of_match;
                                                        search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += next_area; array_ptr = array_ptr.add(next_area);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size);

                                                    if index_of_match + pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                                        next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    } else { index_of_match += pattern_length; }
                                                } else { index_of_match += 1_usize; }
                                            } else { index_of_match += 1_usize; }
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); break;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); break;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); index_of_match += pattern_length;
                                            } else { index_of_match += 1_usize; }
                                        } else { index_of_match += 1_usize; }
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all_overlapping(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, mask_to_vector_mask, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, $precision, $mask) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, unsafe { $dup_one_t($not_ignore_mask) }, 0);

                let (load_match_ptr, write_match_ptr): ([*const i32; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const i32>(matches[0].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[1].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[2].as_ptr()),
                            transmute::<*const $t, *const i32>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[3], $mask_to_vector($eq_compare(four, pattern_mask), mask_to_vector_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[2], $mask_to_vector($eq_compare(third, pattern_mask), mask_to_vector_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));
                                $store(write_match_ptr[1], $mask_to_vector($eq_compare(second, pattern_mask), mask_to_vector_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));

                                $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $mask_to_vector($eq_compare(first, pattern_mask), mask_to_vector_mask));

                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[3], $bitwise_and($mask_to_vector($eq_compare(s_four, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_four, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[2], $bitwise_and($mask_to_vector($eq_compare(s_third, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_third, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));
                                $store(write_match_ptr[1], $bitwise_and($mask_to_vector($eq_compare(s_second, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_second, end_pattern_mask), mask_to_vector_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(load_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const i32>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                            if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                        $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size);
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const i32>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($mask_to_vector($eq_compare(s_first, start_pattern_mask), mask_to_vector_mask), $mask_to_vector($eq_compare(e_first, end_pattern_mask), mask_to_vector_mask)));

                                    if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                    $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const i32>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const i32>(pattern.as_ptr()));

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const i32>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const i32>(ignore_mask.as_ptr())))
                                };

                                if $vector_to_scalar($load(load_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($mask_to_vector($eq_compare(
                                                $load(transmute::<*const $t, *const i32>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), mask_to_vector_mask), and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }
        }
    };
}

#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
macro_rules! generate_search {
    ($t:ty, $t_default:expr, $not_ignore_mask:expr, $t_size:expr, $precision:ty, $register_size:expr, $load:expr, $store:expr, $dup_one_t:expr, $eq_compare:expr, $bitwise_and:expr, $bitwise_not_and:expr, $vector_to_scalar:expr) => {
        impl ByteSearch<$t> {

            const fn is_search_possible(array: &[$t], pattern: &[$t], limit: Option<usize>) -> (usize, usize, bool) {
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

            pub fn search_single(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, i32) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_i32);

                let (read_match_ptr, write_match_ptr): ([*const $precision; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const $precision>(matches[0].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[1].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[2].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, i32) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_i32);

                let (read_match_ptr, write_match_ptr): ([*const $precision; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const $precision>(matches[0].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[1].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[2].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (mut next_pass, start_pattern_mask, end_pattern_mask): (bool, $precision, $precision) = (false, $dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if next_pass { index_of_match += 1_usize; next_pass = false; }
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                }
                                if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if next_pass { index_of_match += 1_usize; }
                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        let (next_area, mut next_index_of_match): (usize, usize) = (pattern_length - (pattern_length % COUNT_OF_VALUES_IN_REGISTER), 0_usize);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        let index: usize = array_index + index_of_match;
                                                        search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += next_area; array_ptr = array_ptr.add(next_area);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size);

                                                    if index_of_match + pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                                        next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    } else { index_of_match += pattern_length; }
                                                } else { index_of_match += 1_usize; }
                                            } else { index_of_match += 1_usize; }
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); break;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); break;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); index_of_match += pattern_length;
                                            } else { index_of_match += 1_usize; }
                                        } else { index_of_match += 1_usize; }
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all_overlapping(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, i32) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_i32);

                let (read_match_ptr, write_match_ptr): ([*const $precision; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const $precision>(matches[0].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[1].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[2].as_ptr()),
                            transmute::<*const $t, *const $precision>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(read_match_ptr[i])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size);
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if $vector_to_scalar($load(read_match_ptr[0])) != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }
        }
    };
}

#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
macro_rules! generate_search {
    ($t:ty, $t_default:expr, $not_ignore_mask:expr, $t_size:expr, $precision:ty, $register_size:expr, $load:expr, $store:expr, $dup_one_t:expr, $eq_compare:expr, $bitwise_and:expr, $bitwise_not_and:expr, $vector_to_scalar:expr) => {
        impl ByteSearch<$t> {

            const fn is_search_possible(array: &[$t], pattern: &[$t], limit: Option<usize>) -> (usize, usize, bool) {
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

            pub fn search_single(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, u128) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_u128);

                let (read_match_ptr, write_match_ptr): ([*const u128; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const u128>(matches[0].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[1].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[2].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if *read_match_ptr[0] != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if *read_match_ptr[0] != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); return search_result; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, u128) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_u128);

                let (read_match_ptr, write_match_ptr): ([*const u128; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const u128>(matches[0].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[1].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[2].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if *read_match_ptr[0] != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (mut next_pass, start_pattern_mask, end_pattern_mask): (bool, $precision, $precision) = (false, $dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                    }
                                    if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if next_pass { index_of_match += 1_usize; next_pass = false; }
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                }
                                if index_of_match > COUNT_OF_VALUES_IN_REGISTER { next_pass = true; }; index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if next_pass { index_of_match += 1_usize; }
                            if *read_match_ptr[0] != zero {
                                while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 2_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        let (next_area, mut next_index_of_match): (usize, usize) = (pattern_length - (pattern_length % COUNT_OF_VALUES_IN_REGISTER), 0_usize);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        let index: usize = array_index + index_of_match;
                                                        search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += next_area; array_ptr = array_ptr.add(next_area);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size); next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    let index: usize = array_index + index_of_match;
                                                    search_result.push(index * $t_size);

                                                    if index_of_match + pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                                        next_index_of_match = (index + pattern_length) % COUNT_OF_VALUES_IN_REGISTER; break;
                                                    } else { index_of_match += pattern_length; }
                                                } else { index_of_match += 1_usize; }
                                            } else { index_of_match += 1_usize; }
                                        }
                                    }
                                    index_of_match = 0_usize; array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); break;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); break;
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size); index_of_match += pattern_length;
                                            } else { index_of_match += 1_usize; }
                                        } else { index_of_match += 1_usize; }
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }

            pub fn search_all_overlapping(array_ptr: &[u8], pattern_ptr: &[u8], limit: Option<usize>) -> Vec<usize>
            where $t: Copy + Sized {

                const COUNT_OF_VALUES_IN_REGISTER: usize = $register_size / $t_size;
                const PRECISION_LENGTH: [usize; 4_usize] = [
                    (4_usize * $register_size) / $t_size,
                    (3_usize * $register_size) / $t_size,
                    (2_usize * $register_size) / $t_size,
                    (1_usize * $register_size) / $t_size
                ];

                let mut search_result: Vec<usize> = Vec::<usize>::new();

                let (array, pattern): (&[$t], &[$t]) = (
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { std::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let (mut matches, and_not_mask, zero): ([[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], $precision, u128) = ([[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize], unsafe { $dup_one_t($not_ignore_mask) }, 0_u128);

                let (read_match_ptr, write_match_ptr): ([*const u128; 4_usize], [*mut $precision; 4_usize]) = unsafe {
                    (
                        [
                            transmute::<*const $t, *const u128>(matches[0].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[1].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[2].as_ptr()),
                            transmute::<*const $t, *const u128>(matches[3].as_ptr())
                        ],
                        [
                            transmute::<*mut $t, *mut $precision>(matches[0].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[1].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[2].as_mut_ptr()),
                            transmute::<*mut $t, *mut $precision>(matches[3].as_mut_ptr())
                        ]
                    )
                };

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));
                                $store(write_match_ptr[3], $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] {
                                let (first, second, third): ($precision, $precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));
                                $store(write_match_ptr[2], $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] {
                                let (first, second): ($precision, $precision) = {
                                    let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr)); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));
                                $store(write_match_ptr[1], $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] {
                                let first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));

                                $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER] = [$t_default; COUNT_OF_VALUES_IN_REGISTER];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));

                            $store(write_match_ptr[0], $eq_compare(first, pattern_mask));

                            if *read_match_ptr[0] != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length == 2_usize {

                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[1]));

                        if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                            while remains_length >= PRECISION_LENGTH[0] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four): ($precision, $precision, $precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_four: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(write_match_ptr[3], $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[0];
                            }

                            if remains_length >= PRECISION_LENGTH[1] + 1_usize {
                                let (s_first, e_first, s_second, e_second, s_third, e_third): ($precision, $precision, $precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_third: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(write_match_ptr[2], $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[1];
                            }

                            if remains_length >= PRECISION_LENGTH[2] + 1_usize {
                                let (s_first, e_first, s_second, e_second): ($precision, $precision, $precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_second: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(write_match_ptr[1], $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if *read_match_ptr[i] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[i][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                            else { index_of_match += 1_usize; }
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER;
                                }

                                remains_length -= PRECISION_LENGTH[2];
                            }

                            if remains_length >= PRECISION_LENGTH[3] + 1_usize {
                                let (s_first, e_first): ($precision, $precision) = {
                                    let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                    let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                        else { index_of_match += 1_usize; }
                                    }
                                    index_of_match = 0_usize;
                                }
                                array_index += COUNT_OF_VALUES_IN_REGISTER;

                                remains_length -= PRECISION_LENGTH[3];
                            }
                        }

                        if remains_length > 0_usize {
                            let aligned_array: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = {
                                let mut indivisible_part: [$t; COUNT_OF_VALUES_IN_REGISTER + 1_usize] = [$t_default; COUNT_OF_VALUES_IN_REGISTER + 1_usize];
                                for i in 0..remains_length { indivisible_part[i] = array[array_index + i]; }
                                indivisible_part
                            };

                            let (s_first, e_first): ($precision, $precision) = {
                                let s_first: $precision = $load(transmute::<*const $t, *const $precision>(aligned_array.as_ptr()));
                                let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(1_usize)));

                                (s_first, e_first)
                            };

                            $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if *read_match_ptr[0] != zero {
                                while index_of_match < remains_length {
                                    if matches[0][index_of_match] != 0 { search_result.push((array_index + index_of_match) * $t_size); index_of_match += 1_usize; }
                                    else { index_of_match += 1_usize; }
                                }
                            }
                        }
                    } else if pattern_length >= 3_usize {
                        let (last_pattern_index, middle_pattern_length): (usize, usize) = (pattern_length - 1_usize, pattern_length - 2_usize);
                        let middle_pattern_parts: usize = (middle_pattern_length + (COUNT_OF_VALUES_IN_REGISTER - 1_usize)) / COUNT_OF_VALUES_IN_REGISTER;
                        let (start_pattern_mask, end_pattern_mask): ($precision, $precision) = ($dup_one_t(pattern[0]), $dup_one_t(pattern[last_pattern_index]));
                        let last_array_index_multiple_of_register: usize = array_length - (array_length % COUNT_OF_VALUES_IN_REGISTER);

                        if last_array_index_multiple_of_register != 0_usize {
                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                        $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                        $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                    ), and_not_mask)) == 0 {
                                                        search_result.push((array_index + index_of_match) * $t_size);
                                                    }
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                    pattern_loaded
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr));
                                        let e_first: $precision = $load(transmute::<*const $t, *const $precision>(array_ptr.add(last_pattern_index)));

                                        (s_first, e_first)
                                    };

                                    $store(write_match_ptr[0], $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if *read_match_ptr[0] != zero {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                                and_not_mask), pattern_ignore_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                            index_of_match += 1_usize;
                                        }
                                        index_of_match = 0_usize;
                                    }
                                    array_index += COUNT_OF_VALUES_IN_REGISTER; array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                }
                            }
                        }

                        if array_index <= array_length {
                            matches[0] = std::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                    $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))),
                                                    $load(transmute::<*const $t, *const $precision>(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)))
                                                ), and_not_mask)) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(transmute::<*const $t, *const $precision>(pattern.as_ptr()));

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))),
                                                pattern_loaded
                                            ), and_not_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length < COUNT_OF_VALUES_IN_REGISTER {
                                let (pattern_loaded, pattern_ignore_mask): ($precision, $precision) = {
                                    let (mut pattern_array, mut ignore_mask): ([$t; COUNT_OF_VALUES_IN_REGISTER], [$t; COUNT_OF_VALUES_IN_REGISTER]) = ([$t_default; COUNT_OF_VALUES_IN_REGISTER], [$t_default; COUNT_OF_VALUES_IN_REGISTER]);
                                    for i in 0_usize..pattern_length { pattern_array[i] = pattern[i]; ignore_mask[i] = $not_ignore_mask; }
                                    ($load(transmute::<*const $t, *const $precision>(pattern_array.as_ptr())), $load(transmute::<*const $t, *const $precision>(ignore_mask.as_ptr())))
                                };

                                if *read_match_ptr[0] != zero {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not_and($eq_compare(
                                                $load(transmute::<*const $t, *const $precision>(array_ptr.add(index_of_match))), pattern_loaded),
                                            and_not_mask), pattern_ignore_mask)) == 0 {
                                                search_result.push((array_index + index_of_match) * $t_size);
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            }
                        }
                    }
                }

                return search_result;
            }
        }
    };
}

#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
generate_search!(i8, 0_i8, -0x01, size_of::<i8>(), __m512i, size_of::<__m512i>(), __mmask64, _mm512_loadu_si512, _mm512_storeu_si512, _mm512_set1_epi8, _mm512_cmpeq_epi8_mask, _mm512_and_si512, _mm512_andnot_si512, _mm512_movepi8_mask, _mm512_maskz_mov_epi8);
#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
generate_search!(i16, 0_i16, -0x0001, size_of::<i16>(), __m512i, size_of::<__m512i>(), __mmask32, _mm512_loadu_si512, _mm512_storeu_si512, _mm512_set1_epi16, _mm512_cmpeq_epi16_mask, _mm512_and_si512, _mm512_andnot_si512, _mm512_movepi16_mask, _mm512_maskz_mov_epi16);
#[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))]
generate_search!(i32, 0_i32, -0x00000001, size_of::<i32>(), __m512i, size_of::<__m512i>(), __mmask16, _mm512_loadu_si512, _mm512_storeu_si512, _mm512_set1_epi32, _mm512_cmpeq_epi32_mask, _mm512_and_si512, _mm512_andnot_si512, _mm512_movepi32_mask, _mm512_maskz_mov_epi32);

#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i8, 0_i8, -0x01, size_of::<i8>(), __m256i, size_of::<__m256i>(), _mm256_loadu_si256, _mm256_storeu_si256, _mm256_set1_epi8, _mm256_cmpeq_epi8, _mm256_and_si256, _mm256_andnot_si256, _mm256_movemask_epi8);
#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i16, 0_i16, -0x0001, size_of::<i16>(), __m256i, size_of::<__m256i>(), _mm256_loadu_si256, _mm256_storeu_si256, _mm256_set1_epi16, _mm256_cmpeq_epi16, _mm256_and_si256, _mm256_andnot_si256, _mm256_movemask_epi8);
#[cfg(all(target_feature = "avx", target_feature = "avx2", not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i32, 0_i32, -0x00000001, size_of::<i32>(), __m256i, size_of::<__m256i>(), _mm256_loadu_si256, _mm256_storeu_si256, _mm256_set1_epi32, _mm256_cmpeq_epi32, _mm256_and_si256, _mm256_andnot_si256, _mm256_movemask_epi8);

#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i8, 0_i8, -0x01, size_of::<i8>(), __m128i, size_of::<__m128i>(), _mm_loadu_si128, _mm_storeu_si128, _mm_set1_epi8, _mm_cmpeq_epi8, _mm_and_si128, _mm_andnot_si128, _mm_movemask_epi8);
#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i16, 0_i16, -0x0001, size_of::<i16>(), __m128i, size_of::<__m128i>(), _mm_loadu_si128, _mm_storeu_si128, _mm_set1_epi16, _mm_cmpeq_epi16, _mm_and_si128, _mm_andnot_si128, _mm_movemask_epi8);
#[cfg(all(target_feature = "sse2", not(target_feature = "avx2"), not(target_feature = "avx512f"), not(target_feature = "avx512bw")))]
generate_search!(i32, 0_i32, -0x00000001, size_of::<i32>(), __m128i, size_of::<__m128i>(), _mm_loadu_si128, _mm_storeu_si128, _mm_set1_epi32, _mm_cmpeq_epi32, _mm_and_si128, _mm_andnot_si128, _mm_movemask_epi8);
