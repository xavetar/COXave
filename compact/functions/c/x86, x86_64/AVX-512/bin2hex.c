/*
 * Copyright 2024 Stanislav Mikhailov (xavetar)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

#include <stdio.h>
#include <stdint.h>
#include <immintrin.h>

void bin2hex_avx512(const uint8_t* input, char* hex, size_t length) {
    const __m512i hex_table = _mm512_set_epi8(
        'F', 'E', 'D', 'C', 'B', 'A', '9', '8',
        '7', '6', '5', '4', '3', '2', '1', '0',
        'F', 'E', 'D', 'C', 'B', 'A', '9', '8',
        '7', '6', '5', '4', '3', '2', '1', '0',
        'F', 'E', 'D', 'C', 'B', 'A', '9', '8',
        '7', '6', '5', '4', '3', '2', '1', '0',
        'F', 'E', 'D', 'C', 'B', 'A', '9', '8',
        '7', '6', '5', '4', '3', '2', '1', '0'
    );

    const __m512i mask = _mm512_set1_epi8(0x0F);

    const __m512i permute_mask_part1 = _mm512_setr_epi64(0x00, 0x01, 0x08, 0x09, 0x02, 0x03, 0x0A, 0x0B);
    const __m512i permute_mask_part2 = _mm512_setr_epi64(0x04, 0x05, 0x0C, 0x0D, 0x06, 0x07, 0x0E, 0x0F);

    size_t i = 0;

    for (; i + 64 <= length; i += 64) {
        // Загружаем 32 байта данных
        __m512i data = _mm512_loadu_si512((__m512i*)(input + i));

        // Разделяем байты на старшие и младшие полубайты
        __m512i high_nibbles = _mm512_and_si512(_mm512_srli_epi16(data, 4), mask);
        __m512i low_nibbles = _mm512_and_si512(data, mask);

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        __m512i hex_high = _mm512_shuffle_epi8(hex_table, high_nibbles);
        __m512i hex_low = _mm512_shuffle_epi8(hex_table, low_nibbles);

        // Чередуем старшие и младшие полубайты
        __m512i hex_packed_even = _mm512_unpacklo_epi8(hex_high, hex_low);
        __m512i hex_packed_odd = _mm512_unpackhi_epi8(hex_high, hex_low);

        // Исправляем порядок, меняя местами куски между lane
        __m512i final_part1 = _mm512_permutex2var_epi64(hex_packed_even, permute_mask_part1, hex_packed_odd);
        __m512i final_part2 = _mm512_permutex2var_epi64(hex_packed_even, permute_mask_part2, hex_packed_odd);

        // Сохраняем результат (128 байт)
        _mm512_storeu_si512((__m512i*)(hex + 2 * i), final_part1);
        _mm512_storeu_si512((__m512i*)(hex + 2 * i + 64), final_part2);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = "0123456789ABCDEF"[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = "0123456789ABCDEF"[input[i] & 0x0F];
    }
}

int main() {
    char output[129] = {0};  // Удвоенный размер + нулевой терминатор

    uint8_t input[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                       0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
                       0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60,
                       0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70};

    bin2hex_avx512(input, output, sizeof(input));

    printf("Hex: %s\n", output);
    return 0;
}