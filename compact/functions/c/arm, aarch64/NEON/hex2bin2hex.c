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
#include <arm_neon.h>

// Определяем макрос для выравнивания
#if defined(_MSC_VER) // MSVC
    #define ALIGNED(x) __declspec(align(x))
#else // GCC/Clang
    #define ALIGNED(x) __attribute__((aligned(x)))
#endif

// Определяем структуру с выравниванием на 16 байт
struct HexChars {
    uint8_t chars[16];
} ALIGNED(16); // Выравниваем структуру на 16 байт

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
    const uint8x16_t OFFSET_ASCII_DIGIT                 = vdupq_n_u8(0x30); // '0'
    const uint8x16_t OFFSET_ASCII_ALPHABET_UPPER        = vdupq_n_u8(0x37); // 'A' - 10
    const uint8x16_t OFFSET_ASCII_ALPHABET_LOWER        = vdupq_n_u8(0x57); // 'a' - 10

    const uint8x16_t ASCII_TABLE_DIGITS_START           = vdupq_n_u8(0x30); // '0'
    const uint8x16_t ASCII_TABLE_DIGITS_END             = vdupq_n_u8(0x39); // '9'
    const uint8x16_t ASCII_TABLE_ALPHABET_CAPITAL_START = vdupq_n_u8(0x41); // 'A'
    const uint8x16_t ASCII_TABLE_ALPHABET_CAPITAL_END   = vdupq_n_u8(0x46); // 'F'
    const uint8x16_t ASCII_TABLE_ALPHABET_SMALL_START   = vdupq_n_u8(0x61); // 'a'
    const uint8x16_t ASCII_TABLE_ALPHABET_SMALL_END     = vdupq_n_u8(0x66); // 'f'

    size_t i = 0;

    // Обработка по 32 символа (16 байт результата) за раз
    for (; i + 32 <= hex_len; i += 32) {
        // Загружаем 32 символа с разделением на пары
        uint8x16x2_t chars = vld2q_u8(hex + i);

        // Первая часть пары (будет сдвинута влево)
        uint8x16_t first_is_digit = vandq_u8(
            vcgeq_u8(chars.val[0], ASCII_TABLE_DIGITS_START),
            vcleq_u8(chars.val[0], ASCII_TABLE_DIGITS_END)
        );
        uint8x16_t first_is_upper = vandq_u8(
            vcgeq_u8(chars.val[0], ASCII_TABLE_ALPHABET_CAPITAL_START),
            vcleq_u8(chars.val[0], ASCII_TABLE_ALPHABET_CAPITAL_END)
        );
        uint8x16_t first_is_lower = vandq_u8(
            vcgeq_u8(chars.val[0], ASCII_TABLE_ALPHABET_SMALL_START),
            vcleq_u8(chars.val[0], ASCII_TABLE_ALPHABET_SMALL_END)
        );

        // Вторая часть пары (останется как есть)
        uint8x16_t second_is_digit = vandq_u8(
            vcgeq_u8(chars.val[1], ASCII_TABLE_DIGITS_START),
            vcleq_u8(chars.val[1], ASCII_TABLE_DIGITS_END)
        );
        uint8x16_t second_is_upper = vandq_u8(
            vcgeq_u8(chars.val[1], ASCII_TABLE_ALPHABET_CAPITAL_START),
            vcleq_u8(chars.val[1], ASCII_TABLE_ALPHABET_CAPITAL_END)
        );
        uint8x16_t second_is_lower = vandq_u8(
            vcgeq_u8(chars.val[1], ASCII_TABLE_ALPHABET_SMALL_START),
            vcleq_u8(chars.val[1], ASCII_TABLE_ALPHABET_SMALL_END)
        );

        uint8x16_t first = vbslq_u8(first_is_digit, vsubq_u8(chars.val[0], OFFSET_ASCII_DIGIT), chars.val[0]);
        uint8x16_t second = vbslq_u8(second_is_digit, vsubq_u8(chars.val[1], OFFSET_ASCII_DIGIT), chars.val[1]);

        first = vbslq_u8(first_is_upper, vsubq_u8(first, OFFSET_ASCII_ALPHABET_UPPER), first);
        first = vbslq_u8(first_is_lower, vsubq_u8(first, OFFSET_ASCII_ALPHABET_LOWER), first);

        second = vbslq_u8(second_is_upper, vsubq_u8(second, OFFSET_ASCII_ALPHABET_UPPER), second);
        second = vbslq_u8(second_is_lower, vsubq_u8(second, OFFSET_ASCII_ALPHABET_LOWER), second);

        // Сохраняем 16 байт результата (Объединяем: первая часть сдвигается на 4 бита влево, затем OR со второй)
        vst1q_u8(bin + i / 2, vorrq_u8(vshlq_n_u8(first, 4), second));
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
    // Определяем таблицу для преобразования полубайтов в шестнадцатеричные символы
    const struct HexChars* CHARS = _case ? &ASCII_HEX_CHARS_LOWER : &ASCII_HEX_CHARS_UPPER;

    const uint8x16_t HEX_TABLE = vld1q_u8((uint8_t*) CHARS);

    const uint8x16_t MASK_LOW_NIBBLE = vdupq_n_u8(0x0F);

    size_t i = 0;

    for (; i + 16 <= length; i += 16) {
        // Загружаем 16 байт данных
        uint8x16_t data = vld1q_u8(input + i);

        // Разделяем байты на старшие и младшие полубайты
        uint8x16_t high_nibbles = vshrq_n_u8(data, 4); // Сдвиг вправо на 4 бита
        uint8x16_t low_nibbles = vandq_u8(data, MASK_LOW_NIBBLE); // Обнуление старших 4 бит

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        uint8x16_t hex_high = vqtbl1q_u8(HEX_TABLE, high_nibbles);
        uint8x16_t hex_low = vqtbl1q_u8(HEX_TABLE, low_nibbles);

        // Сохраняем результат без чередования
        uint8x16x2_t non_interleaved = {hex_high, hex_low};

        // Сохраняем результат с чередованием
        vst2q_u8((uint8_t*)(hex + i * 2), non_interleaved);
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