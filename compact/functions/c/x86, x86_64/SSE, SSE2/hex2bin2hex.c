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

// Определяем структуру с выравниванием на 16 байт
struct HexChars {
    uint8_t chars[16];
} __attribute__((aligned(16))); // Выравниваем структуру на 16 байт

// Таблицы для преобразования полубайтов в шестнадцатеричные символы
const struct HexChars ASCII_HEX_CHARS_UPPER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'} };
const struct HexChars ASCII_HEX_CHARS_LOWER = { .chars = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'} };

void hex2bin(const uint8_t* hex, uint8_t* bin, size_t hex_len) {
    // Проверка на четность длины
    if (hex_len % 2 != 0) {
        printf("Error: Input length must be even\n");
        return;
    }

    // Общие константы (ASCII)
    const __m128i OFFSET_ASCII_DIGIT                  = _mm_set1_epi8(0x30); // '0'
    const __m128i OFFSET_ASCII_ALPHABET_UPPER         = _mm_set1_epi8(0x37); // 'A' - 10
    const __m128i OFFSET_ASCII_ALPHABET_LOWER         = _mm_set1_epi8(0x57); // 'a' - 10

    const __m128i ASCII_TABLE_DIGITS_AFTER            = _mm_set1_epi8(0x2F); // '0' - 1
    const __m128i ASCII_TABLE_DIGITS_BEFORE           = _mm_set1_epi8(0x3A); // '9' + 1
    const __m128i ASCII_TABLE_ALPHABET_CAPITAL_AFTER  = _mm_set1_epi8(0x40); // 'A' - 1
    const __m128i ASCII_TABLE_ALPHABET_CAPITAL_BEFORE = _mm_set1_epi8(0x47); // 'F' - 1
    const __m128i ASCII_TABLE_ALPHABET_SMALL_AFTER    = _mm_set1_epi8(0x60); // 'a' - 1
    const __m128i ASCII_TABLE_ALPHABET_SMALL_BEFORE   = _mm_set1_epi8(0x67); // 'f' - 1

#if __SSSE3__
    const __m128i SECOND_SHUFFLE = _mm_setr_epi8(1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15, -1);

    const __m128i MASK_SECOND_BYTE_TO_PACK = _mm_set1_epi16(0x00FF);
#else
    const __m128i MASK_FIRST_BYTE_TO_PACK = _mm_set1_epi16(0xFF00);
    const __m128i MASK_SECOND_BYTE_TO_PACK = _mm_set1_epi16(0x00FF);
#endif

    size_t i = 0;

    // Обработка по 32 символа (16 байт результата) за раз
    for (; i + 32 <= hex_len; i += 32) {
        // Загружаем 32 символа (16 пар)
        __m128i chars_first = _mm_loadu_si128((__m128i*)(hex + i));
        __m128i chars_second = _mm_loadu_si128((__m128i*)(hex + i + 16));

        // Преобразуем первые 16 символов (chars_first)
        __m128i digit_mask_first = _mm_and_si128(
            _mm_cmpgt_epi8(chars_first,ASCII_TABLE_DIGITS_AFTER),
            _mm_cmplt_epi8(chars_first, ASCII_TABLE_DIGITS_BEFORE)
        );
        __m128i upper_mask_first = _mm_and_si128(
            _mm_cmpgt_epi8(chars_first, ASCII_TABLE_ALPHABET_CAPITAL_AFTER),
            _mm_cmplt_epi8(chars_first, ASCII_TABLE_ALPHABET_CAPITAL_BEFORE)
        );
        __m128i lower_mask_first = _mm_and_si128(
            _mm_cmpgt_epi8(chars_first, ASCII_TABLE_ALPHABET_SMALL_AFTER),
            _mm_cmplt_epi8(chars_first, ASCII_TABLE_ALPHABET_SMALL_BEFORE)
        );

        // Преобразуем вторые 16 символов (chars_second)
        __m128i digit_mask_second = _mm_and_si128(
            _mm_cmpgt_epi8(chars_second,ASCII_TABLE_DIGITS_AFTER),
            _mm_cmplt_epi8(chars_second, ASCII_TABLE_DIGITS_BEFORE)
        );
        __m128i upper_mask_second = _mm_and_si128(
            _mm_cmpgt_epi8(chars_second, ASCII_TABLE_ALPHABET_CAPITAL_AFTER),
            _mm_cmplt_epi8(chars_second, ASCII_TABLE_ALPHABET_CAPITAL_BEFORE)
        );
        __m128i lower_mask_second = _mm_and_si128(
            _mm_cmpgt_epi8(chars_second, ASCII_TABLE_ALPHABET_SMALL_AFTER),
            _mm_cmplt_epi8(chars_second, ASCII_TABLE_ALPHABET_SMALL_BEFORE)
        );

        __m128i digits_first = _mm_and_si128(digit_mask_first, _mm_sub_epi8(chars_first, OFFSET_ASCII_DIGIT));
        __m128i uppers_first = _mm_and_si128(upper_mask_first, _mm_sub_epi8(chars_first, OFFSET_ASCII_ALPHABET_UPPER));
        __m128i lowers_first = _mm_and_si128(lower_mask_first, _mm_sub_epi8(chars_first, OFFSET_ASCII_ALPHABET_LOWER));

        __m128i digits_second = _mm_and_si128(digit_mask_second, _mm_sub_epi8(chars_second, OFFSET_ASCII_DIGIT));
        __m128i uppers_second = _mm_and_si128(upper_mask_second, _mm_sub_epi8(chars_second, OFFSET_ASCII_ALPHABET_UPPER));
        __m128i lowers_second = _mm_and_si128(lower_mask_second, _mm_sub_epi8(chars_second, OFFSET_ASCII_ALPHABET_LOWER));

        __m128i values_first = _mm_or_si128(digits_first, _mm_or_si128(uppers_first, lowers_first));          // 04 08 06 05 06 0C 06 0C 06 0F 02 00 03 01 03 02
        __m128i values_second = _mm_or_si128(digits_second, _mm_or_si128(uppers_second, lowers_second));      // 03 03 03 04 03 05 03 06 03 07 03 08 03 09 03 00

#if __SSSE3__
        // SSSE3: Используем _mm_shuffle_epi8 для извлечения вторых символов
        __m128i shifted_high_and_low_to_msb_first = _mm_slli_epi16(values_first, 4);                          // 40 80 60 50 60 C0 60 C0 60 F0 20 00 30 10 30 20
        __m128i shifted_high_and_low_to_msb_second = _mm_slli_epi16(values_second, 4);                        // 30 30 30 40 30 50 30 60 30 70 30 80 30 90 30 00

        __m128i low_hex_to_lsb_first = _mm_shuffle_epi8(values_first, SECOND_SHUFFLE);                        // 08 00 05 00 0C 00 0C 00 0F 00 00 00 01 00 02 00
        __m128i low_hex_to_lsb_second = _mm_shuffle_epi8(values_second, SECOND_SHUFFLE);                      // 03 00 04 00 05 00 06 00 07 00 08 00 09 00 00 00

        __m128i result_first = _mm_or_si128(shifted_high_and_low_to_msb_first, low_hex_to_lsb_first);         // 48 80 65 50 6C C0 6C C0 6F F0 20 00 31 10 32 20
        __m128i result_second = _mm_or_si128(shifted_high_and_low_to_msb_second, low_hex_to_lsb_second);      // 33 30 34 40 35 50 36 60 37 70 38 80 39 90 30 00

        __m128i final_result = _mm_packus_epi16(
            _mm_and_si128(result_first, MASK_SECOND_BYTE_TO_PACK),                                            // 48 00 65 00 6C 00 6C 00 6F 00 20 00 31 00 32 00
            _mm_and_si128(result_second, MASK_SECOND_BYTE_TO_PACK)                                            // 33 00 34 00 35 00 36 00 37 00 38 00 39 00 30 00
        );
#else
        // SSE2: Извлекаем первые и вторые символы через маски
        __m128i high_hex_nibbles_first = _mm_and_si128(values_first, MASK_SECOND_BYTE_TO_PACK);               // 04 00 06 00 06 00 06 00 06 00 02 00 03 00 03 00
        __m128i low_hex_nibbles_first = _mm_and_si128(values_first, MASK_FIRST_BYTE_TO_PACK);                 // 00 08 00 05 00 0C 00 0C 00 0F 00 00 00 01 00 02

        __m128i high_hex_nibbles_second = _mm_and_si128(values_second, MASK_SECOND_BYTE_TO_PACK);             // 03 00 03 00 03 00 03 00 03 00 03 00 03 00 03 00
        __m128i low_hex_nibbles_second = _mm_and_si128(values_second, MASK_FIRST_BYTE_TO_PACK);               // 00 03 00 04 00 05 00 06 00 07 00 08 00 09 00 00

        __m128i low_hex_to_lsb_first = _mm_srli_epi16(low_hex_nibbles_first, 8);                              // 08 00 05 00 0C 00 0C 00 0F 00 00 00 01 00 02 00
        __m128i low_hex_to_lsb_second = _mm_srli_epi16(low_hex_nibbles_second, 8);                            // 03 00 04 00 05 00 06 00 07 00 08 00 09 00 00 00

        __m128i pack_high_nibbles_to_lsb = _mm_packus_epi16(high_hex_nibbles_first, high_hex_nibbles_second); // 04 06 06 06 06 02 03 03 03 03 03 03 03 03 03 03
        __m128i pack_low_nibbles_to_lsb = _mm_packus_epi16(low_hex_to_lsb_first, low_hex_to_lsb_second);      // 08 05 0C 0C 0F 00 01 02 03 04 05 06 07 08 09 00

        __m128i final_result = _mm_or_si128(_mm_slli_epi16(pack_high_nibbles_to_lsb, 4), pack_low_nibbles_to_lsb);
#endif

        // Сохраняем 16 байт результата
        _mm_storeu_si128((__m128i*)(bin + i / 2), final_result);
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

void bin2hex(const uint8_t *input, char *hex, bool _case, size_t length) {
#if __SSSE3__
    // Определяем таблицу для преобразования полубайтов в шестнадцатеричные символы
    const struct HexChars* CHARS = _case ? &ASCII_HEX_CHARS_LOWER : &ASCII_HEX_CHARS_UPPER;

    const __m128i HEX_TABLE = _mm_load_si128((__m128i*) CHARS);
#else
    const __m128i OFFSET_ASCII_DIGIT = _mm_set1_epi8(0x30); // '0'
    const __m128i OFFSET_ASCII_ALPHABET_UPPER = _mm_set1_epi8(0x37); // 'A' - 10
    const __m128i OFFSET_ASCII_ALPHABET_LOWER = _mm_set1_epi8(0x57); // 'a' - 10
    
    const __m128i OFFSET_ASCII_ALPHABET = _case ? OFFSET_ASCII_ALPHABET_LOWER : OFFSET_ASCII_ALPHABET_UPPER;

    const __m128i THRESHOLD_LAST_ASCII_DIGIT = _mm_set1_epi8(9); // Граница для выбора 0-9 или A-F, где нарушается линейная последовательность
#endif

    const __m128i MASK_LOW_NIBBLE = _mm_set1_epi8(0x0F);

    size_t i = 0;

    for (; i + 16 <= length; i += 16) {
        // Загружаем 16 байт данных
        __m128i data = _mm_loadu_si128((__m128i*) (input + i));

        // Разделяем байты на старшие и младшие полубайты
        __m128i high_nibbles = _mm_and_si128(_mm_srli_epi16(data, 4), MASK_LOW_NIBBLE); // Сдвиг вправо на 4 бита и обнуление в результирующей части старших 4-х бит
        __m128i low_nibbles = _mm_and_si128(data, MASK_LOW_NIBBLE);                     // Обнуление старших 4 бит

#if __SSSE3__
        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        __m128i hex_ascii_high = _mm_shuffle_epi8(HEX_TABLE, high_nibbles);
        __m128i hex_ascii_low = _mm_shuffle_epi8(HEX_TABLE, low_nibbles);
#else
        // Определяем, какие nibbles больше 9 (для A-F)
        __m128i high_is_alpha_mask = _mm_cmpgt_epi8(high_nibbles, THRESHOLD_LAST_ASCII_DIGIT);
        __m128i low_is_alpha_mask = _mm_cmpgt_epi8(low_nibbles, THRESHOLD_LAST_ASCII_DIGIT);

        // Вычисляем значения для 0-9 и 10-15 отдельно
        __m128i high_ascii_digit = _mm_add_epi8(high_nibbles, OFFSET_ASCII_DIGIT);    // Для 0-9: nibble + '0'
        __m128i high_ascii_alpha = _mm_add_epi8(high_nibbles, OFFSET_ASCII_ALPHABET); // Для 10-15: nibble + ('A' - 10) | ('a' - 10)
        __m128i low_ascii_digit = _mm_add_epi8(low_nibbles, OFFSET_ASCII_DIGIT);
        __m128i low_ascii_alpha = _mm_add_epi8(low_nibbles, OFFSET_ASCII_ALPHABET);

        // Выбираем правильные значения через маскирование (аналог blend для SSE2)
        __m128i hex_ascii_high = _mm_or_si128(
            _mm_and_si128(high_is_alpha_mask, high_ascii_alpha),   // Если > 9, берём alpha
            _mm_andnot_si128(high_is_alpha_mask, high_ascii_digit) // Если ≤ 9, берём ascii
        );
        __m128i hex_ascii_low = _mm_or_si128(
            _mm_and_si128(low_is_alpha_mask, low_ascii_alpha),
            _mm_andnot_si128(low_is_alpha_mask, low_ascii_digit)
        );
#endif
        // Чередуем старшие и младшие полубайты
        __m128i interleaved_even = _mm_unpacklo_epi8(hex_ascii_high, hex_ascii_low);
        __m128i interleaved_odd = _mm_unpackhi_epi8(hex_ascii_high, hex_ascii_low);

        // Сохраняем результат (32 байта)
        _mm_storeu_si128((__m128i*)(hex + i * 2), interleaved_even);
        _mm_storeu_si128((__m128i*)(hex + i * 2 + 16), interleaved_odd);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = (*CHARS).chars[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = (*CHARS).chars[input[i] & 0x0F];
    }
}

void test_bin2hex() {
    char hex_result[35] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[17] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x48};

    bin2hex(input, hex_result, false, sizeof(input));

    printf("Input Binary (bin2hex): ");
    for (int i = 0; i < sizeof(input); i++) {
        printf("%02X ", input[i]);
    }
    printf("\nOutput HEX (bin2hex): %s\n", hex_result);
}

void test_hex2bin2hex() {
    char input[35] = {'4', '8', '6', '5', '6', 'C', '6', 'C', '6', 'F', '2', '0',
                      '3', '1', '3', '2', '3', '3', '3', '4', '3', '5', '3', '6',
                      '3', '7', '3', '8', '3', '9', '3', '0', '4', '8', '\0'};
    uint8_t binary[17] = {0};
    char hex_result[35] = {0};

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