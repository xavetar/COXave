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

use core::{
    arch::{
        aarch64::{
            vld1_u8, vld1q_u8,
            vmaxv_u8, vmaxvq_u8
        }
    }
};

impl ASCII {

    fn is_ascii_8x8(array: *const u8, length: usize) -> bool {
        let mut index: usize = 0_usize;

        while index < length { if unsafe { vmaxv_u8(vld1_u8(array.add(index))) } > 0x7F { return false; } else { index += 8_usize; } }

        return true;
    }

    fn is_ascii_8x16(array: *const u8, length: usize) -> bool {
        let mut index: usize = 0_usize;

        while index < length { if unsafe { vmaxvq_u8(vld1q_u8(array.add(index))) } > 0x7F { return false; } else { index += 16_usize; } }

        return true;
    }

    pub fn is_ascii_from_byte_array(array: &[u8]) -> bool {
        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 16_usize, true);

        if length == 0_usize { return false; }

        if indivisible != 0_usize {
            if indivisible < 9_usize {
                let indivisible_code_array: [u8; 8_usize] = {
                    let mut indivisible_code_array: [u8; 8_usize] = [0_u8; 8_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    indivisible_code_array
                };

                result &= ASCII::is_ascii_8x8(indivisible_code_array.as_ptr(), 1_usize);
            } else if indivisible < 16_usize {
                let indivisible_code_array: [u8; 16_usize] = {
                    let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    indivisible_code_array
                };

                result &= ASCII::is_ascii_8x16(indivisible_code_array.as_ptr(), 1_usize);
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= ASCII::is_ascii_8x16(unsafe { array.as_ptr().add(indivisible) }, remains_length);
            }
        }

        return result;
    }
}
