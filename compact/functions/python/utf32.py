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


class UTF32:

    __ENCODING_BYTES: Final[int] = 4

    @staticmethod
    def is_utf32(array: bytes, endian: bool) -> Union[bool, Exception]:

        """
        Функция проверяет исходную последовательность байт на когерентность кодировке UTF-32

        - Идеи:

        1) Оптимизация через большую маску
        2) Оптимизация через параллельность
        3) Проверять BOM в паттерне и исходной последовательности, для авто-определения порядка байт

        :param array: Закодированная исходная последовательность байт
        :param endian: Порядок байт исходной закодированной последовательности
        :return: Результатом является, либо True, либо исключение (нужно вызвать или обработать)
        """

        if array.__len__() % UTF32.__ENCODING_BYTES != 0:
            return Exception(
                "[UTF-32 BE/LE]: The number of bytes must be a multiple of 4 octets to form character"
            )

        (index, array_length) = cast(
            Tuple[int, int],
            (0, array.__len__())
        )

        while index < array_length:

            second_index, third_index, four_index = index + 1, index + 2, index + 3

            if endian: first_byte, second_byte, third_byte, four_byte \
                     = array[four_index], array[third_index], array[second_index], array[index]
            else:      first_byte, second_byte, third_byte, four_byte \
                     = array[index], array[second_index], array[third_index], array[four_index]

            if first_byte == 0x00:
                if second_byte == 0x00:
                    if (third_byte & 0xF8) == 0xD8:
                        return Exception(
                            ("[UTF-32 BE/LE]: The code range from 0x0000D800 to 0x0000DFFF is not intended for "
                             "encoding/decoding in UTF-32: "
                             f"0x{first_byte:02x}{second_byte:02x}{third_byte:02x}{four_byte:02x}")
                        )
                elif second_byte > 0x10:
                    return Exception(
                        ("[UTF-32 BE/LE]: Invalid UTF32 encoding or the code range after 0x0010FFFF is not assigned "
                         "for encoding/decoding in UTF-32: "
                         f"0x{first_byte:02x}{second_byte:02x}{third_byte:02x}{four_byte:02x}")
                    )
                index += 4; continue
            else:
                return Exception(
                    ("[UTF-32 BE/LE]: Invalid UTF32 encoding the code range after 0x0010FFFF is not assigned "
                     "for encoding/decoding in UTF-32: "
                     f"0x{first_byte:02x}{second_byte:02x}{third_byte:02x}{four_byte:02x}")
                )

        return True

    @staticmethod
    def search_presentation_pattern(source: bytes, pattern: bytes, endian: bool, all_matches: bool, limit: Optional[int] = None) -> List[Tuple[int, int]]:

        """
        Функция поиска паттерна в исходном массиве байт

        - Идеи:

        1) Оптимизация поиска через параллельность
        2) Проверять BOM в паттерне и исходной последовательности, для авто-определения порядка байт

        :param source: Закодированная исходная последовательность байт в формате BE или LE
        :param pattern: Закодированная последовательность байт паттерна в формате BE или LE
        :param endian: Порядок байт исходной закодированной последовательности и паттерна (False - BE, True - LE)
        :param all_matches: Флаг, позволяет найти все вхождения паттерна в исходной последовательности байт
        :param limit: Лимит максимальной длины исходной последовательности символов для поиска
        :return: Список индексов начала и конца, байт паттерна в исходной последовательности байт
        """

        if limit is not None: source = source[:limit * UTF32.__ENCODING_BYTES]

        (source_is_utf32, pattern_is_utf32) = cast(
            Tuple[Union[bool, Exception], Union[bool, Exception]],
            (UTF32.is_utf32(source, endian), UTF32.is_utf32(pattern, endian))
        )

        if   source.__len__() == 0:  raise ValueError("[UTF-32 BE/LE]: The length of the source array is zero")
        elif pattern.__len__() == 0: raise ValueError("[UTF-32 BE/LE]: The length of the pattern array is zero")

        if   isinstance(source_is_utf32, Exception):  raise Exception("[UTF-32 BE/LE]: Source array is not UTF-32")
        elif isinstance(pattern_is_utf32, Exception): raise Exception("[UTF-32 BE/LE]: Pattern array is not UTF-32")

        (search_result, intermediate_data) = cast(
            Tuple[List[Tuple[int, int]], List[int]],
            ([], [pattern.__len__(), 0, 0])
        )

        for (index, byte) in enumerate(source[3::4]):

            first_byte_index:  int = index * UTF32.__ENCODING_BYTES
            second_byte_index: int = first_byte_index + 1
            third_byte_index:  int = second_byte_index + 1
            four_byte_index:   int = third_byte_index + 1

            first_pattern_index:  int = intermediate_data[1]
            second_pattern_index: int = first_pattern_index + 1
            third_pattern_index:  int = second_pattern_index + 1
            four_pattern_index:   int = third_pattern_index + 1

            inside: bool = False

            if byte == pattern[four_pattern_index]:
                if source[first_byte_index] == pattern[first_pattern_index]:
                    if source[third_byte_index] == pattern[third_pattern_index]:
                        if source[second_byte_index] == pattern[second_pattern_index]:
                            inside = True

                            if   intermediate_data[1] == 0: intermediate_data[1] += UTF32.__ENCODING_BYTES; intermediate_data[2] = first_byte_index
                            elif intermediate_data[1] != 0: intermediate_data[1] += UTF32.__ENCODING_BYTES

                            if intermediate_data[0] == intermediate_data[1]:
                                search_result.append((intermediate_data[2], four_byte_index + 1))
                                intermediate_data[1] = 0; intermediate_data[2] = 0

                                if not all_matches: return search_result

            if not inside:
                if intermediate_data[1] | intermediate_data[2]: intermediate_data[1] = 0; intermediate_data[2] = 0

        return search_result
