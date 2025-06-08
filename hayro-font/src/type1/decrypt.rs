use super::stream::Stream;
use log::{error, warn};

pub(crate) fn decrypt(data: &[u8]) -> Option<Vec<u8>> {
    let mut stream = Stream::new(data);
    stream.skip_whitespaces();

    let mut b00 = None;
    let mut r: u32 = 55665;

    let mut decrypt = |b: u8| decrypt_byte(b, &mut r);

    for _ in 0..1000 {
        let c = stream.read_byte()?;
        if !is_white_space_after_token_eexec(c) {
            b00 = Some(c);
            break;
        }
    }

    let Some(b00) = b00 else {
        error!("b00 was None");

        return None;
    };

    let mut b = [0u8; 4];
    b[0] = b00;

    for i in 1..=3 {
        let c = stream.read_byte()?;
        b[i] = c;
    }

    let mut is_bin = false;

    for i in 0..4 {
        if !b[i].is_ascii_hexdigit() {
            is_bin = true;
        }
    }

    if is_bin {
        let mut out = vec![];

        for i in 0..4 {
            decrypt(b[i]);
        }

        for b in stream.tail()? {
            out.push(decrypt(*b));
        }

        Some(out)
    } else {
        warn!("non-binary decryption is unimplemented");

        None
    }
}

pub(crate) fn decrypt_byte(cipher: u8, r: &mut u32) -> u8 {
    let cipher = cipher as u32;
    let plain = cipher ^ (*r >> 8);
    *r = ((cipher + *r).wrapping_mul(52845) + 22719) & 0xFFFF;
    (plain & 0xFF) as u8
}

fn is_white_space_after_token_eexec(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}
