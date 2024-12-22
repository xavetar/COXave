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

from typing import Union, Final


class ASCII:

    __ENCODING_BYTES: Final[int] = 1

    @staticmethod
    def is_ascii(array: bytes) -> Union[bool, Exception]:

        """
        Функция проверяет исходную последовательность байт на когерентность кодировке ASCII (от 0x00 до 0x7F)

        - Идеи:

        1) Оптимизация через большую маску
        2) Оптимизация через параллельность
        3) Оптимизация через SIMD (не реализуемо на Python)

        :param array: Закодированная последовательность байт
        :return: Результат проверки последовательности на соответствие формату кодирования
        """

        for byte in array:
            if (byte & 0x80) != 0x00:
                return Exception("Non-ASCII byte")
        else: return True
