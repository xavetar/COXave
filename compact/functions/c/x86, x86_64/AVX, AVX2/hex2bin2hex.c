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
#include <stdbool.h>
#include <immintrin.h>

// Определяем структуру с выравниванием на 32 байт
struct HexChars {
    uint8_t chars[32];
} __attribute__((aligned(32))); // Выравниваем структуру на 32 байт

// Таблицы для преобразования полубайтов в шестнадцатеричные символы
const struct HexChars ASCII_HEX_CHARS_UPPER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'} };
const struct HexChars ASCII_HEX_CHARS_LOWER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'} };

void hex2bin(const uint8_t* hex, uint8_t* bin, size_t hex_len) {
    // Проверка на четность длины
    if (hex_len % 2 != 0) {
        printf("Error: Input length must be even\n");
        return;
    }

   // Общие константы (ASCII)
    const __m256i OFFSET_ASCII_DIGIT                  = _mm256_set1_epi8(0x30); // '0'
    const __m256i OFFSET_ASCII_ALPHABET_UPPER         = _mm256_set1_epi8(0x37); // 'A' - 10
    const __m256i OFFSET_ASCII_ALPHABET_LOWER         = _mm256_set1_epi8(0x57); // 'a' - 10

    const __m256i ASCII_TABLE_DIGITS_AFTER            = _mm256_set1_epi8(0x2F); // '0' - 1
    const __m256i ASCII_TABLE_DIGITS_BEFORE           = _mm256_set1_epi8(0x3A); // '9' + 1
    const __m256i ASCII_TABLE_ALPHABET_CAPITAL_AFTER  = _mm256_set1_epi8(0x40); // 'A' - 1
    const __m256i ASCII_TABLE_ALPHABET_CAPITAL_BEFORE = _mm256_set1_epi8(0x47); // 'F' - 1
    const __m256i ASCII_TABLE_ALPHABET_SMALL_AFTER    = _mm256_set1_epi8(0x60); // 'a' - 1
    const __m256i ASCII_TABLE_ALPHABET_SMALL_BEFORE   = _mm256_set1_epi8(0x67); // 'f' - 1

    const __m256i SECOND_SHUFFLE = _mm256_setr_epi8(
        1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15, -1,
        1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15, -1
    );

    const __m256i MASK_SECOND_BYTE_TO_PACK = _mm256_set1_epi16(0x00FF);

    const int PERMUTE_MASK_ORDER_CORRECTION = _MM_SHUFFLE(3, 1, 2, 0);

    size_t i = 0;

    // Обработка по 64 символа (32 байта результата) за раз
    for (; i + 64 <= hex_len; i += 64) {
        // Загружаем 64 символа (32 пары)
        __m256i chars_first = _mm256_loadu_si256((__m256i*)(hex + i));
        __m256i chars_second = _mm256_loadu_si256((__m256i*)(hex + i + 32));

        // Преобразуем первые 32 символа (chars_first)
        __m256i digit_mask_first = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_first, ASCII_TABLE_DIGITS_AFTER),            // a > '0' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_DIGITS_BEFORE, chars_first)            //'9' + 1 > a
        );
        __m256i upper_mask_first = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_first, ASCII_TABLE_ALPHABET_CAPITAL_AFTER),  // a > 'A' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_ALPHABET_CAPITAL_BEFORE, chars_first)  // 'F' + 1 > a
        );
        __m256i lower_mask_first = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_first, ASCII_TABLE_ALPHABET_SMALL_AFTER),    // a > 'a' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_ALPHABET_SMALL_BEFORE, chars_first)    // 'f' + 1 > a
        );

        // Преобразуем вторые 32 символа (chars_second)
        __m256i digit_mask_second = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_second, ASCII_TABLE_DIGITS_AFTER),           // a > '0' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_DIGITS_BEFORE, chars_second)           //'9' + 1 > a
        );
        __m256i upper_mask_second = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_second, ASCII_TABLE_ALPHABET_CAPITAL_AFTER), // a > 'A' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_ALPHABET_CAPITAL_BEFORE, chars_second) // 'F' + 1 > a
        );
        __m256i lower_mask_second = _mm256_and_si256(
            _mm256_cmpgt_epi8(chars_second, ASCII_TABLE_ALPHABET_SMALL_AFTER),   // a > 'a' - 1
            _mm256_cmpgt_epi8(ASCII_TABLE_ALPHABET_SMALL_BEFORE, chars_second)   // 'f' + 1 > a
        );

        __m256i digits_first = _mm256_and_si256(digit_mask_first, _mm256_sub_epi8(chars_first, OFFSET_ASCII_DIGIT));
        __m256i uppers_first = _mm256_and_si256(upper_mask_first, _mm256_sub_epi8(chars_first, OFFSET_ASCII_ALPHABET_UPPER));
        __m256i lowers_first = _mm256_and_si256(lower_mask_first, _mm256_sub_epi8(chars_first, OFFSET_ASCII_ALPHABET_LOWER));

        __m256i digits_second = _mm256_and_si256(digit_mask_second, _mm256_sub_epi8(chars_second, OFFSET_ASCII_DIGIT));
        __m256i uppers_second = _mm256_and_si256(upper_mask_second, _mm256_sub_epi8(chars_second, OFFSET_ASCII_ALPHABET_UPPER));
        __m256i lowers_second = _mm256_and_si256(lower_mask_second, _mm256_sub_epi8(chars_second, OFFSET_ASCII_ALPHABET_LOWER));

        __m256i values_first = _mm256_or_si256(digits_first, _mm256_or_si256(uppers_first, lowers_first));     // 04 08 06 05 06 0C 06 0C 06 0F 02 00 03 01 03 02 03 03 03 04 03 05 03 06 03 07 03 08 03 09 03 00
        __m256i values_second = _mm256_or_si256(digits_second, _mm256_or_si256(uppers_second, lowers_second)); // 04 01 04 02 04 03 04 04 04 05 04 06 04 07 04 08 04 09 04 0A 04 0B 04 0C 04 0D 04 0E 04 0F 05 00

        // AVX2: Используем _mm256_shuffle_epi8 для извлечения вторых символов
        __m256i shifted_high_and_low_to_msb_first = _mm256_slli_epi16(values_first, 4);                        // 40 80 60 50 60 C0 60 C0 60 F0 20 00 30 10 30 20 30 30 30 40 30 50 30 60 30 70 30 80 30 90 30 00
        __m256i shifted_high_and_low_to_msb_second = _mm256_slli_epi16(values_second, 4);                      // 40 10 40 20 40 30 40 40 40 50 40 60 40 70 40 80 40 90 40 A0 40 B0 40 C0 40 D0 40 E0 40 F0 50 00

        __m256i low_hex_to_lsb_first = _mm256_shuffle_epi8(values_first, SECOND_SHUFFLE);                      // 08 00 05 00 0C 00 0C 00 0F 00 00 00 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 00 00
        __m256i low_hex_to_lsb_second = _mm256_shuffle_epi8(values_second, SECOND_SHUFFLE);                    // 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 0A 00 0B 00 0C 00 0D 00 0E 00 0F 00 00 00

        __m256i result_first = _mm256_or_si256(shifted_high_and_low_to_msb_first, low_hex_to_lsb_first);       // 48 80 65 50 6C C0 6C C0 6F F0 20 00 31 10 32 20 33 30 34 40 35 50 36 60 37 70 38 80 39 90 30 00
        __m256i result_second = _mm256_or_si256(shifted_high_and_low_to_msb_second, low_hex_to_lsb_second);    // 41 10 42 20 43 30 44 40 45 50 46 60 47 70 48 80 49 90 4A A0 4B B0 4C C0 4D D0 4E E0 4F F0 50 00

        __m256i packed_result = _mm256_packus_epi16(
            _mm256_and_si256(result_first, MASK_SECOND_BYTE_TO_PACK),                                          // 48 00 65 00 6C 00 6C 00 6F 00 20 00 31 00 32 00 33 00 34 00 35 00 36 00 37 00 38 00 39 00 30 00
            _mm256_and_si256(result_second, MASK_SECOND_BYTE_TO_PACK)                                          // 41 00 42 00 43 00 44 00 45 00 46 00 47 00 48 00 49 00 4A 00 4B 00 4C 00 4D 00 4E 00 4F 00 50 00
        );                                                                                                     // 48 65 6C 6C 6F 20 31 32 41 42 43 44 45 46 47 48 33 34 35 36 37 38 39 30 49 4A 4B 4C 4D 4E 4F 50

        // Исправляем порядок lane в 256-битном векторе
        __m256i final_result = _mm256_permute4x64_epi64(packed_result, PERMUTE_MASK_ORDER_CORRECTION);         // 48 65 6C 6C 6F 20 31 32 33 34 35 36 37 38 39 30 41 42 43 44 45 46 47 48 49 4A 4B 4C 4D 4E 4F 50

        // Сохраняем 32 байта результата
        _mm256_storeu_si256((__m256i*)(bin + i / 2), final_result);
    }

    // Обработка остатка
    for (; i + 2 <= hex_len; i += 2) {
        uint8_t first = hex[i];
        uint8_t second = hex[i + 1];

        first = (first >= '0' && first <= '9') ? (first - '0') :
                (first >= 'A' && first <= 'F') ? (first - 'A' + 10) :
                (first >= 'a' && first <= 'f') ? (first - 'a' + 10) : 0;

        second = (second >= '0' && second <= '9') ? (second - '0') :
                 (second >= 'A' && second <= 'F') ? (second - 'A' + 10) :
                 (second >= 'a' && second <= 'f') ? (second - 'a' + 10) : 0;

        bin[i / 2] = (first << 4) | second;
    }
}

void bin2hex(const uint8_t* input, char* hex, bool _case, size_t length) {
    // Определяем таблицу для преобразования полубайтов в шестнадцатеричные символы
    const struct HexChars* CHARS = _case ? &ASCII_HEX_CHARS_LOWER : &ASCII_HEX_CHARS_UPPER;

    const __m256i HEX_TABLE = _mm256_load_si256((__m256i*) CHARS);

    const __m256i MASK_LOW_NIBBLE = _mm256_set1_epi8(0x0F);

    size_t i = 0;

    // Обработка по 32 байта (64 символа результата) за раз
    for (; i + 32 <= length; i += 32) {
        // Загружаем 32 байта данных
        __m256i data = _mm256_loadu_si256((__m256i*)(input + i));

        // Выделяем старшие и младшие полубайты
        __m256i high_nibbles = _mm256_and_si256(_mm256_srli_epi16(data, 4), MASK_LOW_NIBBLE); // Сдвиг вправо на 4 бита и обнуление в результирующей части старших 4-х бит
        __m256i low_nibbles = _mm256_and_si256(data, MASK_LOW_NIBBLE);                        // Обнуление старших 4 бит

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        __m256i hex_high = _mm256_shuffle_epi8(HEX_TABLE, high_nibbles);
        __m256i hex_low = _mm256_shuffle_epi8(HEX_TABLE, low_nibbles);

        // Чередуем старшие и младшие полубайты
        __m256i low_result = _mm256_unpacklo_epi8(hex_high, hex_low);                         // Нижние 8 байт каждого lane
        __m256i high_result = _mm256_unpackhi_epi8(hex_high, hex_low);                        // Верхние 8 байт каждого lane

        // Исправляем порядок, меняя местами куски между lane
        __m256i final_low = _mm256_permute2x128_si256(low_result, high_result, 0x20);         // Нижний lane из low, верхний из high
        __m256i final_high = _mm256_permute2x128_si256(low_result, high_result, 0x31);        // Верхний lane из low, нижний из high

        // Сохраняем результат (64 байта)
        _mm256_storeu_si256((__m256i*)(hex + i * 2), final_low);
        _mm256_storeu_si256((__m256i*)(hex + i * 2 + 32), final_high);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = (*CHARS).chars[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = (*CHARS).chars[input[i] & 0x0F];
    }
}

void test_bin2hex() {
    char hex_result[67] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[33] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                         0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
                         0x48};

    bin2hex(input, hex_result, false, sizeof(input));

    printf("Input Binary (bin2hex): ");
    for (int i = 0; i < sizeof(input); i++) {
        printf("%02X ", input[i]);
    }
    printf("\nOutput HEX (bin2hex): %s\n", hex_result);
}

void test_hex2bin2hex() {
    char input[67] = {'4', '8', '6', '5', '6', 'C', '6', 'C', '6', 'F', '2', '0', '3', '1', '3', '2',
                      '3', '3', '3', '4', '3', '5', '3', '6', '3', '7', '3', '8', '3', '9', '3', '0',
                      '4', '1', '4', '2', '4', '3', '4', '4', '4', '5', '4', '6', '4', '7', '4', '8',
                      '4', '9', '4', 'A', '4', 'B', '4', 'C', '4', 'D', '4', 'E', '4', 'F', '5', '0',
                      '4', '8','\0'};
    uint8_t binary[33] = {0};
    char hex_result[67] = {0};

    hex2bin((uint8_t*) input, binary, sizeof(input) - 1);
    bin2hex(binary, hex_result, false, sizeof(binary));

    printf("Original Input (hex2bin2hex): %s\n", input);
    printf("Result Binary (hex2bin2hex): ");
    for (int i = 0; i < sizeof(binary); i++) {
        printf("%02X ", binary[i]);
    }
    printf("\nConverted back (hex2bin2hex): %s\n", hex_result);
}

int main() {
    test_bin2hex();
    test_hex2bin2hex();

    return 0;
}