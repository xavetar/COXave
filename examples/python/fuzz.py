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

import COXave
import random

NUMBER_OF_TESTS: int = 1000000

MIN_LENGTH_TEST: int = 1
MAX_LENGTH_TEST: int = 300


def generate_random_bytes(length):
    return bytes(random.getrandbits(8) for _ in range(length))


def fuzz_is_ascii():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.ASCII.is_ascii(random_bytes)

        try:
            random_bytes.decode('ascii')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[ASCII] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


def fuzz_is_utf8():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.UTF8.is_utf8(random_bytes)

        try:
            random_bytes.decode('utf-8')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[UTF-8] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


def fuzz_is_utf16be():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.UTF16.is_utf16(random_bytes, endian=False, omp=True, only=False)

        try:
            random_bytes.decode('utf-16-be')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[UTF-16-BE] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


def fuzz_is_utf16le():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.UTF16.is_utf16(random_bytes, endian=True, omp=True, only=False)

        try:
            random_bytes.decode('utf-16-le')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[UTF-16-LE] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


def fuzz_is_utf32be():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.UTF32.is_utf32(random_bytes, endian=False)

        try:
            random_bytes.decode('utf-32-be')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[UTF-32-BE] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


def fuzz_is_utf32le():
    for _ in range(NUMBER_OF_TESTS):
        random_bytes: bytes = generate_random_bytes(random.randint(MIN_LENGTH_TEST, MAX_LENGTH_TEST))

        result: bool = COXave.UTF32.is_utf32(random_bytes, endian=True)

        try:
            random_bytes.decode('utf-32-le')
            native_result: bool = True
        except UnicodeDecodeError:
            native_result: bool = False

        if result != native_result:
            print(f"[UTF-32-LE] Mismatch found! Input: {random_bytes} | My Result: {result} | Native Result: {native_result}")


if __name__ == "__main__":

    fuzz_is_utf8()
    fuzz_is_utf16le()
    fuzz_is_utf16be()
    fuzz_is_utf32le()
    fuzz_is_utf32be()
