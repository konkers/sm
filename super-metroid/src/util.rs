use std::io;
use std::io::prelude::*;
use std::io::{Cursor, Read};

pub struct RomReader<'a> {
    cursor: Cursor<&'a [u8]>,
    base_offset: usize,
}

impl<'a> RomReader<'a> {
    pub fn new(data: &'a [u8], base_offset: usize) -> RomReader<'a> {
        RomReader {
            cursor: Cursor::new(&data[base_offset..]),
            base_offset: base_offset,
        }
    }

    pub fn cur_address(&self) -> usize {
        self.base_offset + self.cursor.position() as usize
    }
}

impl<'a> Read for RomReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.cursor.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_reader() {
        let data = [
            0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf,
        ];

        let mut r = RomReader::new(&data, 0x4);
        assert_eq!(r.cur_address(), 0x4);

        let mut rdata = [0; 4];
        let size = r.read(&mut rdata).unwrap();
        assert_eq!(size, 4);
        assert_eq!(rdata, [0x4, 0x5, 0x6, 0x7]);
        assert_eq!(r.cur_address(), 0x8);
    }
}
