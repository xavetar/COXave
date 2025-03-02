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

uint64_t bitwise_lt(uint64_t a, uint64_t b) {
    uint64_t lower_7bit_mask = (~0ULL / 0xFF) * 0x7F;
    uint64_t a_lower_7bits = a & lower_7bit_mask;
    uint64_t b_lower_7bits = b & lower_7bit_mask;
    uint64_t lower_7bit_diff = ((lower_7bit_mask - a_lower_7bits) + b_lower_7bits) & ~lower_7bit_mask;
    uint64_t high_bit_diff = (a ^ b) & ~lower_7bit_mask;
    uint64_t result = lower_7bit_diff & ~(a & high_bit_diff);
    result = result | (b & high_bit_diff);
    return (result << 1) - (result >> 7);
}

uint64_t bitwise_gt(uint64_t a, uint64_t b) {
    uint64_t lower_7bit_mask = (~0ULL / 0xFF) * 0x7F;
    uint64_t a_lower_7bits = a & lower_7bit_mask;
    uint64_t b_lower_7bits = b & lower_7bit_mask;
    uint64_t lower_7bit_diff = ((lower_7bit_mask - b_lower_7bits) + a_lower_7bits) & ~lower_7bit_mask;
    uint64_t high_bit_diff = (a ^ b) & ~lower_7bit_mask;
    uint64_t result = lower_7bit_diff & ~(b & high_bit_diff);
    result = result | (a & high_bit_diff);
    return (result << 1) - (result >> 7);
}

void hex2bin(const uint8_t* hex, uint8_t* bin, size_t hex_len) {
    if (hex_len % 2 != 0) {
        printf("Error: Input length must be even\n");
        return;
    }

    size_t i = 0;

    for (; i + 16 <= hex_len; i += 16) {
        uint64_t chars_high = 0;
        uint64_t chars_low = 0;

        for (int j = 0; j < 8; j++) {
            chars_high |= (uint64_t) hex[i + j * 2] << (j * 8);
            chars_low |= (uint64_t) hex[i + j * 2 + 1] << (j * 8);
        }

        // Определяем диапазоны
        uint64_t high_digit_mask = bitwise_gt(chars_high, 0x2F2F2F2F2F2F2F2FULL) & bitwise_lt(chars_high, 0x3A3A3A3A3A3A3A3AULL);
        uint64_t high_upper_mask = bitwise_gt(chars_high, 0x4040404040404040ULL) & bitwise_lt(chars_high, 0x4747474747474747ULL);
        uint64_t high_lower_mask = bitwise_gt(chars_high, 0x6060606060606060ULL) & bitwise_lt(chars_high, 0x6767676767676767ULL);

        uint64_t low_digit_mask = bitwise_gt(chars_low, 0x2F2F2F2F2F2F2F2FULL) & bitwise_lt(chars_low, 0x3A3A3A3A3A3A3A3AULL);
        uint64_t low_upper_mask = bitwise_gt(chars_low, 0x4040404040404040ULL) & bitwise_lt(chars_low, 0x4747474747474747ULL);
        uint64_t low_lower_mask = bitwise_gt(chars_low, 0x6060606060606060ULL) & bitwise_lt(chars_low, 0x6767676767676767ULL);

        // Изолируем значения, подгоняя меньшие к 0xFF перед вычитанием
        uint64_t high_digits_raw = chars_high | (~high_digit_mask & 0xFFFFFFFFFFFFFFFFULL);
        uint64_t high_uppers_raw = chars_high | (~high_upper_mask & 0xFFFFFFFFFFFFFFFFULL);
        uint64_t high_lowers_raw = chars_high | (~high_lower_mask & 0xFFFFFFFFFFFFFFFFULL);

        uint64_t low_digits_raw = chars_low | (~low_digit_mask & 0xFFFFFFFFFFFFFFFFULL);
        uint64_t low_uppers_raw = chars_low | (~low_upper_mask & 0xFFFFFFFFFFFFFFFFULL);
        uint64_t low_lowers_raw = chars_low | (~low_lower_mask & 0xFFFFFFFFFFFFFFFFULL);

        // Преобразуем значения
        uint64_t high_digits = (high_digits_raw - 0x3030303030303030ULL) & high_digit_mask;
        uint64_t high_uppers = (high_uppers_raw - 0x3737373737373737ULL) & high_upper_mask;
        uint64_t high_lowers = (high_lowers_raw - 0x5757575757575757ULL) & high_lower_mask;

        uint64_t low_digits = (low_digits_raw - 0x3030303030303030ULL) & low_digit_mask;
        uint64_t low_uppers = (low_uppers_raw - 0x3737373737373737ULL) & low_upper_mask;
        uint64_t low_lowers = (low_lowers_raw - 0x5757575757575757ULL) & low_lower_mask;

        // Объединяем значения
        uint64_t high_values = high_digits | high_uppers | high_lowers;
        uint64_t low_values = low_digits | low_uppers | low_lowers;

        // Сохраняем результат побайтово
        for (int j = 0; j < 8; j++) {
            uint8_t high = (high_values >> (j * 8)) & 0x0F;
            uint8_t low = (low_values >> (j * 8)) & 0x0F;
            bin[i / 2 + j] = (high << 4) | low;
        }
    }

    // Обработка остатка
    for (; i + 2 <= hex_len; i += 2) {
        uint8_t high = hex[i];
        uint8_t low = hex[i + 1];

        high = (high <= '9') ? (high - '0') :
               (high >= 'A' && high <= 'F') ? (high - 'A' + 10) :
               (high >= 'a' && high <= 'f') ? (high - 'a' + 10) : 0;

        low = (low <= '9') ? (low - '0') :
              (low >= 'A' && low <= 'F') ? (low - 'A' + 10) :
              (low >= 'a' && low <= 'f') ? (low - 'a' + 10) : 0;

        bin[i / 2] = (high << 4) | low;
    }
}

void bin2hex(const uint8_t* input, char* hex, size_t length) {
    size_t i = 0;

    // Обработка по 8 байт (16 символов результата) за раз
    for (; i + 8 <= length; i += 8) {
        // Загружаем 8 байт в 64-битный регистр побайтово
        uint64_t data = 0;

        for (int j = 0; j < 8; j++) {
            data |= (uint64_t) input[i + j] << (j * 8); // Младший байт справа
        }

        // Разделяем байты на старшие и младшие полубайты
        uint64_t high_nibbles = (data & 0xF0F0F0F0F0F0F0F0ULL) >> 4;
        uint64_t low_nibbles = data & 0x0F0F0F0F0F0F0F0FULL;

        // Определяем, какие nibbles больше 9 (для A-F)
        uint64_t high_is_alpha = bitwise_gt(high_nibbles, 0x0909090909090909ULL);
        uint64_t low_is_alpha = bitwise_gt(low_nibbles, 0x0909090909090909ULL);

        // Вычисляем значения для 0-9 и 10-15 отдельно
        uint64_t high_ascii_digit = high_nibbles + 0x3030303030303030ULL; // "00000000" + nibble
        uint64_t high_ascii_alpha = high_nibbles + 0x3737373737373737ULL; // "77777777" + nibble (A=10 → 0x41)
        uint64_t low_ascii_digit = low_nibbles + 0x3030303030303030ULL;
        uint64_t low_ascii_alpha = low_nibbles + 0x3737373737373737ULL;

        // Обнуляем октеты, не соответствующие маскам
        high_ascii_digit &= ~high_is_alpha;
        high_ascii_alpha &= high_is_alpha;
        low_ascii_digit &= ~low_is_alpha;
        low_ascii_alpha &= low_is_alpha;

        // Объединяем значения
        uint64_t hex_high = high_ascii_digit | high_ascii_alpha;
        uint64_t hex_low = low_ascii_digit | low_ascii_alpha;

        // Чередуем старшие и младшие полубайты вручную в правильном порядке
        for (int j = 0; j < 8; j++) {
            hex[i * 2 + j * 2] = (hex_high >> (j * 8)) & 0xFF;    // Старший nibble, прямой порядок
            hex[i * 2 + j * 2 + 1] = (hex_low >> (j * 8)) & 0xFF; // Младший nibble, прямой порядок
        }
    }

    // Обработка остатка
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = "0123456789ABCDEF"[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = "0123456789ABCDEF"[input[i] & 0x0F];
    }
}

void test_bin2hex() {
    char hex_result[69] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[34] = {0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x48,
                         0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x48, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0xFF};

    bin2hex(input, hex_result, sizeof(input));

    printf("Input Binary (bin2hex): ");
    for (int i = 0; i < sizeof(input); i++) {
        printf("%02X ", input[i]);
    }
    printf("\nOutput HEX (bin2hex): %s\n", hex_result);
}

void test_hex2bin2hex() {
    char input[69] = {'1', '1', '2', '2', '3', '3', '4', '4', '5', '5', '6', '6', '7', '7', '8', '8',
                      '9', '9', '3', '4', '3', '5', '3', '6', '3', '7', '3', '8', '3', '9', '3', '0',
                      '4', '8', 'A', 'A', 'B', 'B', 'C', 'C', 'D', 'D', 'E', 'E', 'F', 'F', '4', '8',
                      '3', '2', '3', '3', '3', '4', '3', '5', '3', '6', '3', '7', '3', '8', '3', '9',
                      '3', '0', 'F', 'F', '\0'};
    uint8_t binary[34] = {0};
    char hex_result[69] = {0};

    hex2bin((uint8_t*) input, binary, sizeof(input) - 1);
    bin2hex(binary, hex_result, sizeof(binary));

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