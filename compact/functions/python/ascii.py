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

from typing import Union, Optional, List, Tuple, Final, cast


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

    @staticmethod
    def search_pattern(source: bytes, pattern: bytes, all_matches: bool, limit: Optional[int] = None) -> List[Tuple[int, int]]:

        """
        Функция поиска паттерна в исходном массиве байт

        - Идеи:

        1) Оптимизация поиска через SIMD (не реализуемо на Python)
        1) Оптимизация поиска через параллельность

        :param source: Закодированная исходная последовательность байт
        :param pattern: Закодированная последовательность байт паттерна
        :param limit: Лимит максимальной длины исходной последовательности символов для поиска
        :param all_matches: Флаг, позволяет найти все вхождения паттерна в исходной последовательности байт
        :return: Список индексов начала и конца, байт паттерна в исходной последовательности байт
        """

        if limit is not None: source = source[:limit * ASCII.__ENCODING_BYTES]

        (source_is_ascii, pattern_is_ascii) = cast(
            Tuple[Union[bool, Exception], Union[bool, Exception]],
            (ASCII.is_ascii(source), ASCII.is_ascii(pattern))
        )

        if   source.__len__() == 0:  raise ValueError("[ASCII]: The length of the source array is zero")
        elif pattern.__len__() == 0: raise ValueError("[ASCII]: The length of the pattern array is zero")

        if   isinstance(source_is_ascii, Exception):  raise Exception("[ASCII]: Source array is not ASCII")
        elif isinstance(pattern_is_ascii, Exception): raise Exception("[ASCII]: Pattern array is not ASCII")

        (search_result, intermediate_data) = cast(
            Tuple[List[Tuple[int, int]], List[int]],
            ([], [pattern.__len__(), 0, 0])
        )

        for (index, byte) in enumerate(source):
            if byte == pattern[intermediate_data[1]]:
                if   intermediate_data[1] == 0: intermediate_data[1] += 1; intermediate_data[2] = index
                elif intermediate_data[1] != 0: intermediate_data[1] += 1

                if intermediate_data[0] == intermediate_data[1]:
                    search_result.append((intermediate_data[2], index + 1))
                    intermediate_data[1] = 0; intermediate_data[2] = 0

                    if not all_matches: return search_result
            else: intermediate_data[1] = 0; intermediate_data[2] = 0

        return search_result
