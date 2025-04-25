use crate::object::dict::Dict;
use crate::object::dict::keys::{BITS_PER_COMPONENT, COLORS, COLUMNS, EARLY_CHANGE, PREDICTOR};
use itertools::izip;

struct PredictorParams {
    predictor: u8,
    colors: u8,
    bits_per_component: u8,
    columns: usize,
    early_change: bool,
}

impl PredictorParams {
    fn bits_per_pixel(&self) -> u8 {
        self.bits_per_component * self.colors
    }

    fn bytes_per_pixel(&self) -> u8 {
        (self.bits_per_pixel() + 7) / 8
    }

    fn row_length_in_bytes(&self) -> usize {
        self.columns * self.bytes_per_pixel() as usize
    }
}

impl Default for PredictorParams {
    fn default() -> Self {
        Self {
            predictor: 1,
            colors: 1,
            bits_per_component: 8,
            columns: 1,
            early_change: true,
        }
    }
}

impl PredictorParams {
    fn from_params(dict: &Dict) -> Self {
        Self {
            predictor: dict.get(PREDICTOR).unwrap_or(1),
            colors: dict.get(COLORS).unwrap_or(1),
            bits_per_component: dict.get(BITS_PER_COMPONENT).unwrap_or(8),
            columns: dict.get(COLUMNS).unwrap_or(1),
            early_change: dict
                .get::<u8>(EARLY_CHANGE)
                .map(|e| if e == 0 { false } else { true })
                .unwrap_or(true),
        }
    }
}

pub mod flate {
    use crate::filter::lzw_flate::{PredictorParams, apply_predictor};
    use crate::object::dict::Dict;

    pub fn decode(data: &[u8], params: Option<&Dict>) -> Option<Vec<u8>> {
        let decoded = zlib(data).or_else(|| deflate(data))?;
        let params = params
            .map(|p| PredictorParams::from_params(p))
            .unwrap_or_default();
        apply_predictor(decoded, &params)
    }

    fn zlib(data: &[u8]) -> Option<Vec<u8>> {
        miniz_oxide::inflate::decompress_to_vec_zlib(data).ok()
    }

    fn deflate(data: &[u8]) -> Option<Vec<u8>> {
        miniz_oxide::inflate::decompress_to_vec(data).ok()
    }
}

pub mod lzw {
    use crate::filter::lzw_flate::{PredictorParams, apply_predictor};

    use crate::object::dict::Dict;
    use bitreader::BitReader;

    pub fn decode(data: &[u8], params: Option<&Dict>) -> Option<Vec<u8>> {
        let params = params
            .map(|p| PredictorParams::from_params(p))
            .unwrap_or_default();

        let decoded = decode_impl(data, params.early_change)?;

        apply_predictor(decoded, &params)
    }

    const CLEAR_TABLE: usize = 256;
    const EOD: usize = 257;
    const MAX_ENTRIES: usize = 4096;
    const INITIAL_SIZE: u16 = 258;

    fn decode_impl(data: &[u8], early_change: bool) -> Option<Vec<u8>> {
        let mut table = Table::new(early_change);

        let mut reader = BitReader::new(data);
        let mut decoded = vec![];
        let mut prev = None;

        loop {
            let next = reader.read_u16(table.code_length()).ok()? as usize;

            match next {
                CLEAR_TABLE => {
                    table.clear();
                    prev = None;
                }
                EOD => return Some(decoded),
                new => {
                    if let Some(entry) = table.get(new) {
                        decoded.extend_from_slice(entry);

                        if let Some(prev) = prev {
                            let _ = table.register(prev, entry[0])?;
                        }
                    } else {
                        let prev = prev?;
                        let new_byte = table.get(prev)?[0];

                        decoded.extend_from_slice(table.register(prev, new_byte)?);
                    }

                    prev = Some(new);
                }
            }
        }
    }

    struct Table {
        early_change: bool,
        entries: Vec<Vec<u8>>,
    }

    impl Table {
        fn new(early_change: bool) -> Self {
            let mut entries: Vec<_> = (0..=255).map(|b| vec![b]).collect();

            // Clear table and EOD don't have any data.
            entries.push(vec![0]);
            entries.push(vec![0]);

            Self {
                early_change,
                entries,
            }
        }

        fn push(&mut self, entry: Vec<u8>) -> Option<&[u8]> {
            if self.entries.len() >= MAX_ENTRIES {
                None
            } else {
                self.entries.push(entry);

                self.entries.last().map(|v| &**v)
            }
        }

        fn register(&mut self, prev: usize, new_byte: u8) -> Option<&[u8]> {
            let prev_entry = self.get(prev)?;

            let mut new_entry = Vec::with_capacity(prev_entry.len() + 1);
            new_entry.extend(prev_entry);
            new_entry.push(new_byte);
            self.push(new_entry)
        }

        fn get(&self, index: usize) -> Option<&[u8]> {
            self.entries.get(index).map(|v| &**v)
        }

        fn clear(&mut self) {
            self.entries.truncate(INITIAL_SIZE as usize);
        }

        fn code_length(&self) -> u8 {
            const TEN: usize = 512;
            const ELEVEN: usize = 1024;
            const TWELVE: usize = 2048;

            let adjusted = self.entries.len() + (if self.early_change { 1 } else { 0 });

            if adjusted >= TWELVE {
                12
            } else if adjusted >= ELEVEN {
                11
            } else if adjusted >= TEN {
                10
            } else {
                9
            }
        }
    }
}

fn apply_predictor(data: Vec<u8>, params: &PredictorParams) -> Option<Vec<u8>> {
    if params.bits_per_component != 8 {
        unimplemented!();
    }

    match params.predictor {
        1 | 10 => Some(data),
        i if i >= 10 => {
            let row_len = params.row_length_in_bytes();
            // + 1 Because each row must start with the predictor that is used.
            let total_row_len = row_len + 1;
            let num_rows = data.len() / total_row_len;

            // Sanity check.
            if num_rows * total_row_len != data.len() {
                return None;
            }

            let colors = params.colors as usize;

            let zero_row = vec![0; row_len];
            let zero_col = vec![0; colors];

            let mut prev_row: &[u8] = &zero_row;

            let mut out = vec![0; num_rows * row_len];

            for (in_row, out_row) in data
                .chunks_exact(total_row_len)
                .zip(out.chunks_exact_mut(row_len))
            {
                let predictor = in_row[0];
                let in_data = &in_row[1..];

                match predictor {
                    0 => out_row.copy_from_slice(in_data),
                    1 => apply::<Sub>(&prev_row, &zero_col, &zero_col, in_data, out_row, colors),
                    2 => apply::<Up>(&prev_row, &zero_col, &zero_col, in_data, out_row, colors),
                    3 => apply::<Avg>(&prev_row, &zero_col, &zero_col, in_data, out_row, colors),
                    4 => apply::<Paeth>(&prev_row, &zero_col, &zero_col, in_data, out_row, colors),
                    _ => unreachable!(),
                }

                prev_row = out_row;
            }

            Some(out)
        }
        _ => unimplemented!(),
    }
}

fn apply<'a, T: Predictor>(
    prev_row: &'a [u8],
    mut prev_col: &'a [u8],
    mut top_left: &'a [u8],
    cur_row: &'a [u8],
    out: &'a mut [u8],
    colors: usize,
) {
    let cur_row = cur_row.chunks_exact(colors);
    let prev_row = prev_row.chunks_exact(colors);
    let out_row = out.chunks_exact_mut(colors);

    for (cur_row, prev_row, out_row) in izip!(cur_row, prev_row, out_row) {
        for (cur_row, prev_row, out_row, prev_col, top_left) in
            izip!(cur_row, prev_row, out_row.iter_mut(), prev_col, top_left)
        {
            *out_row = T::predict(*cur_row, *prev_row, *prev_col, *top_left);
        }

        prev_col = out_row;
        top_left = prev_row;
    }
}

trait Predictor {
    fn predict(cur_row: u8, prev_row: u8, prev_col: u8, top_left: u8) -> u8;
}

struct Sub;
impl Predictor for Sub {
    fn predict(cur_row: u8, _: u8, prev_col: u8, _: u8) -> u8 {
        cur_row.wrapping_add(prev_col)
    }
}

struct Up;
impl Predictor for Up {
    fn predict(cur_row: u8, prev_row: u8, _: u8, _: u8) -> u8 {
        cur_row.wrapping_add(prev_row)
    }
}

struct Avg;
impl Predictor for Avg {
    fn predict(cur_row: u8, prev_row: u8, prev_col: u8, _: u8) -> u8 {
        cur_row.wrapping_add(((prev_col as u16 + prev_row as u16) / 2) as u8)
    }
}

struct Paeth;
impl Predictor for Paeth {
    fn predict(cur_row: u8, prev_row: u8, prev_col: u8, top_left: u8) -> u8 {
        fn paeth(a: u8, b: u8, c: u8) -> u8 {
            let a = a as i16;
            let b = b as i16;
            let c = c as i16;

            let p = a + b - c;
            let pa = (p - a).abs();
            let pb = (p - b).abs();
            let pc = (p - c).abs();

            if pa <= pb && pa <= pc {
                a as u8
            } else if pb <= pc {
                b as u8
            } else {
                c as u8
            }
        }

        cur_row.wrapping_add(paeth(prev_col, prev_row, top_left))
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use crate::filter::lzw_flate::{PredictorParams, apply_predictor, flate, lzw};

    #[test]
    fn decode_lzw() {
        let input = [0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];
        let decoded = lzw::decode(&input, None).unwrap();

        assert_eq!(decoded, vec![45, 45, 45, 45, 45, 65, 45, 45, 45, 66]);
    }

    #[test]
    fn decode_flate_zlib() {
        let input = [
            0x78, 0x9c, 0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x7, 0x0, 0x5, 0x8c, 0x1, 0xf5,
        ];

        let decoded = flate::decode(&input, None).unwrap();
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn decode_flate() {
        let input = [0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x7, 0x0];

        let decoded = flate::decode(&input, None).unwrap();
        assert_eq!(decoded, b"Hello");
    }
    
    fn predictor_expected() -> Vec<u8> {
        vec![
            // Row 1
            127, 127, 127, 125, 129, 127, 123, 130, 128, 
            // Row 2
            128, 129, 126, 126, 132, 124, 121, 127, 126, 
            // Row 3
            131, 130, 122, 133, 129, 128, 127, 100, 126,
        ]
    }

    fn predictor_test(predictor: u8, input: &[u8]) {
        let params = PredictorParams {
            predictor,
            colors: 3,
            bits_per_component: 8,
            columns: 3,
            early_change: false,
        };

        let expected = predictor_expected();
        let out = apply_predictor(input.to_vec(), &params).unwrap();

        assert_eq!(expected, out);
    }

    #[test]
    fn predictor_none() {
        predictor_test(10, &predictor_expected());
    }

    #[test]
    fn predictor_sub() {
        predictor_test(
            11,
            &[
                // Row 1
                1, 127, 127, 127, 254, 2, 0, 254, 1, 1, 
                // Row 2
                1, 128, 129, 126, 254, 3, 254, 251, 251, 2, 
                // Row 3
                1, 131, 130, 122, 2, 255, 6, 250, 227, 254,
            ],
        );
    }

    #[test]
    fn predictor_up() {
        predictor_test(
            12,
            &[
                // Row 1
                2, 127, 127, 127, 125, 129, 127, 123, 130, 128, 
                // Row 2
                2, 1, 2, 255, 1, 3, 253, 254, 253, 254, 
                // Row 3
                2, 3, 1, 252, 7, 253, 4, 6, 229, 0,
            ],
        );
    }

    #[test]
    fn predictor_avg() {
        predictor_test(
            13,
            &[
                // Row 1
                3, 127, 127, 127, 62, 66, 64, 61, 66, 65, 
                // Row 2
                3, 65, 66, 63, 0, 3, 254, 253, 252, 0, 
                // Row 3
                3, 67, 66, 59, 5, 254, 5, 0, 228, 255,
            ],
        );
    }

    #[test]
    fn predictor_paeth() {
        predictor_test(
            14,
            &[
                // Row 1
                4, 127, 127, 127, 254, 2, 0, 254, 1, 1, 
                // Row 2
                4, 1, 2, 255, 1, 3, 254, 254, 251, 2, 
                // Row 3
                4, 3, 1, 252, 5, 253, 6, 1, 229, 254,
            ],
        );
    }
}
