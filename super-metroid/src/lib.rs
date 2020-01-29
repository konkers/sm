use byteorder::{LittleEndian, ReadBytesExt};
use failure::{format_err, Error};
use num::FromPrimitive;
use num_derive::FromPrimitive;

pub mod rommap;

#[derive(Debug, FromPrimitive)]
pub enum Area {
    Crateria = 0x00,
    Brinstar = 0x01,
    Norfair = 0x02,
    WreckedShip = 0x03,
    Maridia = 0x04,
    Tourian = 0x05,
    Ceres = 0x06,
    Debug = 0x07,
}

pub struct RoomMdb {
    pub index: u8,
    pub area: Area,
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub up_scroller: u8,
    pub down_scroller: u8,
    pub graphics_flags: u8,
    pub door_out_ptr: u16,
    pub room_state_used: u16,
    pub level_data_ptr: u32,
    pub tile_set_used: u8,
    pub music_collection: u8,
    pub music_play: u8,
    pub fx1_ptr: u16,
    pub enemy_population_ptr: u16,
    pub enemy_set_ptr: u16,
    pub layer_2_scrolling: u16,
    pub scroll_ptr: u16,
    pub unknown: u16,
    pub fx2_ptr: u16,
    pub plm_ptr: u16,
    pub bg_data: u16,
    pub later1_2: u16,
}

pub struct SuperMetroidData {
    pub room_mdb: Vec<RoomMdb>,
}

fn load_room_mdb(data: &[u8]) -> Result<RoomMdb, Error> {
    if data.len() < rommap::ROOM_MDB_ELEMENT_SIZE {
        return Err(format_err!(
            "Not enought data to read mdb entry. Expected {}, got {}",
            rommap::ROOM_MDB_ELEMENT_SIZE,
            data.len()
        ));
    }

    Ok(RoomMdb {
        // 00 - Room index
        index: data[0x00],
        // 01 - Room area
        area: Area::from_u8(data[0x01]).unwrap(),
        // 02 - X-position on mini-map
        x: data[0x02],
        // 03 - y-position on mini-map
        y: data[0x03],
        // 04 - width of room
        width: data[0x04],
        // 05 - Height of room
        height: data[0x05],
        // 06 - Up scroller
        up_scroller: data[0x06],
        // 07 - Down scroller
        down_scroller: data[0x07],
        // 08 - Special graphics bitflag
        graphics_flags: data[0x08],
        // 09 0a - Door out pointer
        door_out_ptr: (&data[0x09..=0x0a]).read_u16::<LittleEndian>().unwrap(),
        // 0b 0c - Roomstate used
        room_state_used: (&data[0x0b..=0x0c]).read_u16::<LittleEndian>().unwrap(),
        // 0d 0e 0f - Level data pointer
        level_data_ptr: (&data[0x0d..=0x0f]).read_u24::<LittleEndian>().unwrap(),
        // 10 - Tileset used
        tile_set_used: data[0x10],
        // 11 - Music: Collection
        music_collection: data[0x11],
        // 12 - Music: Play
        music_play: data[0x12],
        // 13 14 - FX1 pointer
        fx1_ptr: (&data[0x13..=0x14]).read_u16::<LittleEndian>().unwrap(),
        // 15 16 - Enemy Pop/Allowed pointer
        enemy_population_ptr: (&data[0x15..=0x16]).read_u16::<LittleEndian>().unwrap(),
        // 17 18 - Enemy Set pointer
        enemy_set_ptr: (&data[0x17..=0x18]).read_u16::<LittleEndian>().unwrap(),
        // 19 1a - Layer 2 scrolling
        layer_2_scrolling: (&data[0x19..=0x1a]).read_u16::<LittleEndian>().unwrap(),
        // 1b 1c - Scroll pointer
        scroll_ptr: (&data[0x1b..=0x1c]).read_u16::<LittleEndian>().unwrap(),
        // 1d 1e  - Unknown/RoomVar
        unknown: (&data[0x1d..=0x1e]).read_u16::<LittleEndian>().unwrap(),
        // 1f 20 - FX2 pointer
        fx2_ptr: (&data[0x1f..=0x20]).read_u16::<LittleEndian>().unwrap(),
        // 21 22 - PLM pointer
        plm_ptr: (&data[0x21..=0x22]).read_u16::<LittleEndian>().unwrap(),
        // 23 24 - BG_Data
        bg_data: (&data[0x23..=0x24]).read_u16::<LittleEndian>().unwrap(),
        // 25 26 - Later1_2 (FX0/Setup code)
        later1_2: (&data[0x25..=0x26]).read_u16::<LittleEndian>().unwrap(),
    })
}

pub fn load(rom_data: &[u8]) -> Result<SuperMetroidData, Error> {
    if rom_data.len() != 0x300000 {
        return Err(format_err!("Rom data is wrong size."));
    }

    // TODO: verify checksum/crc/other hash.

    let room_mdb = vec![load_room_mdb(&rom_data[rommap::ROOM_MDB_START..])?];

    Ok(SuperMetroidData { room_mdb: room_mdb })
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
