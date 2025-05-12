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

use std::io::Read;

use crate::filter::ccitt::{CCITTFaxDecoder, CCITTFaxDecoderOptions, CcittFaxSource};
use crate::object::dict::Dict;
use crate::object::dict::keys::{
    BLACK_IS_1, COLUMNS, ENCODED_BYTE_ALIGN, END_OF_BLOCK, END_OF_LINE, K, ROWS,
};

pub fn decode(data: &[u8], params: Dict) -> Option<Vec<u8>> {
    let dp = CCITTFaxDecoderOptions::default();

    let params = CCITTFaxDecoderOptions {
        k: params.get::<i32>(K).unwrap_or(dp.k),
        end_of_line: params.get::<bool>(END_OF_LINE).unwrap_or(dp.end_of_line),
        encoded_byte_align: params
            .get::<bool>(ENCODED_BYTE_ALIGN)
            .unwrap_or(dp.encoded_byte_align),
        columns: params.get::<usize>(COLUMNS).unwrap_or(dp.columns),
        rows: params.get::<usize>(ROWS).unwrap_or(dp.rows),
        eoblock: params.get::<bool>(END_OF_BLOCK).unwrap_or(dp.eoblock),
        black_is_1: params.get::<bool>(BLACK_IS_1).unwrap_or(dp.black_is_1),
    };

    // if params.k < 0 {
    //     let columns = params.columns as usize;
    //     let rows = params.rows as usize;
    //
    //     let height = if params.rows == 0 {
    //         None
    //     } else {
    //         Some(params.rows as u16)
    //     };
    //     let mut buf = Vec::with_capacity(columns * rows);
    //     decode_g4(data.iter().cloned(), columns as u16, height, |line| {
    //         buf.extend(pels(line, columns as u16).map(|c| match c {
    //             Color::Black => 0,
    //             Color::White => 255,
    //         }));
    //         assert_eq!(
    //             buf.len() % columns,
    //             0,
    //             "len={}, columns={}",
    //             buf.len(),
    //             columns
    //         );
    //     })?;
    //     assert_eq!(
    //         buf.len() % columns,
    //         0,
    //         "len={}, columns={}",
    //         buf.len(),
    //         columns
    //     );
    //
    //     if rows != 0 && buf.len() != columns * rows {
    //         panic!(
    //             "decoded length does not match (expected {rows}∙{columns}, got {})",
    //             buf.len()
    //         );
    //     }
    //     Some(buf)
    // } else {
    //     unimplemented!()
    // }

    let mut stream = CCITTFaxStream::new(std::io::Cursor::new(data), params);

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
