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

use core::{
    arch::{
        aarch64::{
            uint32x2_t, uint32x4_t,
            vld1_u8, vld1q_u8,
            vrev32_u8, vrev32q_u8,
            vdup_n_u32, vdupq_n_u32,
            vcle_u32, vcleq_u32,
            vcgt_u32, vcgtq_u32,
            vceq_u32, vceqq_u32,
            vand_u32, vandq_u32,
            vmaxv_u32, vmaxvq_u32,
            vreinterpret_u32_u8, vreinterpretq_u32_u8,
        }
    }
};

impl UTF32 {

    const __ENCODING_BYTES: usize = 4_usize;

    fn is_utf32_32x2(array: *const u8, length: usize, endian: bool) -> bool {

        let mut offset: usize = 0_usize;

        let (max_mask, range_mask, bad_range_mask, bad_result_mask): (uint32x2_t, uint32x2_t, uint32x2_t, uint32x2_t) =
        unsafe { (vdup_n_u32(0x0010FFFF), vdup_n_u32(0x0000FFFF), vdup_n_u32(0x0000F800), vdup_n_u32(0x0000D800)) };

        if endian {
            while offset < length {
                let value: uint32x2_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpret_u32_u8(vrev32_u8(vld1_u8(array.add(offset)))) }
                } else {
                    unsafe { vreinterpret_u32_u8(vld1_u8(array.add(offset))) }
                };

                if unsafe { vmaxv_u32(vcgt_u32(value, max_mask)) } != 0_u32 { return false; }
                else if unsafe { vmaxv_u32(vceq_u32(vand_u32(vand_u32(value, vcle_u32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_u32 { return false; }
                else { offset += 8_usize; }
            }
        } else {
            while offset < length {
                let value: uint32x2_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpret_u32_u8(vld1_u8(array.add(offset))) }
                } else {
                    unsafe { vreinterpret_u32_u8(vrev32_u8(vld1_u8(array.add(offset)))) }
                };

                if unsafe { vmaxv_u32(vcgt_u32(value, max_mask)) } != 0_u32 { return false; }
                else if unsafe { vmaxv_u32(vceq_u32(vand_u32(vand_u32(value, vcle_u32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_u32 { return false; }
                else { offset += 8_usize; }
            }
        }

        return true;
    }

    fn is_utf32_32x4(array: *const u8, length: usize, endian: bool) -> bool {

        let mut offset: usize = 0_usize;

        let (max_mask, range_mask, bad_range_mask, bad_result_mask): (uint32x4_t, uint32x4_t, uint32x4_t, uint32x4_t) =
        unsafe { (vdupq_n_u32(0x0010FFFF), vdupq_n_u32(0x0000FFFF), vdupq_n_u32(0x0000F800), vdupq_n_u32(0x0000D800)) };

        if endian {
            while offset < length {
                let value: uint32x4_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(array.add(offset)))) }
                } else {
                    unsafe { vreinterpretq_u32_u8(vld1q_u8(array.add(offset))) }
                };

                if unsafe { vmaxvq_u32(vcgtq_u32(value, max_mask)) } != 0_u32 { return false; }
                else if unsafe { vmaxvq_u32(vceqq_u32(vandq_u32(vandq_u32(value, vcleq_u32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_u32 { return false; }
                else { offset += 16_usize; }
            }
        } else {
            while offset < length {
                let value: uint32x4_t = if cfg!(target_endian = "big") {
                    unsafe { vreinterpretq_u32_u8(vld1q_u8(array.add(offset))) }
                } else {
                    unsafe { vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(array.add(offset)))) }
                };

                if unsafe { vmaxvq_u32(vcgtq_u32(value, max_mask)) } != 0_u32 { return false; }
                else if unsafe { vmaxvq_u32(vceqq_u32(vandq_u32(vandq_u32(value, vcleq_u32(value, range_mask)), bad_range_mask), bad_result_mask)) } != 0_u32 { return false; }
                else { offset += 16_usize; }
            }
        }

        return true;
    }

    pub fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {

        let length: usize = array.len();

        let (mut index, indivisible, mut result): (usize, usize, bool) = (0_usize, length % 16_usize, true);

        if length == 0_usize || length % UTF32::__ENCODING_BYTES != 0_usize { return false; }

        if indivisible != 0_usize {
            if indivisible < 9_usize {
                let indivisible_code_array: [u8; 8_usize] = {
                    let mut indivisible_code_array: [u8; 8_usize] = [0_u8; 8_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    indivisible_code_array
                };

                result &= UTF32::is_utf32_32x2(indivisible_code_array.as_ptr(), 1_usize, endian);
            } else if indivisible < 13_usize {
                let indivisible_code_array: [u8; 16_usize] = {
                    let mut indivisible_code_array: [u8; 16_usize] = [0_u8; 16_usize];
                    while index < indivisible { indivisible_code_array[index] = array[index]; index += 1_usize; }
                    indivisible_code_array
                };

                result &= UTF32::is_utf32_32x4(indivisible_code_array.as_ptr(), 1_usize, endian);
            }
        }

        if result {
            let remains_length: usize = length - indivisible;

            if remains_length != 0_usize {
                result &= UTF32::is_utf32_32x4(unsafe { array.as_ptr().add(indivisible) }, remains_length, endian);
            }
        }

        return result;
    }
}
