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

from typing import Union, Tuple, Final, cast


class UTF16:

    __ENCODING_REGULAR_PAIR_BYTES:   Final[int] = 2
    __ENCODING_SURROGATE_PAIR_BYTES: Final[int] = 4

    @staticmethod
    def is_utf16(array: bytes, endian: bool) -> Union[bool, Exception]:

        """
        Функция проверяет исходную последовательность байт на когерентность кодировке UTF-16

        - Идеи:

        1) Оптимизация через SIMD (не реализуемо на Python)
        2) Оптимизация через параллельность
        3) Проверять BOM в паттерне и исходной последовательности, для авто-определения порядка байт

        :param array: Закодированная исходная последовательность байт
        :param endian: Порядок байт исходной закодированной последовательности
        :return: Результатом является, либо True, либо исключение (нужно вызвать или обработать)
        """

        if array.__len__() % UTF16.__ENCODING_REGULAR_PAIR_BYTES != 0: return Exception(
            "[UTF-16 BE/LE]: The number of bytes must be a multiple of 2 octets to form regular/surrogate pair/character"
        )

        (index, array_length) = cast(
            Tuple[int, int],
            (0, array.__len__())
        )

        while index < array_length:

            second_index: int = index + 1

            if endian: first_byte, second_byte = array[second_index], array[index]
            else:      first_byte, second_byte = array[index], array[second_index]

            if first_byte <= 0xD7 or first_byte >= 0xE0:
                index += 2; continue
            elif 0xD8 <= first_byte <= 0xDB:

                third_index: int = second_index + 1
                four_index:  int = third_index + 1

                if array.__len__() - (four_index + 1) < 0:
                    return Exception(
                        "[UTF-16 BE/LE]: The next 3 octets are insufficient to form a surrogate pair/character"
                    )

                if endian: third_byte, four_byte = array[four_index], array[third_index]
                else:      third_byte, four_byte = array[third_index], array[four_index]

                if 0xDC <= third_byte <= 0xDF:
                    index += 4; continue
                else:
                    return Exception(
                        ("[UTF-16 BE/LE]: The value must be a surrogate pair, but the lower surrogate is not a valid "
                         f"value: 0x{first_byte:02x}{second_byte:02x}{third_byte:02x}{four_byte:02x}")
                    )
            else: return Exception(
                ("[UTF-16 BE/LE]: Invalid UTF-16 encoding or the code range after 0x10FFFF (0xDBFF:0xDFFF) is not "
                 f"assigned for encoding/decoding in UTF-16: 0x{first_byte:02x}{second_byte:02x}")
            )

        return True
