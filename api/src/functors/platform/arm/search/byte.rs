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

use core::{
    mem::{
        transmute,
    },
    arch::{
        arm::{
            uint8x16_t, uint16x8_t, uint32x4_t,
            vld1q_u8, vld1q_u16, vld1q_u32,
            vdupq_n_u8, vdupq_n_u16, vdupq_n_u32,
            vandq_u8, vandq_u16, vandq_u32,
            vmvnq_u8, vmvnq_u16, vmvnq_u32,
            vceqq_u8, vceqq_u16, vceqq_u32,
            vst1q_u8, vst1q_u16, vst1q_u32
        }
    }
};

macro_rules! generate_search {
    ($t:ty, $t_default:expr, $not_ignore_mask:expr, $t_size:expr, $precision:ty, $register_size:expr, $load:expr, $store:expr, $dup_one_t:expr, $eq_compare:expr, $bitwise_and:expr, $bitwise_not:expr, $vector_to_scalar:expr) => {
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
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let mut matches: [[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize] = [[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize];

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));
                                $store(matches[3].as_mut_ptr(), $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                let first: $precision = $load(array_ptr);

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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

                            let first: $precision = $load(aligned_array.as_ptr());

                            $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(array_ptr);
                                    let e_four: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(matches[3].as_mut_ptr(), $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                let s_first: $precision = $load(aligned_array.as_ptr());
                                let e_first: $precision = $load(array_ptr.add(1_usize));

                                (s_first, e_first)
                            };

                            $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) == 0 {
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
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)),
                                                    pattern_loaded
                                                ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)), pattern_loaded),
                                                ), pattern_ignore_mask)) == 0 {
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
                            matches[0] = core::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                ))) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                ))) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); return search_result;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)),
                                                pattern_loaded
                                            ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)), pattern_loaded),
                                            ), pattern_ignore_mask)) == 0 {
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
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let mut matches: [[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize] = [[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize];

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));
                                $store(matches[3].as_mut_ptr(), $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                let first: $precision = $load(array_ptr);

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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

                            let first: $precision = $load(aligned_array.as_ptr());

                            $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(array_ptr);
                                    let e_four: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(matches[3].as_mut_ptr(), $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if next_pass { index_of_match += 1_usize; next_pass = false; }
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if next_pass { index_of_match += 1_usize; next_pass = false; }
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                let s_first: $precision = $load(aligned_array.as_ptr());
                                let e_first: $precision = $load(array_ptr.add(1_usize));

                                (s_first, e_first)
                            };

                            $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if next_pass { index_of_match += 1_usize; }
                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) == 0 {
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
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)),
                                                    pattern_loaded
                                                ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if next_index_of_match != 0_usize { index_of_match += next_index_of_match; next_index_of_match = 0_usize; }
                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)), pattern_loaded),
                                                ), pattern_ignore_mask)) == 0 {
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
                            matches[0] = core::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

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
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                ))) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                ))) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size); break;
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)),
                                                pattern_loaded
                                            ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                if next_index_of_match != 0_usize { index_of_match += next_index_of_match; }
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)), pattern_loaded),
                                            ), pattern_ignore_mask)) == 0 {
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
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(array_ptr.as_ptr()), array_ptr.len() / $t_size) },
                    unsafe { core::slice::from_raw_parts::<$t>(transmute::<*const u8, *const $t>(pattern_ptr.as_ptr()), pattern_ptr.len() / $t_size) }
                );

                let (array_length, pattern_length, is_search_possible): (usize, usize, bool) = ByteSearch::<$t>::is_search_possible(array, pattern, limit); if !is_search_possible { return search_result; }

                let (mut array_ptr, mut array_index, mut remains_length, mut index_of_match): (*const $t, usize, usize, usize) = (array.as_ptr(), 0_usize, array_length, 0_usize);

                let mut matches: [[$t; COUNT_OF_VALUES_IN_REGISTER]; 4_usize] = [[$t_default; COUNT_OF_VALUES_IN_REGISTER]; 4_usize];

                unsafe {
                    if pattern_length == 1_usize {

                        let pattern_mask: $precision = $dup_one_t(pattern[0]);

                        if remains_length >= PRECISION_LENGTH[3] {
                            while remains_length >= PRECISION_LENGTH[0] {
                                let (first, second, third, four): ($precision, $precision, $precision, $precision) = {
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let four: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third, four)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));
                                $store(matches[3].as_mut_ptr(), $eq_compare(four, pattern_mask));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let third: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second, third)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));
                                $store(matches[2].as_mut_ptr(), $eq_compare(third, pattern_mask));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let first: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);
                                    let second: $precision = $load(array_ptr); array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (first, second)
                                };

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));
                                $store(matches[1].as_mut_ptr(), $eq_compare(second, pattern_mask));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                let first: $precision = $load(array_ptr);

                                $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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

                            let first: $precision = $load(aligned_array.as_ptr());

                            $store(matches[0].as_mut_ptr(), $eq_compare(first, pattern_mask));

                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_four: $precision = $load(array_ptr);
                                    let e_four: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third, s_four, e_four)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));
                                $store(matches[3].as_mut_ptr(), $bitwise_and($eq_compare(s_four, start_pattern_mask), $eq_compare(e_four, end_pattern_mask)));

                                for i in 0..4 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_third: $precision = $load(array_ptr);
                                    let e_third: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second, s_third, e_third)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));
                                $store(matches[2].as_mut_ptr(), $bitwise_and($eq_compare(s_third, start_pattern_mask), $eq_compare(e_third, end_pattern_mask)));

                                for i in 0..3 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    let s_second: $precision = $load(array_ptr);
                                    let e_second: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first, s_second, e_second)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));
                                $store(matches[1].as_mut_ptr(), $bitwise_and($eq_compare(s_second, start_pattern_mask), $eq_compare(e_second, end_pattern_mask)));

                                for i in 0..2 {
                                    if $vector_to_scalar($load(matches[i].as_ptr())) != 0 {
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
                                    let s_first: $precision = $load(array_ptr);
                                    let e_first: $precision = $load(array_ptr.add(1_usize));
                                    array_ptr = array_ptr.add(COUNT_OF_VALUES_IN_REGISTER);

                                    (s_first, e_first)
                                };

                                $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                let s_first: $precision = $load(aligned_array.as_ptr());
                                let e_first: $precision = $load(array_ptr.add(1_usize));

                                (s_first, e_first)
                            };

                            $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                            if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
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
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                let mut result: bool = true;

                                                for i in 0_usize..middle_pattern_parts - 1_usize {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) != 0 {
                                                        result &= false; break
                                                    }
                                                }

                                                if result {
                                                    if $vector_to_scalar($bitwise_not($eq_compare(
                                                        $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                        $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                    ))) == 0 {
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
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)),
                                                    pattern_loaded
                                                ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                while array_index < last_array_index_multiple_of_register {
                                    let (s_first, e_first): ($precision, $precision) = {
                                        let s_first: $precision = $load(array_ptr);
                                        let e_first: $precision = $load(array_ptr.add(last_pattern_index));

                                        (s_first, e_first)
                                    };

                                    $store(matches[0].as_mut_ptr(), $bitwise_and($eq_compare(s_first, start_pattern_mask), $eq_compare(e_first, end_pattern_mask)));

                                    if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                        while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                            if matches[0][index_of_match] != 0 {
                                                if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match)), pattern_loaded),
                                                ), pattern_ignore_mask)) == 0 {
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
                            matches[0] = core::mem::zeroed::<[$t; COUNT_OF_VALUES_IN_REGISTER]>();

                            for i in 0..(array_length - array_index) {
                                if array[array_index + i] == pattern[0] {
                                    if array[array_index + i + last_pattern_index] == pattern[last_pattern_index] { matches[0][i] = $not_ignore_mask; }
                                    else { matches[0][i] = 0; }
                                } else {
                                    matches[0][i] = 0;
                                }
                            }

                            if pattern_length > COUNT_OF_VALUES_IN_REGISTER {
                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            let mut result: bool = true;

                                            for i in 0_usize..middle_pattern_parts - 1_usize {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + 1_usize + i * COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(1_usize + i * COUNT_OF_VALUES_IN_REGISTER))
                                                ))) != 0 {
                                                    result &= false; break
                                                }
                                            }

                                            if result {
                                                if $vector_to_scalar($bitwise_not($eq_compare(
                                                    $load(array_ptr.add(index_of_match + last_pattern_index - COUNT_OF_VALUES_IN_REGISTER)),
                                                    $load(pattern.as_ptr().add(last_pattern_index - COUNT_OF_VALUES_IN_REGISTER))
                                                ))) == 0 {
                                                    search_result.push((array_index + index_of_match) * $t_size);
                                                }
                                            }
                                        }
                                        index_of_match += 1_usize;
                                    }
                                }
                            } else if pattern_length == COUNT_OF_VALUES_IN_REGISTER {
                                let pattern_loaded: $precision = $load(pattern.as_ptr());

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)),
                                                pattern_loaded
                                            ))) == 0 {
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
                                    ($load(pattern_array.as_ptr()), $load(ignore_mask.as_ptr()))
                                };

                                if $vector_to_scalar($load(matches[0].as_ptr())) != 0 {
                                    while index_of_match < COUNT_OF_VALUES_IN_REGISTER {
                                        if matches[0][index_of_match] != 0 {
                                            if $vector_to_scalar($bitwise_and($bitwise_not($eq_compare(
                                                $load(array_ptr.add(index_of_match)), pattern_loaded),
                                            ), pattern_ignore_mask)) == 0 {
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

generate_search!(u8, 0_u8, 0xFF, size_of::<u8>(), uint8x16_t, size_of::<uint8x16_t>(), vld1q_u8, vst1q_u8, vdupq_n_u8, vceqq_u8, vandq_u8, vmvnq_u8, transmute::<uint8x16_t, u128>);
generate_search!(u16, 0_u16, 0xFFFF, size_of::<u16>(), uint16x8_t, size_of::<uint16x8_t>(), vld1q_u16, vst1q_u16, vdupq_n_u16, vceqq_u16, vandq_u16, vmvnq_u16, transmute::<uint16x8_t, u128>);
generate_search!(u32, 0_u32, 0xFFFFFFFF, size_of::<u32>(), uint32x4_t, size_of::<uint32x4_t>(), vld1q_u32, vst1q_u32, vdupq_n_u32, vceqq_u32, vandq_u32, vmvnq_u32, transmute::<uint32x4_t, u128>);
