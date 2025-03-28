# Copyright 2024 Stanislav Mikhailov (xavetar)
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.

from typing import Callable

import COXave
import random

NUMBER_OF_TESTS: int = 1_000_000

MIN_LENGTH_TEST: int = 1
MAX_LENGTH_TEST: int = 300


def generate_random_bytes(length):
    return bytes(random.getrandbits(8) for _ in range(length))


def fuzz(rust_method: Callable, encoding: str, **kwargs):
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        if kwargs:
            result: bool = rust_method(random_bytes, **kwargs)
        else:
            result: bool = rust_method(random_bytes)

        try:
            random_bytes.decode(encoding)
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[{encoding.upper()}] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


if __name__ == "__main__":

    fuzz(COXave.ASCII.is_ascii, "ascii")
    fuzz(COXave.UTF8.is_utf8, "utf-8")
    fuzz(COXave.UTF16.is_utf16, "utf-16-be", endian=False, omp=True, only=False)
    fuzz(COXave.UTF16.is_utf16, "utf-16-le", endian=True, omp=True, only=False)
    fuzz(COXave.UTF32.is_utf32, "utf-32-be", endian=False)
    fuzz(COXave.UTF32.is_utf32, "utf-32-le", endian=True)
