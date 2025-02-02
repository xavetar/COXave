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
            uint8x8_t, uint8x16_t,
            vdup_n_u8, vdupq_n_u8,
            vand_u8, vandq_u8,
            vmaxv_u8, vmaxvq_u8
        }
    }
};

impl ASCII {

    fn is_ascii_8x8(array: &[uint8x8_t]) -> bool {
        let (mut index, length, mask): (usize, usize, uint8x8_t) = (0_usize, array.len(), unsafe { vdup_n_u8(0x80) });

        while index < length { if unsafe { vmaxv_u8(vand_u8(read_unaligned(black_box(&array[index])), mask)) } != 0_u8 { return false; } else { index += 1_usize; } }

        return true;
    }

    fn is_ascii_8x16(array: &[uint8x16_t]) -> bool {
        let (mut index, length, mask): (usize, usize, uint8x16_t) = (0_usize, array.len(), unsafe { vdupq_n_u8(0x80) });

        while index < length { if unsafe { vmaxvq_u8(vandq_u8(read_unaligned(black_box(&array[index])), mask)) } != 0_u8 { return false; } else { index += 1_usize; } }

        return true;
    }

    pub fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 16_usize, true);

        if length == 0_usize { return false; }

        if indivisible != 0_usize {
            if indivisible < 9_usize {
                let indivisible_code_array: uint8x8_t = {
                    let mut indivisible_code_array: [u8; 8_usize] = [0_u8; 8_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    unsafe { transmute::<[u8; 8_usize], uint8x8_t>(indivisible_code_array) }
                };

                result &= ASCII::is_ascii_8x8(&[indivisible_code_array]);
            } else if indivisible < 16_usize {
                let indivisible_code_array: uint8x16_t = {
                    let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    unsafe { transmute::<[u8; 16_usize], uint8x16_t>(indivisible_code_array) }
                };

                result &= ASCII::is_ascii_8x16(&[indivisible_code_array]);
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= ASCII::is_ascii_8x16(unsafe { std::slice::from_raw_parts::<uint8x16_t>(transmute::<*const u8, *const uint8x16_t>(array.as_ptr().add(indivisible)), remains_length / 16_usize) });
            }
        }

        return result;
    }
}
