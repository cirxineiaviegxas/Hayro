use crate::object::dict::Dict;
use crate::object::dict::keys::{BITS_PER_COMPONENT, COLORS, COLUMNS, EARLY_CHANGE, PREDICTOR};
use crate::reader::Reader;

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

fn apply_up(
    prev_row: Option<&[u8]>,
    cur_row: &[u8],
    out: &mut [u8],
    params: &PredictorParams,
) -> Option<()> {
    for i in 0..params.columns {
        let prev = prev_row.map(|p| p[i]).unwrap_or(0);
        out[i] = cur_row[i].wrapping_add(prev);
    }

    Some(())
}

fn apply_predictor(data: Vec<u8>, params: &PredictorParams) -> Option<Vec<u8>> {
    match params.predictor {
        1 => Some(data),
        12 => {
            let num_cols = params.row_length_in_bytes();
            // +1 Because each row must start with the predictor that is used.
            let num_rows = data.len() / (num_cols + 1);

            // Sanity check.
            if num_rows * (num_cols + 1) != data.len() {
                return None;
            }

            let mut out = vec![0; num_rows * num_cols];
            let mut r = Reader::new(&data);

            for i in 0..num_rows {
                let _predictor_byte = r.read_byte()?;

                let row_start = num_cols * i;
                let row_end = num_cols * (i + 1);

                let in_data = r.read_bytes(num_cols)?;
                let (last_row, out_data) = if i == 0 {
                    (None, &mut out[row_start..row_end])
                } else {
                    let prev_row_start = num_cols * (i - 1);
                    let range = &mut out[prev_row_start..row_end];
                    let (last_row, out_data) = range.split_at_mut(num_cols);
                    (Some(&*last_row), out_data)
                };

                apply_up(last_row, in_data, out_data, params)?;
            }

            Some(out)
        }
        _ => unimplemented!(),
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

#[cfg(test)]
mod tests {
    use crate::filter::lzw_flate::lzw;

    #[test]
    fn simple_lzw() {
        let input = [0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];
        let decoded = lzw::decode(&input, None).unwrap();

        assert_eq!(decoded, vec![45, 45, 45, 45, 45, 65, 45, 45, 45, 66]);
    }
}
