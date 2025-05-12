use log::warn;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct BitSize(u8);

impl BitSize {
    pub fn from_u8(value: u8) -> Option<Self> {
        if value > 16 { None } else { Some(Self(value)) }
    }

    pub fn bits(&self) -> usize {
        self.0 as usize
    }

    pub fn mask(&self) -> u32 {
        (1 << self.0) - 1
    }
}

pub struct BitReader<'a> {
    data: &'a [u8],
    cur_pos: usize,
    bit_size: BitSize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8], bit_size: BitSize) -> Self {
        Self {
            data,
            bit_size,
            cur_pos: 0,
        }
    }

    pub fn align(&mut self) {
        let bit_pos = self.bit_pos();

        if bit_pos % 8 != 0 {
            self.cur_pos += 8 - bit_pos;
        }
    }

    fn byte_pos(&self) -> usize {
        self.cur_pos / 8
    }

    fn bit_pos(&self) -> usize {
        self.cur_pos % 8
    }
}

impl<'a> Iterator for BitReader<'a> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let byte_pos = self.byte_pos();

        if byte_pos >= self.data.len() {
            return None;
        }

        let bit_size = self.bit_size;

        match bit_size.0 {
            8 => {
                let item = self.data[byte_pos] as u16;
                self.cur_pos += 8;

                Some(item)
            }
            9..=u8::MAX => {
                let bit_pos = self.bit_pos();
                let end_byte_pos = (bit_pos + bit_size.0 as usize - 1) / 8;
                let mut read = [0u8; 4];

                for i in 0..=end_byte_pos {
                    read[i] = *self.data.get(byte_pos + i)?;
                }

                let item = (u32::from_be_bytes(read) >> (32 - bit_pos - bit_size.0 as usize))
                    & bit_size.mask();
                self.cur_pos += bit_size.0 as usize;

                Some(item as u16)
            }
            0..=7 => {
                let bit_pos = self.bit_pos();
                let advance = self.bit_size.bits();
                let item = (self.data[byte_pos] as u16 >> (8 - bit_pos - advance))
                    & self.bit_size.mask() as u16;

                self.cur_pos += advance;

                Some(item)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_reader_16() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut reader = BitReader::new(&data, BitSize::from_u8(16).unwrap());
        assert_eq!(reader.next().unwrap(), u16::from_be_bytes([0x01, 0x02]));
        assert_eq!(reader.next().unwrap(), u16::from_be_bytes([0x03, 0x04]));
        assert_eq!(reader.next().unwrap(), u16::from_be_bytes([0x05, 0x06]));
    }

    #[test]
    fn bit_reader_12() {
        let data = [0b10011000, 0b00011111, 0b10101001, 0b11101001, 0b00011010];
        let mut reader = BitReader::new(&data, BitSize::from_u8(12).unwrap());
        assert_eq!(reader.next().unwrap(), 0b100110000001);
        assert_eq!(reader.next().unwrap(), 0b111110101001);
        assert_eq!(reader.next().unwrap(), 0b111010010001);
    }

    #[test]
    fn bit_reader_9() {
        let data = [0b10011000, 0b00011111, 0b10101001, 0b11101001, 0b00011010];
        let mut reader = BitReader::new(&data, BitSize::from_u8(9).unwrap());
        assert_eq!(reader.next().unwrap(), 0b100110000);
        assert_eq!(reader.next().unwrap(), 0b001111110);
        assert_eq!(reader.next().unwrap(), 0b101001111);
        assert_eq!(reader.next().unwrap(), 0b010010001);
    }

    #[test]
    fn bit_reader_8() {
        let data = [0x01, 0x02, 0x03];
        let mut reader = BitReader::new(&data, BitSize::from_u8(8).unwrap());
        assert_eq!(reader.next().unwrap(), 0x01);
        assert_eq!(reader.next().unwrap(), 0x02);
        assert_eq!(reader.next().unwrap(), 0x03);
    }

    #[test]
    fn bit_reader_4() {
        let data = [0b10011000, 0b00011111, 0b10101001];
        let mut reader = BitReader::new(&data, BitSize::from_u8(4).unwrap());
        assert_eq!(reader.next().unwrap(), 0b1001);
        assert_eq!(reader.next().unwrap(), 0b1000);
        assert_eq!(reader.next().unwrap(), 0b0001);
        assert_eq!(reader.next().unwrap(), 0b1111);
        assert_eq!(reader.next().unwrap(), 0b1010);
        assert_eq!(reader.next().unwrap(), 0b1001);
    }

    #[test]
    fn bit_reader_2() {
        let data = [0b10011000, 0b00010000];
        let mut reader = BitReader::new(&data, BitSize::from_u8(2).unwrap());
        assert_eq!(reader.next().unwrap(), 0b10);
        assert_eq!(reader.next().unwrap(), 0b01);
        assert_eq!(reader.next().unwrap(), 0b10);
        assert_eq!(reader.next().unwrap(), 0b00);
        assert_eq!(reader.next().unwrap(), 0b00);
        assert_eq!(reader.next().unwrap(), 0b01);
        assert_eq!(reader.next().unwrap(), 0b00);
        assert_eq!(reader.next().unwrap(), 0b00);
    }

    #[test]
    fn bit_reader_1() {
        let data = [0b10011000, 0b00010000];
        let mut reader = BitReader::new(&data, BitSize::from_u8(1).unwrap());
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);

        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
    }

    #[test]
    fn bit_reader_align() {
        let data = [0b10011000, 0b00010000];
        let mut reader = BitReader::new(&data, BitSize::from_u8(1).unwrap());
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b1);
        reader.align();

        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b1);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
        assert_eq!(reader.next().unwrap(), 0b0);
    }
}
