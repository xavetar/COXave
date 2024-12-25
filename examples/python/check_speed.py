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

from typing import Callable, List

import time
import random
import COXave

"""
Равноценным сравнением это назвать нельзя Python работает с Unicode, decode фактически преобразовывает закодированные
последовательности в Unicode, тогда как мы фактически проверяем когерентность кодировки.
"""

LENGTH_TO_TEST: int = 100_000_000

ASCII_CHARS:      List[str] = [chr(i) for i in range(0, 0x80) if chr(i)]

UNICODE_CHARS:    List[str] = [chr(i) for i in range(0, 0xD800) if chr(i)] \
                            + [chr(i) for i in range(0xE000, 0x110000) if chr(i)]

RESULTS_ASCII:    List[float] = [0.0, 0.0]
RESULTS_UTF8:     List[float] = [0.0, 0.0]
RESULTS_UTF16_BE: List[float] = [0.0, 0.0]
RESULTS_UTF16_LE: List[float] = [0.0, 0.0]
RESULTS_UTF32_BE: List[float] = [0.0, 0.0]
RESULTS_UTF32_LE: List[float] = [0.0, 0.0]


def generate_ascii_string() -> str:
    return ''.join(random.choice(ASCII_CHARS) for _ in range(LENGTH_TO_TEST))


def generate_unicode_string() -> str:
    return ''.join(random.choice(UNICODE_CHARS) for _ in range(LENGTH_TO_TEST))


def measure_performance(rust_method: Callable, result: List[float], encoding: str, encoded: bytes, **kwargs):
    start_time: float = time.time()

    if kwargs:
        rust_method(encoded, **kwargs)
    else:
        rust_method(encoded)

    result[0] += time.time() - start_time

    start_time = time.time()

    encoded.decode(encoding)

    result[1] += time.time() - start_time

    del encoded


if __name__ == "__main__":

    random_ascii_string: str = generate_ascii_string()
    random_unicode_string: str = generate_unicode_string()

    measure_performance(
        rust_method=COXave.ASCII.is_ascii, result=RESULTS_ASCII, encoding="ascii",
        encoded=random_ascii_string.encode("ascii")
    )
    measure_performance(
        rust_method=COXave.UTF8.is_utf8, result=RESULTS_UTF8, encoding="utf-8",
        encoded=random_unicode_string.encode("utf-8")
    )
    measure_performance(
        rust_method=COXave.UTF16.is_utf16, result=RESULTS_UTF16_BE, encoding="utf-16-be",
        encoded=random_unicode_string.encode("utf-16-be"), endian=False, omp=True, only=False
    )
    measure_performance(
        rust_method=COXave.UTF16.is_utf16, result=RESULTS_UTF16_LE, encoding="utf-16-le",
        encoded=random_unicode_string.encode("utf-16-le"), endian=True, omp=True, only=False
    )
    measure_performance(
        rust_method=COXave.UTF32.is_utf32, result=RESULTS_UTF32_BE, encoding="utf-32-be",
        encoded=random_unicode_string.encode("utf-32-be"), endian=False
    )
    measure_performance(
        rust_method=COXave.UTF32.is_utf32, result=RESULTS_UTF32_LE, encoding="utf-32-le",
        encoded=random_unicode_string.encode("utf-32-le"), endian=True
    )

    print(
        f"[ASCII] Time: is_ascii    = {RESULTS_ASCII[0]:.10f}s, Python (decode) = {RESULTS_ASCII[1]:.10f}s\n"
        f"[UTF-8] Time: is_utf8     = {RESULTS_UTF8[0]:.10f}s, Python (decode) = {RESULTS_UTF8[1]:.10f}s\n"
        f"[UTF-16BE] Time: is_utf16 = {RESULTS_UTF16_BE[0]:.10f}s, Python (decode) = {RESULTS_UTF16_BE[1]:.10f}s\n"
        f"[UTF-16LE] Time: is_utf16 = {RESULTS_UTF16_LE[0]:.10f}s, Python (decode) = {RESULTS_UTF16_LE[1]:.10f}s\n"
        f"[UTF-32BE] Time: is_utf32 = {RESULTS_UTF32_BE[0]:.10f}s, Python (decode) = {RESULTS_UTF32_BE[1]:.10f}s\n"
        f"[UTF-32LE] Time: is_utf32 = {RESULTS_UTF32_LE[0]:.10f}s, Python (decode) = {RESULTS_UTF32_LE[1]:.10f}s\n"
    )
