#[macro_export]
macro_rules! rom_addr {
    ($bank:expr, $offset:expr) => {
        ((($bank - 0x80) << 15) + ($offset - 0x8000)) as usize
    };
}

pub const ROOM_MDB_START: usize = rom_addr!(0x8f, 0x91f8);
pub const ROOM_MDB_ELEMENT_SIZE: usize = 0xBB;

#[cfg(test)]
mod tests {
    #[test]
    fn rom_addr_macro_works() {
        assert_eq!(rom_addr!(0x8f, 0x93fe), 0x793fe);
    }
}
