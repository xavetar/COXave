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


LENGTH_TO_TEST: int = 100_000_000

ASCII_CHARS:      List[str] = [chr(i) for i in range(0, 0x80) if chr(i)]

RESULTS_ASCII:    List[float] = [0.0, 0.0]
RESULTS_UTF32_BE: List[float] = [0.0, 0.0]
RESULTS_UTF32_LE: List[float] = [0.0, 0.0]

def generate_ascii_string() -> str:
    return ''.join(random.choice(ASCII_CHARS) for _ in range(LENGTH_TO_TEST))


def measure_performance(rust_method: Callable, result: List[float],
                        encoded: bytes, pattern: bytes, all_matches: bool, **kwargs):

    start_time: float = time.time()

    if kwargs:
        rust_method(encoded, pattern, all_matches=all_matches, limit=None, **kwargs)
    else:
        rust_method(encoded, pattern, all_matches=all_matches, limit=None)

    result[0] += time.time() - start_time

    start_time = time.time()

    encoded.find(pattern)

    result[1] += time.time() - start_time

    del encoded


if __name__ == "__main__":

    pattern_string: str = "test1234567890!@#$%^&*()"

    random_ascii_string: str = generate_ascii_string() + pattern_string

    measure_performance(
        rust_method=COXave.ASCII.search_pattern, result=RESULTS_ASCII,
        encoded=random_ascii_string.encode("ascii"), pattern=pattern_string.encode("ascii"), all_matches=True
    )

    measure_performance(
        rust_method=COXave.UTF32.search_pattern, result=RESULTS_UTF32_LE,
        encoded=random_ascii_string.encode("utf-32-be"), pattern=pattern_string.encode("utf-32-be"), all_matches=True, endian=False
    )

    measure_performance(
        rust_method=COXave.UTF32.search_pattern, result=RESULTS_UTF32_BE,
        encoded=random_ascii_string.encode("utf-32-le"), pattern=pattern_string.encode("utf-32-le"), all_matches=True, endian=True
    )

    print(
        f"[ASCII] Time: search_pattern = {RESULTS_ASCII[0]:.10f}s, Python (find) = {RESULTS_ASCII[1]:.10f}s\n"
        f"[UTF-32BE] Time: search_pattern = {RESULTS_UTF32_BE[0]:.10f}s, Python (find) = {RESULTS_UTF32_BE[1]:.10f}s\n"
        f"[UTF-32LE] Time: search_pattern = {RESULTS_UTF32_LE[0]:.10f}s, Python (find) = {RESULTS_UTF32_LE[1]:.10f}s\n"
    )
