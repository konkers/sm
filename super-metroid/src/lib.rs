pub mod rommap;

use byteorder::{LittleEndian, ReadBytesExt};
use failure::{format_err, Error};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use serde::Serialize;
use serde_hex::{CompactPfx, SerHex};

#[derive(Debug, FromPrimitive, Serialize)]
#[repr(u8)]
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

#[derive(Debug, FromPrimitive, PartialEq, Serialize)]
#[repr(u8)]
pub enum Event {
    ZebesAwake = 0x00,
    SuperMetroidAteSideHopper = 0x01,
    MotherBrainGlassBroken = 0x02,
    Zebetite1Destroyed = 0x03,
    Zebetite2Destroyed = 0x04,
    Zebetite3Destroyed = 0x05,
    PhantoonStatueGrey = 0x06,
    RidleyStatueGrey = 0x07,
    DraygonStatueGrey = 0x08,
    KraidStatueGrey = 0x09,
    TourianUnlocked = 0x0a,
    MaridiaTubeBroken = 0x0b,
    LowerNorfairAcidLowered = 0x0c,
    ShaktoolPathClear = 0x0d,
    ZebesTimeBombSet = 0x0e,
    AnimalsSaved = 0x0f,
    FirstMetroidHallClear = 0x10,
    FirstMetroidShaftClear = 0x11,
    SecondMetroidHallClear = 0x12,
    SecondMetroidShaftClear = 0x13,
    Unused = 0x14,
    OutranSpeedBoosterLavaQuake = 0x15,
}

#[derive(Debug, FromPrimitive, Serialize)]
#[repr(u16)]
pub enum StateConditionValue {
    Default = 0xe5e6,
    DoorPointerIs = 0xe5eb,
    MainAreaBossDead = 0xe5ff,
    EventSet = 0xe612,
    AreaBossesDead = 0xe629,
    HasMorphBall = 0xe640,
    HasMorphBallAndMissiles = 0xe652,
    HasPowerBombs = 0xe669,
    HasSpeedBooster = 0xe678,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum StateCondition {
    Default,
    DoorPointerIs { value: u16 },
    MainAreaBossDead,
    EventSet { event: Event },
    AreaBossesDead { bosses: u8 }, // 0x1 == main boss, 0x2 == mini boss, 0x4 = torizo.
    HasMorphBall,
    HasMorphBallAndMissiles,
    HasPowerBombs,
    HasSpeedBooster,
}

#[derive(Debug, Serialize)]
pub struct StateData {
    #[serde(with = "SerHex::<CompactPfx>")]
    level_data: u32,
    tile_set: u8,
    music_data_index: u8,
    music_track: u8,
    #[serde(with = "SerHex::<CompactPfx>")]
    fx_ptr: u16, // Bank $83
    #[serde(with = "SerHex::<CompactPfx>")]
    enemy_population: u16, // bank $a1
    #[serde(with = "SerHex::<CompactPfx>")]
    enemy_set: u16, // bank $b4
    layer_2_scroll_x: u8,
    layer_2_scroll_y: u8,
    #[serde(with = "SerHex::<CompactPfx>")]
    scroll_ptr: u16, // bank $8f?
    #[serde(with = "SerHex::<CompactPfx>")]
    x_ray_block_ptr: u16, // bank ??
    #[serde(with = "SerHex::<CompactPfx>")]
    main_asm_ptr: u16, // bank ??
    plm_ptr: u16, // bank ??
    #[serde(with = "SerHex::<CompactPfx>")]
    bg_ptr: u16, // bank ??
    #[serde(with = "SerHex::<CompactPfx>")]
    setup_asm_ptr: u16, // bank ??
}

#[derive(Debug, Serialize)]
pub struct State {
    pub condition: StateCondition,
    pub data: StateData,
}

#[derive(Debug, Serialize)]
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
    #[serde(with = "SerHex::<CompactPfx>")]
    pub door_list_ptr: u16,

    pub states: Vec<State>,
}

#[derive(Debug, Serialize)]
pub struct SuperMetroidData {
    pub room_mdb: Vec<RoomMdb>,
}

fn load_room_mdb_header(data: &[u8]) -> Result<RoomMdb, Error> {
    Ok(RoomMdb {
        // 00 - Room index
        index: data[0x00],
        // 01 - Room area
        area: Area::from_u8(data[0x01]).ok_or(format_err!("unknown area type."))?,
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
        door_list_ptr: (&data[0x09..=0x0a]).read_u16::<LittleEndian>()?,
        states: Vec::new(),
    })
}

// State conditions are 2 bytes codes followed by 0, 1, or 2 bytes of parameter data.
fn load_state_condition(data: &[u8], offset: usize) -> Result<(StateCondition, usize), Error> {
    let condition_value_raw = (&data[offset..]).read_u16::<LittleEndian>()?;
    let condition_value = StateConditionValue::from_u16(condition_value_raw).ok_or(format_err!(
        "unknown state condition {:04x}",
        condition_value_raw
    ))?;

    let offset = offset + 2;
    Ok(match condition_value {
        StateConditionValue::Default => (StateCondition::Default, offset),
        StateConditionValue::DoorPointerIs => (
            StateCondition::DoorPointerIs {
                value: (&data[offset..]).read_u16::<LittleEndian>()?,
            },
            offset + 2,
        ),
        StateConditionValue::MainAreaBossDead => (StateCondition::MainAreaBossDead, offset),
        StateConditionValue::EventSet => (
            StateCondition::EventSet {
                event: Event::from_u8(data[offset])
                    .ok_or(format_err!("unknown event {:02x}", data[offset]))?,
            },
            offset + 1,
        ),
        StateConditionValue::AreaBossesDead => (
            StateCondition::AreaBossesDead {
                bosses: data[offset],
            },
            offset + 1,
        ),
        StateConditionValue::HasMorphBall => (StateCondition::HasMorphBall, offset),
        StateConditionValue::HasMorphBallAndMissiles => {
            (StateCondition::HasMorphBallAndMissiles, offset)
        }
        StateConditionValue::HasPowerBombs => (StateCondition::HasPowerBombs, offset),
        StateConditionValue::HasSpeedBooster => (StateCondition::HasSpeedBooster, offset),
    })
}

fn load_state_data(data: &[u8]) -> Result<StateData, Error> {
    Ok(StateData {
        level_data: (&data[0x0..]).read_u24::<LittleEndian>()?,
        tile_set: data[0x3],
        music_data_index: data[0x4],
        music_track: data[0x5],
        fx_ptr: (&data[0x6..]).read_u16::<LittleEndian>()?,
        enemy_population: (&data[0x8..]).read_u16::<LittleEndian>()?,
        enemy_set: (&data[0xa..]).read_u16::<LittleEndian>()?,
        layer_2_scroll_x: data[0xc],
        layer_2_scroll_y: data[0xd],
        scroll_ptr: (&data[0xe..]).read_u16::<LittleEndian>()?,
        x_ray_block_ptr: (&data[0x10..]).read_u16::<LittleEndian>()?,
        main_asm_ptr: (&data[0x12..]).read_u16::<LittleEndian>()?,
        plm_ptr: (&data[0x14..]).read_u16::<LittleEndian>()?,
        bg_ptr: (&data[0x16..]).read_u16::<LittleEndian>()?,
        setup_asm_ptr: (&data[0x18..]).read_u16::<LittleEndian>()?,
    })
}

fn load_room_mdb(rom_data: &[u8], offset: usize) -> Result<RoomMdb, Error> {
    let mut mdb = load_room_mdb_header(&rom_data[offset..])?;
    let mut state_offset = offset + 0xb;

    loop {
        let (condition, new_offset) = load_state_condition(rom_data, state_offset)?;
        state_offset = new_offset;

        let (data_ptr, done) = match condition {
            // The default condition's state data immediately follows the end of the
            // end of the state condition list.  It also signifies the end of that
            // list.
            StateCondition::Default => (((state_offset & 0x7fff) + 0x8000) as u16, true),

            // For all other conditions, the state data is pointed to by the next u16.
            _ => (
                {
                    let ptr = (&rom_data[state_offset..]).read_u16::<LittleEndian>()?;
                    state_offset += 2;
                    ptr
                },
                false,
            ),
        };

        let data_offset = rom_addr!(0x8f, data_ptr);
        println!("{:x}", data_offset);
        mdb.states.push(State {
            condition: condition,
            data: load_state_data(&rom_data[rom_addr!(0x8f, data_ptr)..])?,
        });
        if done {
            break;
        }
    }

    Ok(mdb)
}

pub fn load(rom_data: &[u8]) -> Result<SuperMetroidData, Error> {
    if rom_data.len() != 0x300000 {
        return Err(format_err!("Rom data is wrong size."));
    }

    // TODO: verify checksum/crc/other hash.

    let room_mdb = vec![load_room_mdb(rom_data, rommap::ROOM_MDB_START)?];

    Ok(SuperMetroidData { room_mdb: room_mdb })
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
