#[macro_export]
macro_rules! rom_addr {
    ($bank:expr, $offset:expr) => {
        ((($bank - 0x80) << 15) as usize + ($offset - 0x8000) as usize)
    };
}

#[macro_export]
macro_rules! snes_to_rom_addr {
    ($addr:expr) => {
        rom_addr!(($addr >> 16) & 0xff, $addr & 0xffff)
    };
}

#[macro_export]
macro_rules! rom_addr_to_snes16 {
    ($addr:expr) => {
        (($addr & 0x7fff) + 0x8000) as u16
    };
}

#[macro_export]
macro_rules! rom_addr_to_snes {
    ($addr:expr) => {
        (0x80_0000 + (($addr << 1) & 0xff_0000) + (($addr & 0x7fff) + 0x8000)) as u32
    };
}

pub const ROOM_MDB_START: usize = rom_addr!(0x8f, 0x91f8);
pub const TILESET_POINTER_TABLE: usize = rom_addr!(0x8f, 0xe7a7);
pub const TILESET_POINTER_TABLE_COUNT: usize = 29;
pub const TILESET_ENTRY_BANK: usize = 0x8f;
pub const CRE_TILES: usize = rom_addr!(0xb9, 0x8000);
pub const CRE_TILE_TABLE: usize = rom_addr!(0xb9, 0xa09d);

#[cfg(test)]
mod tests {
    #[test]
    fn rom_addr_macro_works() {
        assert_eq!(rom_addr!(0x8f, 0x93fe), 0x793fe);
    }
}
