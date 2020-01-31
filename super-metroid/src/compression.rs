use byteorder::{LittleEndian, ReadBytesExt};
use failure::{format_err, Error};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use std::io::Cursor;
use std::num::Wrapping;

#[derive(Debug, FromPrimitive, PartialEq)]
enum Op {
    DirectCopy = 0x0,
    ByteFill = 0x1,
    WordFill = 0x2,
    SigmaFill = 0x3,
    LibraryCopy = 0x4,
    XorCopy = 0x5,
    SubtractCopy = 0x6,
    ExtendedCmd = 0x7,
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    // Algorithm from http://patrickjohnston.org/ASM/ROM%20data/Super%20Metroid/decompress.py
    // and https://www.romhacking.net/documents/243/

    let mut r = Cursor::new(data);
    let mut out = Vec::new();

    loop {
        let b = r.read_u8()?;

        if b == 0xff {
            break;
        }

        let mut op = Op::from_u8(b >> 5).ok_or(format_err!("unknown op"))?;

        let size = if op == Op::ExtendedCmd {
            // Extended ops are encoded as
            // |7  6  5 |4  3  2 |1  0 | |7  6  5  4  3  2  1  0 |
            // +--------+--------+-----+ +-----------------------+
            // |1  1  1 |c2 c1 c0|s9 s8| |s7 s6 s5 s4 s3 s2 s1 s0|
            // +--------+--------+-----+ +-----------------------+
            //
            // Where c[2..0] is the new op and s[9..0] is the size of the op
            op = Op::from_u8((b >> 2) & 0x7).ok_or(format_err!("unknown op"))?;
            (((b as usize & 0x3) << 8) | r.read_u8()? as usize) + 1
        } else {
            // All other ops are encoded as:
            // |7  6  5 |4  3  2  1  0 |
            // +--------+--------------+
            // |c2 c1 c0|s4 s3 s2 s1 s0|
            // +--------+--------------+
            //
            // Where c[2..0] is the op and s[4..0] is the size of the op.
            (b as usize & 0x1f) + 1
        };

        match op {
            Op::DirectCopy => {
                // Copy out <size> bytes from the stream
                for _ in 0..size {
                    out.push(r.read_u8()?);
                }
            }
            Op::ByteFill => {
                // Fill the next <size> bytes with the next byte in the stream.
                let b = r.read_u8()?;
                for _ in 0..size {
                    out.push(b);
                }
            }
            Op::WordFill => {
                // Fill the next <size> bytes with the next word in the stream.
                // If <size> is odd, the final byte will be the lower byte of
                // the word.
                let b = vec![r.read_u8()?, r.read_u8()?];
                for i in 0..size {
                    out.push(b[i & 0x1]);
                }
            }
            Op::SigmaFill => {
                // Fill the next <size> bytes with the next byte from the stream,
                // incrementing it on every write.
                let b = Wrapping(r.read_u8()?);
                for i in 0..size {
                    out.push((b + Wrapping(i as u8)).0);
                }
            }
            Op::LibraryCopy => {
                // Copy the <size> bytes from the output.  The address is
                // specified by the next word in the stream.
                let addr = r.read_u16::<LittleEndian>()? as usize;
                for i in 0..size {
                    out.push(out[addr + i]);
                }
            }
            Op::XorCopy => {
                // Works like library copy except the values are xored with 0xff
                // as they are copied.
                let addr = r.read_u16::<LittleEndian>()? as usize;
                for i in 0..size {
                    out.push(out[addr + i] ^ 0xff);
                }
            }
            Op::SubtractCopy => {
                // the next byte in the stream an offset from the end of the
                // current decompression output.  <size> bytes are copied from
                // that offset.
                let addr = out.len() - r.read_u8()? as usize;
                for i in 0..size {
                    out.push(out[addr + i]);
                }
            }
            Op::ExtendedCmd => {
                // According to the python implementation this works like a
                // combination of SubtractCopy and XorCopy.
                let addr = out.len() - r.read_u8()? as usize;
                for i in 0..size {
                    out.push(out[addr + i] ^ 0xff);
                }
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_copy_op() {
        assert_eq!(
            decompress(&[0x2, 0x1, 0x2, 0x3, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3]
        );
    }

    #[test]
    fn test_byte_fill_op() {
        assert_eq!(decompress(&[0x22, 0x1, 0xff]).unwrap(), vec![0x1, 0x1, 0x1]);
    }

    #[test]
    fn test_word_fill_op() {
        // Aligned
        assert_eq!(
            decompress(&[0x43, 0x55, 0xaa, 0xff]).unwrap(),
            vec![0x55, 0xaa, 0x55, 0xaa]
        );
        // Unaligned
        assert_eq!(
            decompress(&[0x44, 0x55, 0xaa, 0xff]).unwrap(),
            vec![0x55, 0xaa, 0x55, 0xaa, 0x55]
        );
    }

    #[test]
    fn test_sigma_fill_op() {
        assert_eq!(
            decompress(&[0x64, 0x1, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3, 0x4, 0x5]
        );

        // Test overflow
        assert_eq!(
            decompress(&[0x64, 0xfe, 0xff]).unwrap(),
            vec![0xfe, 0xff, 0x00, 0x01, 0x02]
        );
    }

    #[test]
    fn test_library_copy_op() {
        assert_eq!(
            decompress(&[0x64, 0x1, 0x82, 0x01, 0x00, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x2, 0x3, 0x4]
        );
    }

    #[test]
    fn test_xor_copy_op() {
        assert_eq!(
            decompress(&[0x64, 0x1, 0xa2, 0x01, 0x00, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x2 ^ 0xff, 0x3 ^ 0xff, 0x4 ^ 0xff]
        );
    }

    #[test]
    fn test_subtract_copy_op() {
        assert_eq!(
            decompress(&[0x64, 0x1, 0xc2, 0x03, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x3, 0x4, 0x5]
        );
    }

    #[test]
    fn test_extended_and_subtract_xor_copy_ops() {
        assert_eq!(
            decompress(&[0x64, 0x1, 0xfc, 0x02, 0x03, 0xff]).unwrap(),
            vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x3 ^ 0xff, 0x4 ^ 0xff, 0x5 ^ 0xff]
        );
    }
}
