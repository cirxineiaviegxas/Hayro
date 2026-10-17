//! Behaviour pinned for streams that end at (or beyond) a row boundary.

use crate::{decode, DecodeError, DecodeSettings, Decoder, DecoderContext, EncodingMode};
use alloc::vec::Vec;

struct TestDecoder {
    pixels: Vec<bool>,
    rows: usize,
}

impl Decoder for TestDecoder {
    fn push_pixel(&mut self, white: bool) {
        self.pixels.push(white);
    }

    fn push_pixel_chunk(&mut self, white: bool, chunk_count: u32) {
        for _ in 0..chunk_count * 8 {
            self.pixels.push(white);
        }
    }

    fn next_line(&mut self) {
        self.rows += 1;
    }
}

fn settings(rows: u32, encoding: EncodingMode) -> DecodeSettings {
    DecodeSettings {
        columns: 8,
        rows,
        end_of_block: false,
        end_of_line: false,
        rows_are_byte_aligned: false,
        encoding,
        invert_black: false,
    }
}

fn decoder() -> TestDecoder {
    TestDecoder { pixels: Vec::new(), rows: 0 }
}

#[test]
fn group4_single_v0_row_decodes_cleanly() {
    // 0b1000_0000 = V(0) first: the reference line is empty, so b1 is the line
    // width and the row completes with eight white pixels.
    let mut d = decoder();
    let mut ctx = DecoderContext::new(settings(1, EncodingMode::Group4));
    let res = decode(&[0b1000_0000], &mut d, &mut ctx);
    assert_eq!(res, Ok(1));
    assert_eq!(d.rows, 1);
    assert_eq!(d.pixels.len(), 8);
    assert!(d.pixels.iter().all(|&p| p));
}

#[test]
fn group4_input_ending_at_a_row_boundary_is_not_an_error() {
    // One byte holding one complete row; the stream ends right at the boundary.
    let mut d = decoder();
    let mut ctx = DecoderContext::new(settings(1, EncodingMode::Group4));
    let res = decode(&[0b1111_1111], &mut d, &mut ctx);
    assert_eq!(res, Ok(1));
    assert_eq!(d.rows, 1);
}

#[test]
fn group4_mid_code_garbage_reports_the_offset() {
    // One row completes (one bit); the second row's mode code runs into a run
    // of zeros that cannot be a valid code, and the error must name the bit
    // offset where the invalid code ended.
    let mut d = decoder();
    let mut ctx = DecoderContext::new(settings(2, EncodingMode::Group4));
    let res = decode(&[0b1000_0000], &mut d, &mut ctx);
    match res {
        Err(DecodeError::InvalidCode { offset }) => assert_eq!(offset, 7),
        other => panic!("expected invalid code at bit 7, got {other:?}"),
    }
    assert_eq!(d.rows, 1);
}

#[test]
fn group3_empty_input_is_mid_code_truncation() {
    // A Group 3 stream cannot end before a row is complete; empty input is a
    // truncation at bit zero, not a clean end.
    let mut d = decoder();
    let mut ctx = DecoderContext::new(settings(1, EncodingMode::Group3_1D));
    let res = decode(&[], &mut d, &mut ctx);
    match res {
        Err(DecodeError::UnexpectedEof { offset }) => assert_eq!(offset, 0),
        other => panic!("expected truncation at bit 0, got {other:?}"),
    }
}
