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

from typing import Union, Tuple, cast


class UTF8:

    @staticmethod
    def is_utf8(array: bytes) -> Union[bool, Exception]:

        """
        Функция проверяет исходную последовательность байт на когерентность кодировке UTF-8

        - Идеи:

        1) Оптимизация через SIMD (не реализуемо на Python)
        2) Оптимизация через параллельность

        :param array: Закодированная последовательность байт
        :return: Результат проверки последовательности на соответствие формату кодирования
        """

        (index, array_length) = cast(
            Tuple[int, int],
            (0, array.__len__())
        )

        while index < array_length:
            second_index, third_index, four_index = index + 1, index + 2, index + 3

            if   (array[index] & 0x80) == 0x00: index += 1; continue
            elif (array[index] & 0xE0) == 0xC0:
                if (array[index] & 0xFE) == 0xC0: return Exception(
                    ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                     f"0x{array[index]:02x}")
                )
                elif second_index >= array_length: return Exception(
                    "[UTF-8]: One of the next 3 octets is missing or the encoding format is not UTF-8"
                )
                elif (array[second_index] & 0xC0) != 0x80: return Exception(
                    ("[UTF-8]: The following decoded second octet does not correspond to UTF-8 encoding: "
                     f"0x{array[index]:02x}{array[second_index]:02x}")
                )

                index += 2; continue
            elif (array[index] & 0xF0) == 0xE0: # 3 octets/bytes
                if third_index >= array_length: return Exception(
                    "[UTF-8]: One of the next 3 octets is missing or the encoding format is not UTF-8"
                )
                elif (array[second_index] & 0xC0) != 0x80 \
                or   (array[third_index]  & 0xC0) != 0x80: return Exception(
                    ("[UTF-8]: One or more of the following decoded octets 2,3 do not correspond to UTF-8 encoding: "
                     f"0x{array[index]:02x}{array[second_index]:02x}{array[third_index]:02x}")
                )
                elif array[index] == 0xE0:
                    if 0x80 <= array[second_index] <= 0x9F: return Exception(
                        ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                         f"0x{array[index]:02x}{array[second_index]:02x}")
                    )
                elif array[index] == 0xED:
                    if 0xA0 <= array[second_index] <= 0xBF: return Exception(
                        ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                         f"0x{array[index]:02x}{array[second_index]:02x}")
                    )
                index += 3; continue
            elif (array[index] & 0xF8) == 0xF0: # 4 octets/bytes
                if four_index >= array_length: return Exception(
                    "[UTF-8]: One of the next 3 octets is missing or the encoding format is not UTF-8"
                )
                elif (array[second_index] & 0xC0) != 0x80 \
                or   (array[third_index]  & 0xC0) != 0x80 \
                or   (array[four_index]   & 0xC0) != 0x80: return Exception(
                    ("[UTF-8]: One or more of the following decoded octets 2,3,4 do not correspond to UTF-8 encoding: "
                     f"0x{array[index]:02x}{array[second_index]:02x}{array[third_index]:02x}{array[four_index]:02x}")
                )
                elif array[index] == 0xF0:
                    if 0x80 <= array[second_index] <= 0x8F: return Exception(
                        ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                         f"0x{array[index]:02x}{array[second_index]:02x}")
                    )
                elif array[index] == 0xF4:
                    if 0x90 <= array[second_index] <= 0xBF: return Exception(
                        ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                         f"0x{array[index]:02x}{array[second_index]:02x}")
                    )
                elif array[index] > 0xF4: return Exception(
                    ("[UTF-8]: The code range is not intended to be encoded/decoded in UTF-8: "
                     f"0x{array[index]:02x}")
                )
                index += 4; continue
            else: return Exception(
                ("[UTF-8]: Invalid UTF-8 encoding of HTTP header or the code range after 0x10FFFF "
                 "(0xF4:0x8F:0xBF:0xBF) is not assigned for encoding/decoding in UTF-8.")
            )

        return True
