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

pub struct UTF32;

impl UTF32 {
    const __ENCODING_BYTES: usize = 4_usize;

    const fn is_not_utf32(code: u128) -> bool {
        return if (code & 0xFFFF0000FFFF0000FFFF0000FFFF0000) == 0x00000000000000000000000000000000  {

            let mask_result: u128 = !(code ^ 0xFFFFD8FFFFFFD8FFFFFFD8FFFFFFD8FF) | 0x00000700000007000000070000000700;

            if (((mask_result >> 8) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 40) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 72) & 0xFF) as u8 == 0xFF)
            || (((mask_result >> 104) & 0xFF) as u8 == 0xFF) { true } else { false }
        }
        else {
            if (((code >> 16) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 48) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 80) & 0xFFFF) as u16 > 0x0010)
            || (((code >> 112) & 0xFFFF) as u16 > 0x0010) { true } else { false }
        }
    }

    pub const fn is_utf32(array: &[u128], endian: bool) -> bool {
        const fn swap_endian(value: u128) -> u128 {
            return ((value & 0xFF000000000000000000000000000000) >> 24)
                 | ((value & 0x00FF0000000000000000000000000000) >> 8)
                 | ((value & 0x0000FF00000000000000000000000000) << 8)
                 | ((value & 0x000000FF000000000000000000000000) << 24)
                 | ((value & 0x00000000FF0000000000000000000000) >> 24)
                 | ((value & 0x0000000000FF00000000000000000000) >> 8)
                 | ((value & 0x000000000000FF000000000000000000) << 8)
                 | ((value & 0x00000000000000FF0000000000000000) << 24)
                 | ((value & 0x0000000000000000FF00000000000000) >> 24)
                 | ((value & 0x000000000000000000FF000000000000) >> 8)
                 | ((value & 0x00000000000000000000FF0000000000) << 8)
                 | ((value & 0x0000000000000000000000FF00000000) << 24)
                 | ((value & 0x000000000000000000000000FF000000) >> 24)
                 | ((value & 0x00000000000000000000000000FF0000) >> 8)
                 | ((value & 0x0000000000000000000000000000FF00) << 8)
                 | ((value & 0x000000000000000000000000000000FF) << 24);
        }

        let (mut index, length): (usize, usize) = (0_usize, array.len());

        #[cfg(target_endian = "big")]
        if endian {
            while index < length { if UTF32::is_not_utf32(swap_endian(array[index])) { return false; }; index += 1_usize; }
        } else {
            while index < length { if UTF32::is_not_utf32(array[index]) { return false; }; index += 1_usize; }
        }

        #[cfg(target_endian = "little")]
        if endian {
            while index < length { if UTF32::is_not_utf32(array[index]) { return false; }; index += 1_usize; }
        } else {
            while index < length { if UTF32::is_not_utf32(swap_endian(array[index])) { return false; }; index += 1_usize; }
        }

        return true;
    }

    pub const fn is_utf32_from_byte_array(array: &[u8], endian: bool) -> bool {
        let length: usize = array.len();

        let (mut index, mut indivisible_code_array): (usize, [u8; 16_usize]) = (0_usize, [0_u8; 16_usize]);

        return if length == 0 || length % UTF32::__ENCODING_BYTES != 0_usize { false }
        else if length < 16_usize {
            while index < length { indivisible_code_array[index] = array[index]; index += 1; }

            UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(indivisible_code_array.as_ptr() as *const u128, 1_usize) }, endian)
        } else {
            let indivisible: usize = length % 16_usize;

            if indivisible != 0 {
                while index < indivisible { indivisible_code_array[index] = array[index]; index += 1; }

                if !UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(indivisible_code_array.as_ptr() as *const u128, 1_usize) }, endian) { false }
                else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr().add(indivisible) as *const u128, (length - indivisible) / 16_usize) }, endian) }
            } else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u128>(array.as_ptr() as *const u128, length / 16_usize) }, endian) }
        }
    }
}
