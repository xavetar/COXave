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

// Определяем макрос для выравнивания
#if defined(_MSC_VER) // MSVC
    #define ALIGNED(x) __declspec(align(x))
#else // GCC/Clang
    #define ALIGNED(x) __attribute__((aligned(x)))
#endif

// Определяем структуру с выравниванием на 64 байт
struct HexChars {
    uint8_t chars[64];
} ALIGNED(64); // Выравниваем структуру на 64 байт

// Таблицы для преобразования полубайтов в шестнадцатеричные символы
const struct HexChars ASCII_HEX_CHARS_UPPER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'} };
const struct HexChars ASCII_HEX_CHARS_LOWER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
                                                          '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'} };

void hex2bin(const uint8_t* hex, uint8_t* bin, size_t hex_len) {
    // Проверка на четность длины
    if (hex_len % 2 != 0) {
        printf("Error: Input length must be even\n");
        return;
    }

    // Общие константы (ASCII)
    const __m512i OFFSET_ASCII_DIGIT                  = _mm512_set1_epi8(0x30); // '0'
    const __m512i OFFSET_ASCII_ALPHABET_UPPER         = _mm512_set1_epi8(0x37); // 'A' - 10
    const __m512i OFFSET_ASCII_ALPHABET_LOWER         = _mm512_set1_epi8(0x57); // 'a' - 10

    const __m512i ASCII_TABLE_DIGITS_AFTER            = _mm512_set1_epi8(0x2F); // '0' - 1
    const __m512i ASCII_TABLE_DIGITS_BEFORE           = _mm512_set1_epi8(0x3A); // '9' + 1
    const __m512i ASCII_TABLE_ALPHABET_CAPITAL_AFTER  = _mm512_set1_epi8(0x40); // 'A' - 1
    const __m512i ASCII_TABLE_ALPHABET_CAPITAL_BEFORE = _mm512_set1_epi8(0x47); // 'F' - 1
    const __m512i ASCII_TABLE_ALPHABET_SMALL_AFTER    = _mm512_set1_epi8(0x60); // 'a' - 1
    const __m512i ASCII_TABLE_ALPHABET_SMALL_BEFORE   = _mm512_set1_epi8(0x67); // 'f' - 1

    const __m512i SECOND_SHUFFLE = _mm512_set_epi8(
        -1, 15, -1, 13, -1, 11, -1, 9, -1, 7, -1, 5, -1, 3, -1, 1,
        -1, 15, -1, 13, -1, 11, -1, 9, -1, 7, -1, 5, -1, 3, -1, 1,
        -1, 15, -1, 13, -1, 11, -1, 9, -1, 7, -1, 5, -1, 3, -1, 1,
        -1, 15, -1, 13, -1, 11, -1, 9, -1, 7, -1, 5, -1, 3, -1, 1
    );

    const __m512i MASK_SECOND_BYTE_TO_PACK = _mm512_set1_epi16(0x00FF);

    const __m512i PERMUTE_MASK_ORDER_CORRECTION = _mm512_setr_epi64(0, 2, 4, 6, 1, 3, 5, 7);

    size_t i = 0;

    // Обработка по 128 символов (64 байта результата) за раз
    for (; i + 128 <= hex_len; i += 128) {
        // Загружаем 128 символов (64 пары)
        __m512i chars_first = _mm512_loadu_si512((__m512i*) (hex + i));
        __m512i chars_second = _mm512_loadu_si512((__m512i*) (hex + i + 64));

        // Преобразуем первые 64 символа (chars_first)
        __mmask64 digit_mask_first = _mm512_cmpgt_epi8_mask(chars_first, ASCII_TABLE_DIGITS_AFTER) \
                                   & _mm512_cmplt_epi8_mask(chars_first, ASCII_TABLE_DIGITS_BEFORE);
        __mmask64 upper_mask_first = _mm512_cmpgt_epi8_mask(chars_first, ASCII_TABLE_ALPHABET_CAPITAL_AFTER) \
                                   & _mm512_cmplt_epi8_mask(chars_first, ASCII_TABLE_ALPHABET_CAPITAL_BEFORE);
        __mmask64 lower_mask_first = _mm512_cmpgt_epi8_mask(chars_first, ASCII_TABLE_ALPHABET_SMALL_AFTER) \
                                   & _mm512_cmplt_epi8_mask(chars_first, ASCII_TABLE_ALPHABET_SMALL_BEFORE);

        // Преобразуем вторые 64 символа (chars_second)
        __mmask64 digit_mask_second = _mm512_cmpgt_epi8_mask(chars_second, ASCII_TABLE_DIGITS_AFTER) \
                                    & _mm512_cmplt_epi8_mask(chars_second, ASCII_TABLE_DIGITS_BEFORE);
        __mmask64 upper_mask_second = _mm512_cmpgt_epi8_mask(chars_second, ASCII_TABLE_ALPHABET_CAPITAL_AFTER) \
                                    & _mm512_cmplt_epi8_mask(chars_second, ASCII_TABLE_ALPHABET_CAPITAL_BEFORE);
        __mmask64 lower_mask_second = _mm512_cmpgt_epi8_mask(chars_second, ASCII_TABLE_ALPHABET_SMALL_AFTER) \
                                    & _mm512_cmplt_epi8_mask(chars_second, ASCII_TABLE_ALPHABET_SMALL_BEFORE);

        __m512i digits_first = _mm512_maskz_sub_epi8(digit_mask_first, chars_first, OFFSET_ASCII_DIGIT);
        __m512i uppers_first = _mm512_maskz_sub_epi8(upper_mask_first, chars_first, OFFSET_ASCII_ALPHABET_UPPER);
        __m512i lowers_first = _mm512_maskz_sub_epi8(lower_mask_first, chars_first, OFFSET_ASCII_ALPHABET_LOWER);

        __m512i digits_second = _mm512_maskz_sub_epi8(digit_mask_second, chars_second, OFFSET_ASCII_DIGIT);
        __m512i uppers_second = _mm512_maskz_sub_epi8(upper_mask_second, chars_second, OFFSET_ASCII_ALPHABET_UPPER);
        __m512i lowers_second = _mm512_maskz_sub_epi8(lower_mask_second, chars_second, OFFSET_ASCII_ALPHABET_LOWER);

        __m512i values_first = _mm512_or_si512(digits_first, _mm512_or_si512(uppers_first, lowers_first));     // 04 08 06 05 06 0C 06 0C 06 0F 02 00 03 01 03 02 03 03 03 04 03 05 03 06 03 07 03 08 03 09 03 00 04 01 04 02 04 03 04 04 04 05 04 06 04 07 04 08 04 09 04 0A 04 0B 04 0C 04 0D 04 0E 04 0F 05 00
        __m512i values_second = _mm512_or_si512(digits_second, _mm512_or_si512(uppers_second, lowers_second)); // 05 01 05 02 05 03 05 04 05 05 05 06 05 07 05 08 05 09 05 0A 05 0B 05 0C 05 0D 05 0E 05 0F 06 00 06 01 06 02 06 03 06 04 06 05 06 06 06 07 06 08 06 09 06 0A 06 0B 06 0C 06 0D 06 0E 06 0F 07 00

        // AVX-512: Используем _mm512_shuffle_epi8 для извлечения вторых символов
        __m512i shifted_high_and_low_to_msb_first = _mm512_slli_epi16(values_first, 4);                        // 40 80 60 50 60 C0 60 C0 60 F0 20 00 30 10 30 20 30 30 30 40 30 50 30 60 30 70 30 80 30 90 30 00 40 10 40 20 40 30 40 40 40 50 40 60 40 70 40 80 40 90 40 A0 40 B0 40 C0 40 D0 40 E0 40 F0 50 00
        __m512i shifted_high_and_low_to_msb_second = _mm512_slli_epi16(values_second, 4);                      // 50 10 50 20 50 30 50 40 50 50 50 60 50 70 50 80 50 90 50 A0 50 B0 50 C0 50 D0 50 E0 50 F0 60 00 60 10 60 20 60 30 60 40 60 50 60 60 60 70 60 80 60 90 60 A0 60 B0 60 C0 60 D0 60 E0 60 F0 70 00

        __m512i low_hex_to_lsb_first = _mm512_shuffle_epi8(values_first, SECOND_SHUFFLE);                      // 08 00 05 00 0C 00 0C 00 0F 00 00 00 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 00 00 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 0A 00 0B 00 0C 00 0D 00 0E 00 0F 00 00 00
        __m512i low_hex_to_lsb_second = _mm512_shuffle_epi8(values_second, SECOND_SHUFFLE);                    // 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 0A 00 0B 00 0C 00 0D 00 0E 00 0F 00 00 00 01 00 02 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 0A 00 0B 00 0C 00 0D 00 0E 00 0F 00 00 00

        __m512i result_first = _mm512_or_si512(shifted_high_and_low_to_msb_first, low_hex_to_lsb_first);       // 48 80 65 50 6C C0 6C C0 6F F0 20 00 31 10 32 20 33 30 34 40 35 50 36 60 37 70 38 80 39 90 30 00 41 10 42 20 43 30 44 40 45 50 46 60 47 70 48 80 49 90 4A A0 4B B0 4C C0 4D D0 4E E0 4F F0 50 00
        __m512i result_second = _mm512_or_si512(shifted_high_and_low_to_msb_second, low_hex_to_lsb_second);    // 51 10 52 20 53 30 54 40 55 50 56 60 57 70 58 80 59 90 5A A0 5B B0 5C C0 5D D0 5E E0 5F F0 60 00 61 10 62 20 63 30 64 40 65 50 66 60 67 70 68 80 69 90 6A A0 6B B0 6C C0 6D D0 6E E0 6F F0 70 00

        __m512i packed_result = _mm512_packus_epi16(
            _mm512_and_si512(result_first, MASK_SECOND_BYTE_TO_PACK),                                          // 48 00 65 00 6C 00 6C 00 6F 00 20 00 31 00 32 00 33 00 34 00 35 00 36 00 37 00 38 00 39 00 30 00 41 00 42 00 43 00 44 00 45 00 46 00 47 00 48 00 49 00 4A 00 4B 00 4C 00 4D 00 4E 00 4F 00 50 00
            _mm512_and_si512(result_second, MASK_SECOND_BYTE_TO_PACK)                                          // 51 00 52 00 53 00 54 00 55 00 56 00 57 00 58 00 59 00 5A 00 5B 00 5C 00 5D 00 5E 00 5F 00 60 00 61 00 62 00 63 00 64 00 65 00 66 00 67 00 68 00 69 00 6A 00 6B 00 6C 00 6D 00 6E 00 6F 00 70 00
        );                                                                                                     // 48 65 6C 6C 6F 20 31 32 51 52 53 54 55 56 57 58 33 34 35 36 37 38 39 30 59 5A 5B 5C 5D 5E 5F 60 41 42 43 44 45 46 47 48 61 62 63 64 65 66 67 68 49 4A 4B 4C 4D 4E 4F 50 69 6A 6B 6C 6D 6E 6F 70

        // Исправляем порядок lane в 512-битном векторе
        __m512i final_result = _mm512_permutexvar_epi64(PERMUTE_MASK_ORDER_CORRECTION, packed_result);         // 48 65 6C 6C 6F 20 31 32 33 34 35 36 37 38 39 30 41 42 43 44 45 46 47 48 49 4A 4B 4C 4D 4E 4F 50 51 52 53 54 55 56 57 58 59 5A 5B 5C 5D 5E 5F 60 61 62 63 64 65 66 67 68 69 6A 6B 6C 6D 6E 6F 70

        // Сохраняем 64 байта результата
        _mm512_storeu_si512((__m512i*)(bin + i / 2), final_result);
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

    const __m512i HEX_TABLE = _mm512_load_si512((__m512i*) CHARS);

    const __m512i MASK_LOW_NIBBLE = _mm512_set1_epi8(0x0F);

    const __m512i PERMUTE_MASK_FIRST = _mm512_setr_epi64(0x00, 0x01, 0x08, 0x09, 0x02, 0x03, 0x0A, 0x0B);
    const __m512i PERMUTE_MASK_SECOND = _mm512_setr_epi64(0x04, 0x05, 0x0C, 0x0D, 0x06, 0x07, 0x0E, 0x0F);

    size_t i = 0;

    for (; i + 64 <= length; i += 64) {
        // Загружаем 32 байта данных
        __m512i data = _mm512_loadu_si512((__m512i*)(input + i));

        // Разделяем байты на старшие и младшие полубайты
        __m512i high_nibbles = _mm512_and_si512(_mm512_srli_epi16(data, 4), MASK_LOW_NIBBLE);
        __m512i low_nibbles = _mm512_and_si512(data, MASK_LOW_NIBBLE);

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        __m512i hex_high = _mm512_shuffle_epi8(HEX_TABLE, high_nibbles);
        __m512i hex_low = _mm512_shuffle_epi8(HEX_TABLE, low_nibbles);

        // Чередуем старшие и младшие полубайты
        __m512i hex_packed_even = _mm512_unpacklo_epi8(hex_high, hex_low);
        __m512i hex_packed_odd = _mm512_unpackhi_epi8(hex_high, hex_low);

        // Исправляем порядок, меняя местами куски между lane
        __m512i final_part_first = _mm512_permutex2var_epi64(hex_packed_even, PERMUTE_MASK_FIRST, hex_packed_odd);
        __m512i final_part_second = _mm512_permutex2var_epi64(hex_packed_even, PERMUTE_MASK_SECOND, hex_packed_odd);

        // Сохраняем результат (128 байт)
        _mm512_storeu_si512((__m512i*)(hex + 2 * i), final_part_first);
        _mm512_storeu_si512((__m512i*)(hex + 2 * i + 64), final_part_second);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = (*CHARS).chars[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = (*CHARS).chars[input[i] & 0x0F];
    }
}

void test_bin2hex() {
    char hex_result[131] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[65] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30,
                         0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50,
                         0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60,
                         0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70,
                         0x48};

    bin2hex(input, hex_result, false, sizeof(input));

    printf("Input Binary (bin2hex): ");
    for (int i = 0; i < sizeof(input); i++) {
        printf("%02X ", input[i]);
    }
    printf("\nOutput HEX (bin2hex): %s\n", hex_result);
}

void test_hex2bin2hex() {
    char input[131] = {'4', '8', '6', '5', '6', 'C', '6', 'C', '6', 'F', '2', '0', '3', '1', '3', '2',
                       '3', '3', '3', '4', '3', '5', '3', '6', '3', '7', '3', '8', '3', '9', '3', '0',
                       '4', '1', '4', '2', '4', '3', '4', '4', '4', '5', '4', '6', '4', '7', '4', '8',
                       '4', '9', '4', 'A', '4', 'B', '4', 'C', '4', 'D', '4', 'E', '4', 'F', '5', '0',
                       '5', '1', '5', '2', '5', '3', '5', '4', '5', '5', '5', '6', '5', '7', '5', '8',
                       '5', '9', '5', 'A', '5', 'B', '5', 'C', '5', 'D', '5', 'E', '5', 'F', '6', '0',
                       '6', '1', '6', '2', '6', '3', '6', '4', '6', '5', '6', '6', '6', '7', '6', '8',
                       '6', '9', '6', 'A', '6', 'B', '6', 'C', '6', 'D', '6', 'E', '6', 'F', '7', '0',
                       '4', '8', '\0'};
    uint8_t binary[65] = {0};
    char hex_result[131] = {0};

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