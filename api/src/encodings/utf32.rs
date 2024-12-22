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

    const fn is_not_utf32(code: u32) -> bool {
        return if (code & 0xFFFF0000) == 0x00000000 && (code & 0x0000F800) == 0x0000D800 { true }
        else if (code & 0xFFFF0000) > 0x00100000 { true }
        else { false };
    }

    pub const fn is_utf32(array: &[u32], endian: bool) -> bool {
        const fn swap_endian(value: u32) -> u32 {
            return ((value & 0xFF000000) >> 24)
                 | ((value & 0x00FF0000) >> 8)
                 | ((value & 0x0000FF00) << 8)
                 | ((value & 0x000000FF) << 24);
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

    pub const fn is_utf32_from_byte_array(bytes: &[u8], endian: bool) -> bool {
        let length: usize = bytes.len();

        return if length == 0 || length % UTF32::__ENCODING_BYTES != 0_usize { false }
        else { UTF32::is_utf32(unsafe { std::slice::from_raw_parts::<u32>(bytes.as_ptr() as *const u32, length / UTF32::__ENCODING_BYTES) }, endian) }
    }
}
