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

use std::{
    hint::{
        black_box
    },
    mem::{
        transmute
    },
    ptr::{
        read_unaligned
    },
    arch::{
        aarch64::{
            uint16x4_t, uint16x8_t,
            vdup_n_u16, vdupq_n_u16,
            vld1_u16, vld1q_u16,
            vmaxv_u16, vmaxvq_u16,
            vclt_u16, vcltq_u16,
            vcgt_u16, vcgtq_u16,
            vceq_u16, vceqq_u16,
            vorr_u16, vorrq_u16,
            vand_u16, vandq_u16,
            vrev16_u8, vrev16q_u8,
            vreinterpret_u8_u16, vreinterpretq_u8_u16,
            vreinterpret_u16_u8, vreinterpretq_u16_u8,
            vextq_u16, vmvnq_u16
        }
    }
};

impl UTF16 {

    const __ENCODING_REGULAR_PAIR_BYTES:   usize = 2_usize;
    const __ENCODING_SURROGATE_PAIR_BYTES: usize = 4_usize;

    fn is_utf16_bmp_16x4(array: &[uint16x4_t], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (bad_range_mask, bad_result_mask): (uint16x4_t, uint16x4_t) =
        unsafe { (vdup_n_u16(0xF800), vdup_n_u16(0xD800)) };

        if endian {
            while index < length {
                let value: uint16x4_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpret_u16_u8(vrev16_u8(vreinterpret_u8_u16(read_unaligned::<uint16x4_t>(black_box(&array[index]))))) }
                } else {
                    unsafe { read_unaligned::<uint16x4_t>(black_box(&array[index])) }
                };

                if unsafe { vmaxv_u16(vceq_u16(vand_u16(value, bad_range_mask), bad_result_mask)) } != 0_u16 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: uint16x4_t = if cfg!(target_endian = "big") {
                    unsafe { read_unaligned::<uint16x4_t>(black_box(&array[index])) }
                } else {
                    unsafe { vreinterpret_u16_u8(vrev16_u8(vreinterpret_u8_u16(read_unaligned::<uint16x4_t>(black_box(&array[index]))))) }
                };

                if unsafe { vmaxv_u16(vceq_u16(vand_u16(value, bad_range_mask), bad_result_mask)) } != 0_u16 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    fn is_utf16_bmp_16x8(array: &[uint16x8_t], endian: bool) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (bad_range_mask, bad_result_mask): (uint16x8_t, uint16x8_t) =
        unsafe { (vdupq_n_u16(0xF800), vdupq_n_u16(0xD800)) };

        if endian {
            while index < length {
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                } else {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                };

                if unsafe { vmaxvq_u16(vceqq_u16(vandq_u16(value, bad_range_mask), bad_result_mask)) } != 0_u16 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                } else {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                };

                if unsafe { vmaxvq_u16(vceqq_u16(vandq_u16(value, bad_range_mask), bad_result_mask)) } != 0_u16 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    fn is_utf16_omp_16x4(array: &[uint16x4_t], endian: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (restricted_less_than_mask, restricted_big_than_mask): (uint16x4_t, uint16x4_t) =
        unsafe {
            (
                vld1_u16([0xD800, 0xDC00, 0xD800, 0xDC00].as_ptr()),
                vld1_u16([0xDBFF, 0xDFFF, 0xDBFF, 0xDFFF].as_ptr())
            )
        };

        if endian {
            while index < length {
                let value: uint16x4_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpret_u16_u8(vrev16_u8(vreinterpret_u8_u16(read_unaligned::<uint16x4_t>(black_box(&array[index]))))) }
                } else {
                    unsafe { read_unaligned::<uint16x4_t>(black_box(&array[index])) }
                };

                if unsafe { vmaxv_u16(vorr_u16(vclt_u16(value, restricted_less_than_mask), vcgt_u16(value, restricted_big_than_mask))) } != 0_u16 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: uint16x4_t = if cfg!(target_endian = "big") {
                    unsafe { read_unaligned::<uint16x4_t>(black_box(&array[index])) }
                } else {
                    unsafe { vreinterpret_u16_u8(vrev16_u8(vreinterpret_u8_u16(read_unaligned::<uint16x4_t>(black_box(&array[index]))))) }
                };

                if unsafe { vmaxv_u16(vorr_u16(vclt_u16(value, restricted_less_than_mask), vcgt_u16(value, restricted_big_than_mask))) } != 0_u16 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    fn is_utf16_omp_16x8(array: &[uint16x8_t], endian: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (restricted_less_than_mask, restricted_big_than_mask): (uint16x8_t, uint16x8_t) =
        unsafe {
            (
                vld1q_u16([0xD800, 0xDC00, 0xD800, 0xDC00, 0xD800, 0xDC00, 0xD800, 0xDC00].as_ptr()),
                vld1q_u16([0xDBFF, 0xDFFF, 0xDBFF, 0xDFFF, 0xDBFF, 0xDFFF, 0xDBFF, 0xDFFF].as_ptr())
            )
        };

        if endian {
            while index < length {
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                } else {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                };

                if unsafe { vmaxvq_u16(vorrq_u16(vcltq_u16(value, restricted_less_than_mask), vcgtq_u16(value, restricted_big_than_mask))) } != 0_u16 { return false; }

                index += 1_usize;
            }
        } else {
            while index < length {
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                } else {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                };

                if unsafe { vmaxvq_u16(vorrq_u16(vcltq_u16(value, restricted_less_than_mask), vcgtq_u16(value, restricted_big_than_mask))) } != 0_u16 { return false; }

                index += 1_usize;
            }
        }

        return true;
    }

    fn is_utf16_mixed_16x8(array: &[uint16x8_t], endian: bool, mut continuation: bool) -> bool {

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        let (any_part_surrogate_detect_mask, following_surrogate_detect_mask): (uint16x8_t, uint16x8_t) = unsafe {
            (vdupq_n_u16(0xF800), vdupq_n_u16(0xFC00))
        };

        let (high_surrogate_detect_mask, low_surrogate_detect_mask): (uint16x8_t, uint16x8_t) = unsafe {
            (vdupq_n_u16(0xD800), vdupq_n_u16(0xDC00))
        };

        let (test_continuation_mask, test_following_continuation_mask, ignore_leading_continuation_mask): (uint16x8_t, uint16x8_t, uint16x8_t) = unsafe {
            (
                vld1q_u16([0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0xFFFF].as_ptr()),
                vld1q_u16([0xFFFF, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000].as_ptr()),
                vld1q_u16([0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0x0000].as_ptr())
            )
        };

        let zero: uint16x8_t = unsafe { vdupq_n_u16(0x0000) };

        if endian {
            while index < length {
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                } else {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                };

                let any_surrogate_mask: uint16x8_t = unsafe { vceqq_u16(vandq_u16(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 {
                    let following_surrogate_mask: uint16x8_t = unsafe { vceqq_u16(vandq_u16(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                            let (any_surrogate_mask, following_surrogate_mask): (uint16x8_t, uint16x8_t) = if unsafe {
                                vmaxvq_u16(vandq_u16(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_u16 {
                                if unsafe { vmaxvq_u16(vandq_u16(any_surrogate_mask, test_continuation_mask)) } == 0_u16 { continuation = false; }
                                else if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, test_continuation_mask)) } != 0_u16 { continuation = false; }

                                if continuation {
                                    unsafe { (vextq_u16::<1>(vandq_u16(any_surrogate_mask, ignore_leading_continuation_mask), zero), vextq_u16::<1>(following_surrogate_mask, zero)) }
                                } else {
                                    unsafe { (vextq_u16::<1>(any_surrogate_mask, zero), vextq_u16::<1>(following_surrogate_mask, zero)) }
                                }

                            } else {
                                return false;
                            };

                            if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                                let potential_high_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<1>(following_surrogate_mask, zero) };

                                if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_u16 {
                                    let high_surrogates_mask: uint16x8_t = unsafe { vandq_u16(any_surrogate_mask, vmvnq_u16(following_surrogate_mask)) };

                                    let potential_following_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<7>(zero, high_surrogates_mask) };

                                    if unsafe { vmaxvq_u16(high_surrogates_mask) } == 0_u16 { return false; }
                                    else if unsafe { transmute::<uint16x8_t, u128>(potential_high_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(high_surrogates_mask) } { return false; }
                                    else if unsafe { transmute::<uint16x8_t, u128>(potential_following_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (uint16x8_t, uint16x8_t) = if unsafe {
                            vmaxvq_u16(vandq_u16(any_surrogate_mask, test_continuation_mask))
                        } != 0_u16 {
                            if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, test_continuation_mask)) } == 0_u16 { continuation = true; }

                            if continuation {
                                unsafe { (vandq_u16(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                            let potential_high_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<1>(following_surrogate_mask, zero) };

                            if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_u16 {

                                let high_surrogates_mask: uint16x8_t = unsafe { vandq_u16(any_surrogate_mask, vmvnq_u16(following_surrogate_mask)) };

                                let potential_following_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<7>(zero, high_surrogates_mask) };

                                if unsafe { vmaxvq_u16(high_surrogates_mask) } == 0_u16 { return false; }
                                else if unsafe { transmute::<uint16x8_t, u128>(potential_high_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(high_surrogates_mask) } {return false; }
                                else if unsafe { transmute::<uint16x8_t, u128>(potential_following_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 { return false; }
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
                let value: uint16x8_t = if cfg!(target_endian = "big") {
                    unsafe { read_unaligned::<uint16x8_t>(black_box(&array[index])) }
                } else {
                    unsafe { vreinterpretq_u16_u8(vrev16q_u8(vreinterpretq_u8_u16(read_unaligned::<uint16x8_t>(black_box(&array[index]))))) }
                };

                let any_surrogate_mask: uint16x8_t = unsafe { vceqq_u16(vandq_u16(value, any_part_surrogate_detect_mask), high_surrogate_detect_mask) };

                if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 {
                    let following_surrogate_mask: uint16x8_t = unsafe { vceqq_u16(vandq_u16(value, following_surrogate_detect_mask), low_surrogate_detect_mask) };

                    if continuation {
                        if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                            let (any_surrogate_mask, following_surrogate_mask): (uint16x8_t, uint16x8_t) = if unsafe {
                                vmaxvq_u16(vandq_u16(following_surrogate_mask, test_following_continuation_mask))
                            } != 0_u16 {
                                if unsafe { vmaxvq_u16(vandq_u16(any_surrogate_mask, test_continuation_mask)) } == 0_u16 { continuation = false; }
                                else if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, test_continuation_mask)) } != 0_u16 { continuation = false; }

                                if continuation {
                                    unsafe { (vextq_u16::<1>(vandq_u16(any_surrogate_mask, ignore_leading_continuation_mask), zero), vextq_u16::<1>(following_surrogate_mask, zero)) }
                                } else {
                                    unsafe { (vextq_u16::<1>(any_surrogate_mask, zero), vextq_u16::<1>(following_surrogate_mask, zero)) }
                                }

                            } else {
                                return false;
                            };

                            if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                                let potential_high_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<1>(following_surrogate_mask, zero) };

                                if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_u16 {
                                    let high_surrogates_mask: uint16x8_t = unsafe { vandq_u16(any_surrogate_mask, vmvnq_u16(following_surrogate_mask)) };

                                    let potential_following_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<7>(zero, high_surrogates_mask) };

                                    if unsafe { vmaxvq_u16(high_surrogates_mask) } == 0_u16 { return false; }
                                    else if unsafe { transmute::<uint16x8_t, u128>(potential_high_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(high_surrogates_mask) } { return false; }
                                    else if unsafe { transmute::<uint16x8_t, u128>(potential_following_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(following_surrogate_mask) } { return false; }
                                } else {
                                    return false;
                                }
                            } else {
                                if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 { return false; }
                            }
                        } else {
                            return false;
                        }
                    } else {
                        let (any_surrogate_mask, following_surrogate_mask): (uint16x8_t, uint16x8_t) = if unsafe {
                            vmaxvq_u16(vandq_u16(any_surrogate_mask, test_continuation_mask))
                        } != 0_u16 {
                            if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, test_continuation_mask)) } == 0_u16 { continuation = true; }

                            if continuation {
                                unsafe { (vandq_u16(any_surrogate_mask, ignore_leading_continuation_mask), following_surrogate_mask) }
                            } else {
                                (any_surrogate_mask, following_surrogate_mask)
                            }
                        } else {
                            (any_surrogate_mask, following_surrogate_mask)
                        };

                        if unsafe { vmaxvq_u16(following_surrogate_mask) } != 0_u16 {
                            let potential_high_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<1>(following_surrogate_mask, zero) };

                            if unsafe { vmaxvq_u16(vandq_u16(following_surrogate_mask, potential_high_surrogates_mask)) } == 0_u16 {

                                let high_surrogates_mask: uint16x8_t = unsafe { vandq_u16(any_surrogate_mask, vmvnq_u16(following_surrogate_mask)) };

                                let potential_following_surrogates_mask: uint16x8_t = unsafe { vextq_u16::<7>(zero, high_surrogates_mask) };

                                if unsafe { vmaxvq_u16(high_surrogates_mask) } == 0_u16 { return false; }
                                else if unsafe { transmute::<uint16x8_t, u128>(potential_high_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(high_surrogates_mask) } {return false; }
                                else if unsafe { transmute::<uint16x8_t, u128>(potential_following_surrogates_mask) } != unsafe { transmute::<uint16x8_t, u128>(following_surrogate_mask) } { return false; }
                            } else {
                                return false;
                            }
                        } else {
                            if unsafe { vmaxvq_u16(any_surrogate_mask) } != 0_u16 { return false; }
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
                if indivisible < 9_usize {
                    if omp {
                        let indivisible_code_array: uint16x4_t = {
                            if endian {
                                let mut indivisible_code_array: [u8; 8_usize] = [0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC];

                                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 8_usize], uint16x4_t>(indivisible_code_array) }
                            } else {
                                let mut indivisible_code_array: [u8; 8_usize] = [0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00];

                                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 8_usize], uint16x4_t>(indivisible_code_array) }
                            }
                        };

                        result &= UTF16::is_utf16_omp_16x4(&[indivisible_code_array], endian);

                    } else {
                        let indivisible_code_array: uint16x4_t = {
                            let mut indivisible_code_array: [u8; 8_usize] = [0_u8; 8_usize];
                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 8_usize], uint16x4_t>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_bmp_16x4(&[indivisible_code_array], endian);
                    }
                } else if indivisible < 17_usize {
                    if omp {
                        let indivisible_code_array: uint16x8_t = {
                            if endian {
                                let mut indivisible_code_array: [u8; 16_usize] = [0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC];

                                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
                            } else {
                                let mut indivisible_code_array: [u8; 16_usize] = [0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00, 0xD8, 0x00, 0xDC, 0x00];

                                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
                            }
                        };

                        result &= UTF16::is_utf16_omp_16x8(&[indivisible_code_array], endian);
                    } else {
                        let indivisible_code_array: uint16x8_t = {
                            let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                            while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_bmp_16x8(&[indivisible_code_array], endian);
                    }
                }
            } else {
                if length < 17_usize {
                    let indivisible_code_array: uint16x8_t = {
                        let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                        while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                        unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
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
                        let indivisible_code_array: uint16x8_t = {
                            let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                            while index < align_indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                            unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
                        };

                        result &= UTF16::is_utf16_mixed_16x8(&[indivisible_code_array], endian, false);
                    } else {
                        continuation = true;

                        if potentially_surrogate_index >= 2_usize {
                            let indivisible_code_array: uint16x8_t = {
                                let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                                while index < align_indivisible - 2_usize { indivisible_code_array[index] = array[index]; index += 1_usize; }
                                unsafe { transmute::<[u8; 16_usize], uint16x8_t>(indivisible_code_array) }
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
                        result &= UTF16::is_utf16_omp_16x8(unsafe { std::slice::from_raw_parts::<uint16x8_t>(transmute::<*const u8, *const uint16x8_t>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian);
                    } else {
                        result &= UTF16::is_utf16_bmp_16x8(unsafe { std::slice::from_raw_parts::<uint16x8_t>(transmute::<*const u8, *const uint16x8_t>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian);
                    }
                } else {
                    result &= UTF16::is_utf16_mixed_16x8(unsafe { std::slice::from_raw_parts::<uint16x8_t>(transmute::<*const u8, *const uint16x8_t>(array.as_ptr().add(indivisible)), remains_length / 16_usize) }, endian, continuation);
                }
            }
        }

        return result;
    }
}