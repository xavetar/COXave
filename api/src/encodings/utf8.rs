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

pub struct UTF8;

impl UTF8 {

    const fn is_lead(code: u8) -> bool {
        return if (code & 0x80) == 0x80 { if (code & 0x40) == 0x40 { true } else { false } } else { true };
    }

    const fn is_following(code: u8) -> bool {
        return if (code & 0xC0) == 0x80 { true } else { false };
    }

    const fn is_not_following(code: u8) -> bool {
        return if (code & 0x80) == 0x00 || (code & 0xC0) == 0xC0 { true } else { false };
    }

    pub const fn is_utf8(array: &[u8]) -> bool {
        let (mut index, length): (usize, usize) = (0_usize, array.len());

        if length == 0_usize { return false; }

        let (mut second_index, mut third_index, mut four_index): (usize, usize, usize);

        while index < length {
            (second_index, third_index, four_index) = (index + 1, index + 2, index + 3);

            if (array[index] & 0x80) == 0x00 {
                index += 1
            } else if (array[index] & 0xE0) == 0xC0 {
                if (array[index] & 0xFE) == 0xC0 { return false; }
                else if second_index >= length { return false; }
                else if UTF8::is_not_following(array[second_index]) { return false; }
                index += 2;
            } else if (array[index] & 0xF0) == 0xE0 {
                if third_index >= length { return false; }
                else if UTF8::is_not_following(array[second_index])
                     || UTF8::is_not_following(array[third_index]) { return false; }
                else if array[index] == 0xE0 { if array[second_index] <= 0x9F { return false; } }
                else if array[index] == 0xED { if array[second_index] >= 0xA0 { return false; } }
                index += 3;
            } else if (array[index] & 0xF8) == 0xF0 {
                if four_index >= length { return false; }
                else if UTF8::is_not_following(array[second_index])
                     || UTF8::is_not_following(array[third_index])
                     || UTF8::is_not_following(array[four_index]) { return false; }
                else if array[index] == 0xF0 { if array[second_index] <= 0x8F { return false; } }
                else if array[index] == 0xF4 { if array[second_index] >= 0x90 { return false; } }
                else if array[index] > 0xF4 { return false; }
                index += 4;
            } else { return false; }
        }

        return true;
    }
}
