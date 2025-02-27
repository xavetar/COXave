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

void bin2hex_avx2(const uint8_t* input, char* hex, size_t length) {
    // Таблица для преобразования полубайтов в шестнадцатеричные символы
    const __m256i hex_table = _mm256_setr_epi8(
        '0', '1', '2', '3', '4', '5', '6', '7',
        '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
        '0', '1', '2', '3', '4', '5', '6', '7',
        '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'
    );

    size_t i = 0;

    for (; i + 32 <= length; i += 32) {
        // Загружаем 32 байта данных
        __m256i data = _mm256_loadu_si256((__m256i*)(input + i));

        // Выделяем старшие и младшие полубайты
        __m256i high_nibbles = _mm256_and_si256(_mm256_srli_epi16(data, 4), _mm256_set1_epi8(0x0F)); // Сдвиг вправо на 4 бита и обнуление в результирующей части старших 4-х бит
        __m256i low_nibbles = _mm256_and_si256(data, _mm256_set1_epi8(0x0F)); // Обнуление старших 4 бит

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        __m256i hex_high = _mm256_shuffle_epi8(hex_table, high_nibbles);
        __m256i hex_low = _mm256_shuffle_epi8(hex_table, low_nibbles);

        // Чередуем старшие и младшие полубайты
        __m256i low_result = _mm256_unpacklo_epi8(hex_high, hex_low);  // Нижние 8 байт каждого lane
        __m256i high_result = _mm256_unpackhi_epi8(hex_high, hex_low); // Верхние 8 байт каждого lane

        // Исправляем порядок, меняя местами куски между lane
        __m256i final_low = _mm256_permute2x128_si256(low_result, high_result, 0x20); // Нижний lane из low, верхний из high
        __m256i final_high = _mm256_permute2x128_si256(low_result, high_result, 0x31); // Верхний lane из low, нижний из high

        // Сохраняем результат (64 байта)
        _mm256_storeu_si256((__m256i*)(hex + i * 2), final_low);
        _mm256_storeu_si256((__m256i*)(hex + i * 2 + 32), final_high);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = "0123456789ABCDEF"[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = "0123456789ABCDEF"[input[i] & 0x0F];
    }
}

int main() {
    char hex[65] = {0xFF}; // Удвоенный размер + нулевой терминатор

    uint8_t input[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                       0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50};

    bin2hex_avx2(input, hex, sizeof(input));

    printf("Hex: %s\n", hex);
    return 0;
}
