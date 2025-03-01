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
#include <arm_neon.h>

void hex2bin(const uint8_t* hex, uint8_t* bin, size_t hex_len) {
    // Проверка на четность длины
    if (hex_len % 2 != 0) {
        printf("Error: Input length must be even\n");
        return;
    }

    // Константы для преобразования
    uint8x16_t offset_digit = vdupq_n_u8('0');
    uint8x16_t offset_upper = vdupq_n_u8('A' - 10);
    uint8x16_t offset_lower = vdupq_n_u8('a' - 10);

    size_t i = 0;

    // Обработка по 32 символа (16 байт результата) за раз
    for (; i + 32 <= hex_len; i += 32) {
        // Загружаем 32 символа с разделением на пары
        uint8x16x2_t chars = vld2q_u8(hex + i);

        // Инициализируем результат нулем
        uint8x16_t result = vdupq_n_u8(0);

        // Первая часть пары (будет сдвинута влево)
        uint8x16_t first = chars.val[0];
        uint8x16_t first_is_digit = vandq_u8(vcgeq_u8(first, vdupq_n_u8('0')), vcleq_u8(first, vdupq_n_u8('9')));
        uint8x16_t first_is_upper = vandq_u8(vcgeq_u8(first, vdupq_n_u8('A')), vcleq_u8(first, vdupq_n_u8('F')));
        uint8x16_t first_is_lower = vandq_u8(vcgeq_u8(first, vdupq_n_u8('a')), vcleq_u8(first, vdupq_n_u8('f')));

        first = vbslq_u8(first_is_digit, vsubq_u8(first, offset_digit), first);
        first = vbslq_u8(first_is_upper, vsubq_u8(first, offset_upper), first);
        first = vbslq_u8(first_is_lower, vsubq_u8(first, offset_lower), first);

        // Вторая часть пары (останется как есть)
        uint8x16_t second = chars.val[1];
        uint8x16_t second_is_digit = vandq_u8(vcgeq_u8(second, vdupq_n_u8('0')), vcleq_u8(second, vdupq_n_u8('9')));
        uint8x16_t second_is_upper = vandq_u8(vcgeq_u8(second, vdupq_n_u8('A')), vcleq_u8(second, vdupq_n_u8('F')));
        uint8x16_t second_is_lower = vandq_u8(vcgeq_u8(second, vdupq_n_u8('a')), vcleq_u8(second, vdupq_n_u8('f')));

        second = vbslq_u8(second_is_digit, vsubq_u8(second, offset_digit), second);
        second = vbslq_u8(second_is_upper, vsubq_u8(second, offset_upper), second);
        second = vbslq_u8(second_is_lower, vsubq_u8(second, offset_lower), second);

        // Объединяем: первая часть сдвигается на 4 бита влево, затем OR со второй
        result = vorrq_u8(vshlq_n_u8(first, 4), second);

        // Сохраняем 16 байт результата
        vst1q_u8(bin + i/2, result);
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

        bin[i/2] = (first << 4) | second;
    }
}

void bin2hex(const uint8_t *input, char *hex, size_t length) {
    // Таблица для преобразования полубайтов в шестнадцатеричные символы
    uint8_t hex_chars[16] = {'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'};
    uint8x16_t hex_table = vld1q_u8(hex_chars);

    size_t i = 0;

    for (; i + 16 <= length; i += 16) {
        // Загружаем 16 байт данных
        uint8x16_t data = vld1q_u8(input + i);

        // Разделяем байты на старшие и младшие полубайты
        uint8x16_t high_nibbles = vshrq_n_u8(data, 4); // Сдвиг вправо на 4 бита
        uint8x16_t low_nibbles = vandq_u8(data, vdupq_n_u8(0x0F)); // Обнуление старших 4 бит

        // Сопоставляем полубайты в шестнадцатеричные символы представления ASCII совместимой кодировки
        uint8x16_t hex_high = vqtbl1q_u8(hex_table, high_nibbles);
        uint8x16_t hex_low = vqtbl1q_u8(hex_table, low_nibbles);

        // Сохраняем результат без чередования
        uint8x16x2_t non_interleaved = {hex_high, hex_low};

        // Сохраняем результат с чередованием
        vst2q_u8((uint8_t*)(hex + i * 2), non_interleaved);
    }

    // Обрабатываем не кратную часть
    for (; i + 1 <= length; i += 1) {
        hex[2 * i] = "0123456789ABCDEF"[(input[i] >> 4) & 0x0F];
        hex[2 * i + 1] = "0123456789ABCDEF"[input[i] & 0x0F];
    }
}

void test_bin2hex() {
    char hex_result[35] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[17] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x48};

    bin2hex(input, hex_result, sizeof(input));

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