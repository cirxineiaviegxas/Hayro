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

// See <https://github.com/mozilla/pdf.js/blob/master/src/core/ccitt_stream.js>

use std::io::{self, Read};

use crate::filter::ccitt::{CCITTFaxDecoder, CCITTFaxDecoderOptions, CcittFaxSource};
use crate::object::dict::Dict;
use crate::object::dict::keys::{BLACK_IS_1, COLUMNS, ENCODED_BYTE_ALIGN, END_OF_BLOCK, END_OF_LINE, K, ROWS};

pub fn decode(data: &[u8], params: Dict) -> Option<Vec<u8>> {
    
        let params = CCITTFaxDecoderOptions {
            k: params.get::<i32>(K),
            end_of_line: params.get::<bool>(END_OF_LINE),
            encoded_byte_align: params.get::<bool>(ENCODED_BYTE_ALIGN),
            columns: params.get::<usize>(COLUMNS),
            rows: params.get::<usize>(ROWS),
            eoblock: params.get::<bool>(END_OF_BLOCK),
            black_is_1: params.get::<bool>(BLACK_IS_1),
        };
    
        let mut stream = CCITTFaxStream::new(io::Cursor::new(data), params);

        let mut out = vec![];
    
        loop {
            let byte = stream.decoder.read_next_char();
            if byte == -1 {
                break;
            }

            out.push(byte as u8);
        }
    
        Some(out)
}

/// A stream that reads CCITT fax encoded data and decodes it into bytes.
pub struct CCITTFaxStream<R: Read> {
    decoder: CCITTFaxDecoder<CCITTFaxSourceImpl<R>>,
}

impl<R: Read> CCITTFaxStream<R> {
    /// Creates a new CCITT fax stream from a reader.
    pub fn new(reader: R, options: CCITTFaxDecoderOptions) -> Self {
        Self {
            decoder: CCITTFaxDecoder::new(CCITTFaxSourceImpl::new(reader), options),
        }
    }
}

/// Implementation of CcittFaxSource for a Read trait object
struct CCITTFaxSourceImpl<R: Read> {
    reader: R,
}

impl<R: Read> CCITTFaxSourceImpl<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: Read> CcittFaxSource for CCITTFaxSourceImpl<R> {
    fn next(&mut self) -> Option<u8> {
        let mut buf = [0];
        match self.reader.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            _ => None,
        }
    }
} 