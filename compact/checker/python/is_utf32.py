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
