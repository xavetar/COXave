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

void bin2hex_neon(const uint8_t *input, char *hex, size_t length) {
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

int main() {
    char hex[33] = {0}; // Удвоенный размер + нулевой терминатор

    uint8_t input[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30};

    bin2hex_neon(input, hex, sizeof(input));

    printf("Hex: %s\n", hex);
    return 0;
}