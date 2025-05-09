/* Copyright 2012 Mozilla Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
/* Copyright 1996-2003 Glyph & Cog, LLC
 *
 * The CCITT stream implementation contained in this file is a JavaScript port
 * of XPDF's implementation, made available under the Apache 2.0 open source
 * license.
 */

// See <https://github.com/mozilla/pdf.js/blob/master/src/core/ccitt.js>


const ccittEOL: i32 = -2;
const ccittEOF: i32 = -1;
const twoDimPass: i32 = 0;
const twoDimHoriz: i32 = 1;
const twoDimVert0: i32 = 2;
const twoDimVertR1: i32 = 3;
const twoDimVertL1: i32 = 4;
const twoDimVertR2: i32 = 5;
const twoDimVertL2: i32 = 6;
const twoDimVertR3: i32 = 7;
const twoDimVertL3: i32 = 8;

// prettier-ignore
static twoDimTable: [[i32; 2]; 128] = [
    [-1, -1], [-1, -1],                   // 000000x
    [7, twoDimVertL3],                    // 0000010
    [7, twoDimVertR3],                    // 0000011
    [6, twoDimVertL2], [6, twoDimVertL2], // 000010x
    [6, twoDimVertR2], [6, twoDimVertR2], // 000011x
    [4, twoDimPass], [4, twoDimPass],     // 0001xxx
    [4, twoDimPass], [4, twoDimPass],
    [4, twoDimPass], [4, twoDimPass],
    [4, twoDimPass], [4, twoDimPass],
    [3, twoDimHoriz], [3, twoDimHoriz],   // 001xxxx
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimHoriz], [3, twoDimHoriz],
    [3, twoDimVertL1], [3, twoDimVertL1], // 010xxxx
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertL1], [3, twoDimVertL1],
    [3, twoDimVertR1], [3, twoDimVertR1], // 011xxxx
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [3, twoDimVertR1], [3, twoDimVertR1],
    [1, twoDimVert0], [1, twoDimVert0],   // 1xxxxxx
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0],
    [1, twoDimVert0], [1, twoDimVert0]
];

// prettier-ignore
const whiteTable1: [[i32; 2]; 32] = [
  [-1, -1],                               // 00000
  [12, ccittEOL],                         // 00001
  [-1, -1], [-1, -1],                     // 0001x
  [-1, -1], [-1, -1], [-1, -1], [-1, -1], // 001xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1], // 010xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1], // 011xx
  [11, 1792], [11, 1792],                 // 1000x
  [12, 1984],                             // 10010
  [12, 2048],                             // 10011
  [12, 2112],                             // 10100
  [12, 2176],                             // 10101
  [12, 2240],                             // 10110
  [12, 2304],                             // 10111
  [11, 1856], [11, 1856],                 // 1100x
  [11, 1920], [11, 1920],                 // 1101x
  [12, 2368],                             // 11100
  [12, 2432],                             // 11101
  [12, 2496],                             // 11110
  [12, 2560]                              // 11111
];

// prettier-ignore
const whiteTable2: [[i32; 2]; 512] = [
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],     // 0000000xx
  [8, 29], [8, 29],                           // 00000010x
  [8, 30], [8, 30],                           // 00000011x
  [8, 45], [8, 45],                           // 00000100x
  [8, 46], [8, 46],                           // 00000101x
  [7, 22], [7, 22], [7, 22], [7, 22],         // 0000011xx
  [7, 23], [7, 23], [7, 23], [7, 23],         // 0000100xx
  [8, 47], [8, 47],                           // 00001010x
  [8, 48], [8, 48],                           // 00001011x
  [6, 13], [6, 13], [6, 13], [6, 13],         // 000011xxx
  [6, 13], [6, 13], [6, 13], [6, 13],
  [7, 20], [7, 20], [7, 20], [7, 20],         // 0001000xx
  [8, 33], [8, 33],                           // 00010010x
  [8, 34], [8, 34],                           // 00010011x
  [8, 35], [8, 35],                           // 00010100x
  [8, 36], [8, 36],                           // 00010101x
  [8, 37], [8, 37],                           // 00010110x
  [8, 38], [8, 38],                           // 00010111x
  [7, 19], [7, 19], [7, 19], [7, 19],         // 0001100xx
  [8, 31], [8, 31],                           // 00011010x
  [8, 32], [8, 32],                           // 00011011x
  [6, 1], [6, 1], [6, 1], [6, 1],             // 000111xxx
  [6, 1], [6, 1], [6, 1], [6, 1],
  [6, 12], [6, 12], [6, 12], [6, 12],         // 001000xxx
  [6, 12], [6, 12], [6, 12], [6, 12],
  [8, 53], [8, 53],                           // 00100100x
  [8, 54], [8, 54],                           // 00100101x
  [7, 26], [7, 26], [7, 26], [7, 26],         // 0010011xx
  [8, 39], [8, 39],                           // 00101000x
  [8, 40], [8, 40],                           // 00101001x
  [8, 41], [8, 41],                           // 00101010x
  [8, 42], [8, 42],                           // 00101011x
  [8, 43], [8, 43],                           // 00101100x
  [8, 44], [8, 44],                           // 00101101x
  [7, 21], [7, 21], [7, 21], [7, 21],         // 0010111xx
  [7, 28], [7, 28], [7, 28], [7, 28],         // 0011000xx
  [8, 61], [8, 61],                           // 00110010x
  [8, 62], [8, 62],                           // 00110011x
  [8, 63], [8, 63],                           // 00110100x
  [8, 0], [8, 0],                             // 00110101x
  [8, 320], [8, 320],                         // 00110110x
  [8, 384], [8, 384],                         // 00110111x
  [5, 10], [5, 10], [5, 10], [5, 10],         // 00111xxxx
  [5, 10], [5, 10], [5, 10], [5, 10],
  [5, 10], [5, 10], [5, 10], [5, 10],
  [5, 10], [5, 10], [5, 10], [5, 10],
  [5, 11], [5, 11], [5, 11], [5, 11],         // 01000xxxx
  [5, 11], [5, 11], [5, 11], [5, 11],
  [5, 11], [5, 11], [5, 11], [5, 11],
  [5, 11], [5, 11], [5, 11], [5, 11],
  [7, 27], [7, 27], [7, 27], [7, 27],         // 0100100xx
  [8, 59], [8, 59],                           // 01001010x
  [8, 60], [8, 60],                           // 01001011x
  [9, 1472],                                  // 010011000
  [9, 1536],                                  // 010011001
  [9, 1600],                                  // 010011010
  [9, 1728],                                  // 010011011
  [7, 18], [7, 18], [7, 18], [7, 18],         // 0100111xx
  [7, 24], [7, 24], [7, 24], [7, 24],         // 0101000xx
  [8, 49], [8, 49],                           // 01010010x
  [8, 50], [8, 50],                           // 01010011x
  [8, 51], [8, 51],                           // 01010100x
  [8, 52], [8, 52],                           // 01010101x
  [7, 25], [7, 25], [7, 25], [7, 25],         // 0101011xx
  [8, 55], [8, 55],                           // 01011000x
  [8, 56], [8, 56],                           // 01011001x
  [8, 57], [8, 57],                           // 01011010x
  [8, 58], [8, 58],                           // 01011011x
  [6, 192], [6, 192], [6, 192], [6, 192],     // 010111xxx
  [6, 192], [6, 192], [6, 192], [6, 192],
  [6, 1664], [6, 1664], [6, 1664], [6, 1664], // 011000xxx
  [6, 1664], [6, 1664], [6, 1664], [6, 1664],
  [8, 448], [8, 448],                         // 01100100x
  [8, 512], [8, 512],                         // 01100101x
  [9, 704],                                   // 011001100
  [9, 768],                                   // 011001101
  [8, 640], [8, 640],                         // 01100111x
  [8, 576], [8, 576],                         // 01101000x
  [9, 832],                                   // 011010010
  [9, 896],                                   // 011010011
  [9, 960],                                   // 011010100
  [9, 1024],                                  // 011010101
  [9, 1088],                                  // 011010110
  [9, 1152],                                  // 011010111
  [9, 1216],                                  // 011011000
  [9, 1280],                                  // 011011001
  [9, 1344],                                  // 011011010
  [9, 1408],                                  // 011011011
  [7, 256], [7, 256], [7, 256], [7, 256],     // 0110111xx
  [4, 2], [4, 2], [4, 2], [4, 2],             // 0111xxxxx
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 2], [4, 2], [4, 2], [4, 2],
  [4, 3], [4, 3], [4, 3], [4, 3],             // 1000xxxxx
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [4, 3], [4, 3], [4, 3], [4, 3],
  [5, 128], [5, 128], [5, 128], [5, 128],     // 10010xxxx
  [5, 128], [5, 128], [5, 128], [5, 128],
  [5, 128], [5, 128], [5, 128], [5, 128],
  [5, 128], [5, 128], [5, 128], [5, 128],
  [5, 8], [5, 8], [5, 8], [5, 8],             // 10011xxxx
  [5, 8], [5, 8], [5, 8], [5, 8],
  [5, 8], [5, 8], [5, 8], [5, 8],
  [5, 8], [5, 8], [5, 8], [5, 8],
  [5, 9], [5, 9], [5, 9], [5, 9],             // 10100xxxx
  [5, 9], [5, 9], [5, 9], [5, 9],
  [5, 9], [5, 9], [5, 9], [5, 9],
  [5, 9], [5, 9], [5, 9], [5, 9],
  [6, 16], [6, 16], [6, 16], [6, 16],         // 101010xxx
  [6, 16], [6, 16], [6, 16], [6, 16],
  [6, 17], [6, 17], [6, 17], [6, 17],         // 101011xxx
  [6, 17], [6, 17], [6, 17], [6, 17],
  [4, 4], [4, 4], [4, 4], [4, 4],             // 1011xxxxx
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 4], [4, 4], [4, 4], [4, 4],
  [4, 5], [4, 5], [4, 5], [4, 5],             // 1100xxxxx
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [4, 5], [4, 5], [4, 5], [4, 5],
  [6, 14], [6, 14], [6, 14], [6, 14],         // 110100xxx
  [6, 14], [6, 14], [6, 14], [6, 14],
  [6, 15], [6, 15], [6, 15], [6, 15],         // 110101xxx
  [6, 15], [6, 15], [6, 15], [6, 15],
  [5, 64], [5, 64], [5, 64], [5, 64],         // 11011xxxx
  [5, 64], [5, 64], [5, 64], [5, 64],
  [5, 64], [5, 64], [5, 64], [5, 64],
  [5, 64], [5, 64], [5, 64], [5, 64],
  [4, 6], [4, 6], [4, 6], [4, 6],             // 1110xxxxx
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 6], [4, 6], [4, 6], [4, 6],
  [4, 7], [4, 7], [4, 7], [4, 7],             // 1111xxxxx
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7],
  [4, 7], [4, 7], [4, 7], [4, 7]
];

// prettier-ignore
static blackTable1: [[i32; 2]; 128]= [
  [-1, -1], [-1, -1],                             // 000000000000x
  [12, ccittEOL], [12, ccittEOL],                 // 000000000001x
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000001xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000010xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000011xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000100xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000101xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000110xx
  [-1, -1], [-1, -1], [-1, -1], [-1, -1],         // 00000000111xx
  [11, 1792], [11, 1792], [11, 1792], [11, 1792], // 00000001000xx
  [12, 1984], [12, 1984],                         // 000000010010x
  [12, 2048], [12, 2048],                         // 000000010011x
  [12, 2112], [12, 2112],                         // 000000010100x
  [12, 2176], [12, 2176],                         // 000000010101x
  [12, 2240], [12, 2240],                         // 000000010110x
  [12, 2304], [12, 2304],                         // 000000010111x
  [11, 1856], [11, 1856], [11, 1856], [11, 1856], // 00000001100xx
  [11, 1920], [11, 1920], [11, 1920], [11, 1920], // 00000001101xx
  [12, 2368], [12, 2368],                         // 000000011100x
  [12, 2432], [12, 2432],                         // 000000011101x
  [12, 2496], [12, 2496],                         // 000000011110x
  [12, 2560], [12, 2560],                         // 000000011111x
  [10, 18], [10, 18], [10, 18], [10, 18],         // 0000001000xxx
  [10, 18], [10, 18], [10, 18], [10, 18],
  [12, 52], [12, 52],                             // 000000100100x
  [13, 640],                                      // 0000001001010
  [13, 704],                                      // 0000001001011
  [13, 768],                                      // 0000001001100
  [13, 832],                                      // 0000001001101
  [12, 55], [12, 55],                             // 000000100111x
  [12, 56], [12, 56],                             // 000000101000x
  [13, 1280],                                     // 0000001010010
  [13, 1344],                                     // 0000001010011
  [13, 1408],                                     // 0000001010100
  [13, 1472],                                     // 0000001010101
  [12, 59], [12, 59],                             // 000000101011x
  [12, 60], [12, 60],                             // 000000101100x
  [13, 1536],                                     // 0000001011010
  [13, 1600],                                     // 0000001011011
  [11, 24], [11, 24], [11, 24], [11, 24],         // 00000010111xx
  [11, 25], [11, 25], [11, 25], [11, 25],         // 00000011000xx
  [13, 1664],                                     // 0000001100100
  [13, 1728],                                     // 0000001100101
  [12, 320], [12, 320],                           // 000000110011x
  [12, 384], [12, 384],                           // 000000110100x
  [12, 448], [12, 448],                           // 000000110101x
  [13, 512],                                      // 0000001101100
  [13, 576],                                      // 0000001101101
  [12, 53], [12, 53],                             // 000000110111x
  [12, 54], [12, 54],                             // 000000111000x
  [13, 896],                                      // 0000001110010
  [13, 960],                                      // 0000001110011
  [13, 1024],                                     // 0000001110100
  [13, 1088],                                     // 0000001110101
  [13, 1152],                                     // 0000001110110
  [13, 1216],                                     // 0000001110111
  [10, 64], [10, 64], [10, 64], [10, 64],         // 0000001111xxx
  [10, 64], [10, 64], [10, 64], [10, 64]
];

// prettier-ignore
static blackTable2: [[i32; 2]; 192] = [
  [8, 13], [8, 13], [8, 13], [8, 13],     // 00000100xxxx
  [8, 13], [8, 13], [8, 13], [8, 13],
  [8, 13], [8, 13], [8, 13], [8, 13],
  [8, 13], [8, 13], [8, 13], [8, 13],
  [11, 23], [11, 23],                     // 00000101000x
  [12, 50],                               // 000001010010
  [12, 51],                               // 000001010011
  [12, 44],                               // 000001010100
  [12, 45],                               // 000001010101
  [12, 46],                               // 000001010110
  [12, 47],                               // 000001010111
  [12, 57],                               // 000001011000
  [12, 58],                               // 000001011001
  [12, 61],                               // 000001011010
  [12, 256],                              // 000001011011
  [10, 16], [10, 16], [10, 16], [10, 16], // 0000010111xx
  [10, 17], [10, 17], [10, 17], [10, 17], // 0000011000xx
  [12, 48],                               // 000001100100
  [12, 49],                               // 000001100101
  [12, 62],                               // 000001100110
  [12, 63],                               // 000001100111
  [12, 30],                               // 000001101000
  [12, 31],                               // 000001101001
  [12, 32],                               // 000001101010
  [12, 33],                               // 000001101011
  [12, 40],                               // 000001101100
  [12, 41],                               // 000001101101
  [11, 22], [11, 22],                     // 00000110111x
  [8, 14], [8, 14], [8, 14], [8, 14],     // 00000111xxxx
  [8, 14], [8, 14], [8, 14], [8, 14],
  [8, 14], [8, 14], [8, 14], [8, 14],
  [8, 14], [8, 14], [8, 14], [8, 14],
  [7, 10], [7, 10], [7, 10], [7, 10],     // 0000100xxxxx
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 10], [7, 10], [7, 10], [7, 10],
  [7, 11], [7, 11], [7, 11], [7, 11],     // 0000101xxxxx
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [7, 11], [7, 11], [7, 11], [7, 11],
  [9, 15], [9, 15], [9, 15], [9, 15],     // 000011000xxx
  [9, 15], [9, 15], [9, 15], [9, 15],
  [12, 128],                              // 000011001000
  [12, 192],                              // 000011001001
  [12, 26],                               // 000011001010
  [12, 27],                               // 000011001011
  [12, 28],                               // 000011001100
  [12, 29],                               // 000011001101
  [11, 19], [11, 19],                     // 00001100111x
  [11, 20], [11, 20],                     // 00001101000x
  [12, 34],                               // 000011010010
  [12, 35],                               // 000011010011
  [12, 36],                               // 000011010100
  [12, 37],                               // 000011010101
  [12, 38],                               // 000011010110
  [12, 39],                               // 000011010111
  [11, 21], [11, 21],                     // 00001101100x
  [12, 42],                               // 000011011010
  [12, 43],                               // 000011011011
  [10, 0], [10, 0], [10, 0], [10, 0],     // 0000110111xx
  [7, 12], [7, 12], [7, 12], [7, 12],     // 0000111xxxxx
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12],
  [7, 12], [7, 12], [7, 12], [7, 12]
];

// prettier-ignore
static blackTable3: [[i32; 2]; 64] = [
  [-1, -1], [-1, -1], [-1, -1], [-1, -1], // 0000xx
  [6, 9],                                 // 000100
  [6, 8],                                 // 000101
  [5, 7], [5, 7],                         // 00011x
  [4, 6], [4, 6], [4, 6], [4, 6],         // 0010xx
  [4, 5], [4, 5], [4, 5], [4, 5],         // 0011xx
  [3, 1], [3, 1], [3, 1], [3, 1],         // 010xxx
  [3, 1], [3, 1], [3, 1], [3, 1],
  [3, 4], [3, 4], [3, 4], [3, 4],         // 011xxx
  [3, 4], [3, 4], [3, 4], [3, 4],
  [2, 3], [2, 3], [2, 3], [2, 3],         // 10xxxx
  [2, 3], [2, 3], [2, 3], [2, 3],
  [2, 3], [2, 3], [2, 3], [2, 3],
  [2, 3], [2, 3], [2, 3], [2, 3],
  [2, 2], [2, 2], [2, 2], [2, 2],         // 11xxxx
  [2, 2], [2, 2], [2, 2], [2, 2],
  [2, 2], [2, 2], [2, 2], [2, 2],
  [2, 2], [2, 2], [2, 2], [2, 2]
];

pub trait CcittFaxSource {
    /// Returns the next byte, or None if EOF.
    fn next(&mut self) -> Option<u8>;
}

pub struct CCITTFaxDecoder<S: CcittFaxSource> {
    pub source: S,
    pub eof: bool,
    pub encoding: i32,
    pub eoline: bool,
    pub byte_align: bool,
    pub columns: usize,
    pub rows: usize,
    pub eoblock: bool,
    pub black: bool,
    pub coding_line: Vec<u32>,
    pub ref_line: Vec<u32>,
    pub coding_pos: usize,
    pub row: usize,
    pub next_line_2d: bool,
    pub input_bits: usize,
    pub input_buf: u32,
    pub output_bits: usize,
    pub rows_done: bool,
    pub err: bool,
}

impl<S: CcittFaxSource> CCITTFaxDecoder<S> {
    pub fn new(source: S, options: CCITTFaxDecoderOptions) -> Self {
        let k = options.k.unwrap_or(0);
        let eoline = options.end_of_line.unwrap_or(false);
        let byte_align = options.encoded_byte_align.unwrap_or(false);
        let columns = options.columns.unwrap_or(1728);
        let rows = options.rows.unwrap_or(0);
        let eoblock = options.eoblock.unwrap_or(true);
        let black = options.black_is_1.unwrap_or(false);

        let mut ref_line = vec![0; columns + 2];
        let mut coding_line = vec![0; columns + 1];
        coding_line[0] = columns as u32;

        let mut decoder = Self {
            source,
            eof: false,
            encoding: k,
            eoline,
            byte_align,
            columns,
            rows,
            eoblock,
            black,
            coding_line,
            ref_line,
            coding_pos: 0,
            row: 0,
            next_line_2d: k < 0,
            input_bits: 0,
            input_buf: 0,
            output_bits: 0,
            rows_done: false,
            err: false,
        };

        // Initial bit reading logic
        let mut code1;
        while {
            code1 = decoder.look_bits(12);
            code1 == 0
        } {
            decoder.eat_bits(1);
        }
        if code1 == 1 {
            decoder.eat_bits(12);
        }
        if decoder.encoding > 0 {
            decoder.next_line_2d = decoder.look_bits(1) == 0;
            decoder.eat_bits(1);
        }

        decoder
    }

    fn look_bits(&mut self, n: usize) -> i32 {
        let mut c;
        while self.input_bits < n {
            if let Some(byte) = self.source.next() {
                c = byte;
            } else {
                if self.input_bits == 0 {
                    return ccittEOF;
                }
                return ((self.input_buf << (n - self.input_bits)) & (0xffff >> (16 - n))) as i32;
            }
            self.input_buf = (self.input_buf << 8) | c as u32;
            self.input_bits += 8;
        }
        ((self.input_buf >> (self.input_bits - n)) & (0xffff >> (16 - n))) as i32
    }

    fn eat_bits(&mut self, n: usize) {
        if self.input_bits < n {
            self.input_bits = 0;
        } else {
            self.input_bits -= n;
        }
    }

    fn add_pixels(&mut self, a1: u32, black_pixels: bool) {
        let coding_line = &mut self.coding_line;
        let mut coding_pos = self.coding_pos;

        if a1 > coding_line[coding_pos] {
            if a1 > self.columns as u32 {
                println!("row is wrong length");
                self.err = true;
                // a1 = self.columns;
            }
            if ((coding_pos & 1) != 0) ^ black_pixels {
                coding_pos += 1;
            }

            coding_line[coding_pos] = a1;
        }
        self.coding_pos = coding_pos;
    }

    fn add_pixels_neg(&mut self, a1: u32, black_pixels: bool) {
        let coding_line = &mut self.coding_line;
        let mut coding_pos = self.coding_pos;

        if a1 > coding_line[coding_pos] {
            if a1 > self.columns as u32 {
                println!("row is wrong length");
                self.err = true;
                // a1 = self.columns;
            }
            if ((coding_pos & 1) != 0) ^ black_pixels {
                coding_pos += 1;
            }

            coding_line[coding_pos] = a1;
        } else if a1 < coding_line[coding_pos] {
            if a1 < 0 {
                println!("invalid code");
                self.err = true;
                // a1 = 0;
            }
            while coding_pos > 0 && a1 < coding_line[coding_pos - 1] {
                coding_pos -= 1;
            }
            coding_line[coding_pos] = a1;
        }

        self.coding_pos = coding_pos;
    }

    fn find_table_code(&mut self, start: usize, end: usize, table: &[[i32; 2]], limit: Option<usize>) -> (bool, i32, bool) {
        let limit_value = limit.unwrap_or(0);
        for i in start..=end {
            let code = self.look_bits(i);
            if code == ccittEOF {
                return (true, 1, false);
            }
            let mut code_shifted = code;
            if i < end {
                code_shifted <<= end - i;
            }
            if limit_value == 0 || code_shifted as usize >= limit_value {
                let p = table[(code_shifted as usize) - limit_value];
                if p[0] == i as i32 {
                    self.eat_bits(i);
                    return (true, p[1], true);
                }
            }
        }
        (false, 0, false)
    }

    fn get_two_dim_code(&mut self) -> i32 {
        let code = self.look_bits(7);
        let entry = twoDimTable[code as usize];
        self.eat_bits(entry[0] as usize);
        entry[1]
    }

    fn get_white_code(&mut self) -> i32 {
        let code = self.look_bits(5);
        let entry = whiteTable1[code as usize];
        if entry[0] > 0 {
            self.eat_bits(entry[0] as usize);
            return entry[1];
        }
        let code = self.look_bits(9);
        let entry = whiteTable2[code as usize];
        self.eat_bits(entry[0] as usize);
        entry[1]
    }

    fn get_black_code(&mut self) -> i32 {
        let code = self.look_bits(5);
        let entry = blackTable1[code as usize];
        if entry[0] > 0 {
            self.eat_bits(entry[0] as usize);
            return entry[1];
        }
        let code = self.look_bits(9);
        let entry = blackTable2[code as usize];
        self.eat_bits(entry[0] as usize);
        entry[1]
    }

    pub fn read_next_char(&mut self) -> i32 {
        if self.eof {
            return -1;
        }
        let ref_line = &mut self.ref_line;
        let coding_line = &mut self.coding_line;
        let columns = self.columns;

        let mut ref_pos;
        let mut black_pixels;
        let mut bits;
        let mut i = 0;

        if self.output_bits == 0 {
            if self.rows_done {
                self.eof = true;
            }
            if self.eof {
                return -1;
            }
            self.err = false;

            let mut code1;
            let mut code2;
            let mut code3;
            if self.next_line_2d {
                loop {
                    if coding_line[i] >= columns as u32 {
                        break;
                    }
                    
                    ref_line[i] = coding_line[i];
                    i += 1;
                }
                
                ref_line[i] = columns as u32;
                ref_line[i + 1] = columns as u32;
                coding_line[0] = 0;
                self.coding_pos = 0;
                ref_pos = 0;
                black_pixels = false;

                while coding_line[self.coding_pos] < columns as u32 {
                    code1 = self.get_two_dim_code();
                    match code1 {
                        x if x == twoDimPass => {
                            self.add_pixels(ref_line[ref_pos + 1], black_pixels);
                            if ref_line[ref_pos + 1] < columns as u32 {
                                ref_pos += 2;
                            }
                        }
                        x if x == twoDimHoriz => {
                            code1 = 0;
                            code2 = 0;
                            if black_pixels {
                                loop {
                                    code3 = self.get_black_code();
                                    code1 += code3;
                                    if code3 < 64 {
                                        break;
                                    }
                                }
                                loop {
                                    code3 = self.get_white_code();
                                    code2 += code3;
                                    if code3 < 64 {
                                        break;
                                    }
                                }
                            } else {
                                loop {
                                    code3 = self.get_white_code();
                                    code1 += code3;
                                    if code3 < 64 {
                                        break;
                                    }
                                }
                                loop {
                                    code3 = self.get_black_code();
                                    code2 += code3;
                                    if code3 < 64 {
                                        break;
                                    }
                                }
                            }
                            self.add_pixels(coding_line[self.coding_pos] + code1 as u32, black_pixels);
                            if coding_line[self.coding_pos] < columns as u32 {
                                self.add_pixels(coding_line[self.coding_pos] + code2 as u32, black_pixels ^ true);
                            }
                            while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                ref_pos += 2;
                            }
                        }
                        x if x == twoDimVertR3 => {
                            self.add_pixels(ref_line[ref_pos] + 3, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                ref_pos += 1;
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVertR2 => {
                            self.add_pixels(ref_line[ref_pos] + 2, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                ref_pos += 1;
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVertR1 => {
                            self.add_pixels(ref_line[ref_pos] + 1, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                ref_pos += 1;
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVert0 => {
                            self.add_pixels(ref_line[ref_pos], black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                ref_pos += 1;
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVertL3 => {
                            self.add_pixels_neg(ref_line[ref_pos] - 3, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                if ref_pos > 0 {
                                    ref_pos -= 1;
                                } else {
                                    ref_pos += 1;
                                }
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVertL2 => {
                            self.add_pixels_neg(ref_line[ref_pos] - 2, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                if ref_pos > 0 {
                                    ref_pos -= 1;
                                } else {
                                    ref_pos += 1;
                                }
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == twoDimVertL1 => {
                            self.add_pixels_neg(ref_line[ref_pos] - 1, black_pixels);
                            black_pixels ^= true;
                            if coding_line[self.coding_pos] < columns as u32 {
                                if ref_pos > 0 {
                                    ref_pos -= 1;
                                } else {
                                    ref_pos += 1;
                                }
                                while ref_line[ref_pos] <= coding_line[self.coding_pos] && ref_line[ref_pos] < columns as u32 {
                                    ref_pos += 2;
                                }
                            }
                        }
                        x if x == ccittEOF => {
                            self.add_pixels(columns as u32, false);
                            self.eof = true;
                        }
                        _ => {
                            println!("bad 2d code");
                            self.add_pixels(columns as u32, false);
                            self.err = true;
                        }
                    }
                }
            } else {
                coding_line[0] = 0;
                self.coding_pos = 0;
                black_pixels = false;
                while coding_line[self.coding_pos] < columns as u32 {
                    code1 = 0;
                    if black_pixels {
                        loop {
                            code3 = self.get_black_code();
                            code1 += code3;
                            if code3 < 64 {
                                break;
                            }
                        }
                    } else {
                        loop {
                            code3 = self.get_white_code();
                            code1 += code3;
                            if code3 < 64 {
                                break;
                            }
                        }
                    }
                    self.add_pixels(coding_line[self.coding_pos] + code1 as u32, black_pixels);
                    black_pixels ^= true;
                }
            }

            let mut got_eol = false;

            if self.byte_align {
                self.input_bits &= !7;
            }

            if !self.eoblock && self.row == self.rows - 1 {
                self.rows_done = true;
            } else {
                code1 = self.look_bits(12);
                if self.eoline {
                    while code1 != ccittEOF && code1 != 1 {
                        self.eat_bits(1);
                        code1 = self.look_bits(12);
                    }
                } else {
                    while code1 == 0 {
                        self.eat_bits(1);
                        code1 = self.look_bits(12);
                    }
                }
                if code1 == 1 {
                    self.eat_bits(12);
                    got_eol = true;
                } else if code1 == ccittEOF {
                    self.eof = true;
                }
            }

            if !self.eof && self.encoding > 0 && !self.rows_done {
                self.next_line_2d = self.look_bits(1) == 0;
                self.eat_bits(1);
            }

            if self.eoblock && got_eol && self.byte_align {
                code1 = self.look_bits(12);
                if code1 == 1 {
                    self.eat_bits(12);
                    if self.encoding > 0 {
                        self.look_bits(1);
                        self.eat_bits(1);
                    }
                    if self.encoding >= 0 {
                        for _ in 0..4 {
                            code1 = self.look_bits(12);
                            if code1 != 1 {
                                println!("bad rtc code: {}", code1);
                            }
                            self.eat_bits(12);
                            if self.encoding > 0 {
                                self.look_bits(1);
                                self.eat_bits(1);
                            }
                        }
                    }
                    self.eof = true;
                }
            } else if self.err && self.eoline {
                loop {
                    code1 = self.look_bits(13);
                    if code1 == ccittEOF {
                        self.eof = true;
                        return -1;
                    }
                    if code1 >> 1 == 1 {
                        break;
                    }
                    self.eat_bits(1);
                }
                self.eat_bits(12);
                if self.encoding > 0 {
                    self.eat_bits(1);
                    self.next_line_2d = (code1 & 1) == 0;
                }
            }

            self.output_bits = if coding_line[0] > 0 {
                self.coding_pos = 0;
                coding_line[0]
            } else {
                self.coding_pos = 1;
                coding_line[1]
            } as usize;
            self.row += 1;
        }

        let mut c;
        if self.output_bits >= 8 {
            c = if self.coding_pos & 1 != 0 { 0 } else { 0xff };
            self.output_bits -= 8;
            if self.output_bits == 0 && coding_line[self.coding_pos] < columns as u32 {
                self.coding_pos += 1;
                self.output_bits = (coding_line[self.coding_pos] - coding_line[self.coding_pos - 1]) as usize;
            }
        } else {
            bits = 8;
            c = 0;
            loop {
                if self.output_bits > bits {
                    c <<= bits;
                    if self.coding_pos & 1 == 0 {
                        c |= 0xff >> (8 - bits);
                    }
                    self.output_bits -= bits;
                    bits = 0;
                } else {
                    c <<= self.output_bits;
                    if self.coding_pos & 1 == 0 {
                        c |= 0xff >> (8 - self.output_bits);
                    }
                    bits -= self.output_bits;
                    self.output_bits = 0;
                    if coding_line[self.coding_pos] < columns as u32 {
                        self.coding_pos += 1;
                        self.output_bits = (coding_line[self.coding_pos] - coding_line[self.coding_pos - 1]) as usize;
                    } else if bits > 0 {
                        c <<= bits;
                        bits = 0;
                    }
                }
                if bits == 0 {
                    break;
                }
            }
        }
        if self.black {
            c ^= 0xff;
        }
        c
    }
}

pub struct CCITTFaxDecoderOptions {
    pub k: Option<i32>,
    pub end_of_line: Option<bool>,
    pub encoded_byte_align: Option<bool>,
    pub columns: Option<usize>,
    pub rows: Option<usize>,
    pub eoblock: Option<bool>,
    pub black_is_1: Option<bool>,
}

impl Default for CCITTFaxDecoderOptions {
    fn default() -> Self {
        Self {
            k: None,
            end_of_line: None,
            encoded_byte_align: None,
            columns: None,
            rows: None,
            eoblock: None,
            black_is_1: None,
        }
    }
} 