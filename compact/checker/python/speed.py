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

from typing import List

import time
import random
import COXave

import is_ascii
import is_utf8
import is_utf16
import is_utf32


LENGTH_TO_TEST: int = 100_000_000

ASCII_CHARS:   List[str] = [chr(i) for i in range(0, 0x80) if chr(i)]

UNICODE_CHARS: List[str] = [chr(i) for i in range(0, 0xD800) if chr(i)] \
                         + [chr(i) for i in range(0xE000, 0x110000) if chr(i)]

RESULTS_ASCII:    List[float] = [0.0, 0.0, 0.0]
RESULTS_UTF8:     List[float] = [0.0, 0.0, 0.0]
RESULTS_UTF16_BE: List[float] = [0.0, 0.0, 0.0]
RESULTS_UTF16_LE: List[float] = [0.0, 0.0, 0.0]
RESULTS_UTF32_BE: List[float] = [0.0, 0.0, 0.0]
RESULTS_UTF32_LE: List[float] = [0.0, 0.0, 0.0]


def generate_ascii_string() -> str:
    return ''.join(random.choice(ASCII_CHARS) for _ in range(LENGTH_TO_TEST))


def generate_unicode_string() -> str:
    return ''.join(random.choice(UNICODE_CHARS) for _ in range(LENGTH_TO_TEST))


def measure_performance_ascii(encoded: bytes):
    start_time: float = time.time()

    COXave.ASCII.is_ascii(encoded)

    RESULTS_ASCII[0] += time.time() - start_time

    start_time = time.time()

    is_ascii.ASCII.is_ascii(encoded)

    RESULTS_ASCII[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('ascii')

    RESULTS_ASCII[2] += time.time() - start_time

    del encoded


def measure_performance_utf8(encoded: bytes):
    start_time: float = time.time()

    COXave.UTF8.is_utf8(encoded)

    RESULTS_UTF8[0] += time.time() - start_time

    start_time = time.time()

    is_utf8.UTF8.is_utf8(encoded)

    RESULTS_UTF8[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('utf-8')

    RESULTS_UTF8[2] += time.time() - start_time

    del encoded


def measure_performance_utf16_be(encoded: bytes):
    start_time: float = time.time()

    COXave.UTF16.is_utf16(encoded, endian=False, omp=True, only=False)

    RESULTS_UTF16_BE[0] += time.time() - start_time

    start_time = time.time()

    is_utf16.UTF16.is_utf16(encoded, endian=False)

    RESULTS_UTF16_BE[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('utf-16-be')

    RESULTS_UTF16_BE[2] += time.time() - start_time

    del encoded


def measure_performance_utf16_le(encoded: bytes):
    start_time: float = time.time()

    COXave.UTF16.is_utf16(encoded, endian=True, omp=True, only=False)

    RESULTS_UTF16_LE[0] += time.time() - start_time

    start_time = time.time()

    is_utf16.UTF16.is_utf16(encoded, endian=True)

    RESULTS_UTF16_LE[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('utf-16-le')

    RESULTS_UTF16_LE[2] += time.time() - start_time

    del encoded


def measure_performance_utf32_be(encoded: bytes):
    start_time: float = time.time()

    COXave.UTF32.is_utf32(encoded, endian=False)

    RESULTS_UTF32_BE[0] += time.time() - start_time

    start_time = time.time()

    is_utf32.UTF32.is_utf32(encoded, endian=False)

    RESULTS_UTF32_BE[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('utf-32-be')

    RESULTS_UTF32_BE[2] += time.time() - start_time

    del encoded


def measure_performance_utf32_le(encoded: bytes):
    start_time: float = time.time()

    COXave.UTF32.is_utf32(encoded, endian=True)

    RESULTS_UTF32_LE[0] += time.time() - start_time

    start_time = time.time()

    is_utf32.UTF32.is_utf32(encoded, endian=True)

    RESULTS_UTF32_LE[1] += time.time() - start_time

    start_time = time.time()

    encoded.decode('utf-32-le')

    RESULTS_UTF32_LE[2] += time.time() - start_time

    del encoded


if __name__ == "__main__":

    random_ascii_string: str = generate_ascii_string()
    random_unicode_string: str = generate_unicode_string()

    measure_performance_ascii(random_ascii_string.encode("ascii"))
    measure_performance_utf8(random_unicode_string.encode("utf-8"))
    measure_performance_utf16_be(random_unicode_string.encode("utf-16-be"))
    measure_performance_utf16_le(random_unicode_string.encode("utf-16-le"))
    measure_performance_utf32_be(random_unicode_string.encode("utf-32-be"))
    measure_performance_utf32_le(random_unicode_string.encode("utf-32-le"))

    print(
        f"[ASCII] Time: Rust (is_ascii)    = {RESULTS_ASCII[0]:.10f}s, Python Native (is_ascii) = {RESULTS_ASCII[1]:.10f}s, Python (decode) = {RESULTS_ASCII[2]:.10f}s\n"
        f"[UTF-8] Time: Rust (is_utf8)     = {RESULTS_UTF8[0]:.10f}s, Python Native (is_utf8) = {RESULTS_UTF8[1]:.10f}s, Python (decode) = {RESULTS_UTF8[2]:.10f}s\n"
        f"[UTF-16BE] Time: Rust (is_utf16) = {RESULTS_UTF16_BE[0]:.10f}s, Python Native (is_utf16) = {RESULTS_UTF16_BE[1]:.10f}s, Python (decode) = {RESULTS_UTF16_BE[2]:.10f}s\n"
        f"[UTF-16LE] Time: Rust (is_utf16) = {RESULTS_UTF16_LE[0]:.10f}s, Python Native (is_utf16) = {RESULTS_UTF16_LE[1]:.10f}s, Python (decode) = {RESULTS_UTF16_LE[2]:.10f}s\n"
        f"[UTF-32BE] Time: Rust (is_utf32) = {RESULTS_UTF32_BE[0]:.10f}s, Python Native (is_utf32) = {RESULTS_UTF32_BE[1]:.10f}s, Python (decode) = {RESULTS_UTF32_BE[2]:.10f}s\n"
        f"[UTF-32LE] Time: Rust (is_utf32) = {RESULTS_UTF32_LE[0]:.10f}s, Python Native (is_utf32) = {RESULTS_UTF32_LE[1]:.10f}s, Python (decode) = {RESULTS_UTF32_LE[2]:.10f}s\n"
    )
