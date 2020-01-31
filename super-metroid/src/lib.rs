pub mod compression;
mod rommap;

use byteorder::{LittleEndian, ReadBytesExt};
use failure::{format_err, Error};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use serde::Serialize;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

macro_rules! is_bit_set {
    ($value:expr, $test:expr) => {
        ($value & $test) == $test
    };
}

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
    level_data: u32,
    tile_set: u8,
    music_data_index: u8,
    music_track: u8,
    fx_ptr: u16,           // Bank $83
    enemy_population: u16, // bank $a1
    enemy_set: u16,        // bank $b4
    layer_2_scroll_x: u8,
    layer_2_scroll_y: u8,
    scroll_ptr: u16,      // bank $8f?
    x_ray_block_ptr: u16, // bank ??
    main_asm_ptr: u16,    // bank ??
    plm_ptr: u16,         // bank ??
    bg_ptr: u16,          // bank ??
    setup_asm_ptr: u16,   // bank ??
}

#[derive(Debug, Serialize)]
pub struct State {
    pub condition: StateCondition,
    pub data: StateData,
}

#[derive(Debug, FromPrimitive, PartialEq, Serialize)]
#[repr(u8)]
pub enum BlockType {
    Air = 0x0,
    Slope = 0x1,
    SpikeAir = 0x2,
    SpecialAir = 0x3,
    ShootableAir = 0x4,
    HorizontalExtension = 0x5,
    UnusedAir = 0x6,
    BombableAir = 0x7,
    SolidBlock = 0x8,
    DoorBlock = 0x9,
    SpikeBlock = 0xa,
    SpecialBlock = 0xb,
    ShootableBlock = 0xc,
    VerticalExtension = 0xd,
    GrappleBlock = 0xe,
    BombableBlock = 0xf,
}

#[derive(Debug, Serialize)]
pub struct BlockInfo {
    pub ty: BlockType,
    pub x_flip: bool,
    pub y_flip: bool,
    pub tile_index: u16,
}

#[derive(Debug, Serialize)]
pub struct RoomData {
    // These three come from the data in the rom.
    pub layer_1: Vec<BlockInfo>,
    pub bts: Vec<u8>,
    pub layer_2: Option<Vec<BlockInfo>>,
    // The following are computed on load.
    pub num_doors: usize,
}

#[derive(Debug, Serialize)]
pub struct DoorData {
    pub dest_room_ptr: u16, // bank 0x8f
    pub elevator_props: u8,
    pub orientation: u8,
    pub x: u16,
    pub y: u16,
    pub spawn_dist: u16,
    pub asm_ptr: u16, // bank 0x8f
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
    pub door_list_ptr: u16,

    pub states: Vec<State>,
    pub door_list: Vec<DoorData>,
}

#[derive(Debug, Serialize)]
pub struct SuperMetroidData {
    pub room_mdb: HashMap<u16, RoomMdb>,
    pub level_data: HashMap<u32, RoomData>,
}

fn load_room_mdb_header(data: &[u8]) -> Result<RoomMdb, Error> {
    let mut r = Cursor::new(data);
    Ok(RoomMdb {
        index: r.read_u8()?,
        area: Area::from_u8(r.read_u8()?).ok_or(format_err!("unknown area type."))?,
        x: r.read_u8()?,
        y: r.read_u8()?,
        width: r.read_u8()?,
        height: r.read_u8()?,
        up_scroller: r.read_u8()?,
        down_scroller: r.read_u8()?,
        graphics_flags: r.read_u8()?,
        door_list_ptr: r.read_u16::<LittleEndian>()?,
        states: Vec::new(),
        door_list: Vec::new(),
    })
}

// State conditions are 2 bytes codes followed by 0, 1, or 2 bytes of parameter data.
fn load_state_condition(data: &[u8], offset: usize) -> Result<(StateCondition, usize), Error> {
    let mut r = Cursor::new(&data[offset..]);
    let condition_value_raw = r.read_u16::<LittleEndian>()?;
    let condition_value = StateConditionValue::from_u16(condition_value_raw).ok_or(format_err!(
        "unknown state condition {:04x}",
        condition_value_raw
    ))?;

    let condition = match condition_value {
        StateConditionValue::Default => StateCondition::Default,
        StateConditionValue::DoorPointerIs => StateCondition::DoorPointerIs {
            value: r.read_u16::<LittleEndian>()?,
        },
        StateConditionValue::MainAreaBossDead => StateCondition::MainAreaBossDead,
        StateConditionValue::EventSet => {
            let b = r.read_u8()?;
            StateCondition::EventSet {
                event: Event::from_u8(b).ok_or(format_err!("unknown event {:02x}", b))?,
            }
        }
        StateConditionValue::AreaBossesDead => StateCondition::AreaBossesDead {
            bosses: r.read_u8()?,
        },
        StateConditionValue::HasMorphBall => StateCondition::HasMorphBall,
        StateConditionValue::HasMorphBallAndMissiles => StateCondition::HasMorphBallAndMissiles,
        StateConditionValue::HasPowerBombs => StateCondition::HasPowerBombs,
        StateConditionValue::HasSpeedBooster => StateCondition::HasSpeedBooster,
    };

    Ok((condition, offset + r.position() as usize))
}

fn load_state_data(data: &[u8]) -> Result<StateData, Error> {
    let mut r = Cursor::new(data);
    Ok(StateData {
        level_data: r.read_u24::<LittleEndian>()?,
        tile_set: r.read_u8()?,
        music_data_index: r.read_u8()?,
        music_track: r.read_u8()?,
        fx_ptr: r.read_u16::<LittleEndian>()?,
        enemy_population: r.read_u16::<LittleEndian>()?,
        enemy_set: r.read_u16::<LittleEndian>()?,
        layer_2_scroll_x: r.read_u8()?,
        layer_2_scroll_y: r.read_u8()?,
        scroll_ptr: r.read_u16::<LittleEndian>()?,
        x_ray_block_ptr: r.read_u16::<LittleEndian>()?,
        main_asm_ptr: r.read_u16::<LittleEndian>()?,
        plm_ptr: r.read_u16::<LittleEndian>()?,
        bg_ptr: r.read_u16::<LittleEndian>()?,
        setup_asm_ptr: r.read_u16::<LittleEndian>()?,
    })
}

fn load_states(rom_data: &[u8], state_offset: usize, states: &mut Vec<State>) -> Result<(), Error> {
    // Create a mutable shadow so we can increment state_offset in this function's scope.
    let mut state_offset = state_offset;
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

        states.push(State {
            condition: condition,
            data: load_state_data(&rom_data[rom_addr!(0x8f, data_ptr)..])?,
        });
        if done {
            break;
        }
    }

    Ok(())
}

fn load_room_mdb(rom_data: &[u8], offset: usize) -> Result<RoomMdb, Error> {
    let mut mdb = load_room_mdb_header(&rom_data[offset..])?;
    load_states(rom_data, offset + 0xb, &mut mdb.states)?;

    Ok(mdb)
}

fn load_block_info(r: &mut dyn Read) -> Result<BlockInfo, Error> {
    let data = r.read_u16::<LittleEndian>()?;

    Ok(BlockInfo {
        ty: BlockType::from_u8((data >> 12) as u8 & 0xf)
            .ok_or(format_err!("unknown block type"))?,
        x_flip: is_bit_set!(data, 0x400),
        y_flip: is_bit_set!(data, 0x800),
        tile_index: data & 0x3ff,
    })
}

fn load_room_data(data: &[u8]) -> Result<RoomData, Error> {
    let data_len = data.len() - 2;
    let mut r = Cursor::new(data);

    // The first word gives the size of layer 1 block data.  Since block
    // info is 2 bytes, we divide by two to get the total number of blocks.
    let num_blocks = r.read_u16::<LittleEndian>()? as usize / 2;

    // There will either be 1 layer (2 bytes per block) and bts (1 byte per
    // block or 2 layers and bts.
    if (data_len != num_blocks * 3) && (data_len != num_blocks * 5) {
        return Err(format_err!("wrong sized room data"));
    }

    let has_layer2 = data_len == num_blocks * 5;

    let mut layer_1 = Vec::new();
    for _ in 0..num_blocks {
        layer_1.push(load_block_info(&mut r)?);
    }

    let mut max_door_index = 0;
    let mut bts = Vec::new();
    for i in 0..num_blocks {
        // Reading a byte at a time is not so efficient....
        let b = r.read_u8()?;
        bts.push(b);

        // Keep track of maximum door index.
        if layer_1[i].ty == BlockType::DoorBlock {
            max_door_index = cmp::max(max_door_index, b as usize);
        }
    }

    let layer_2 = if has_layer2 {
        let mut layer_2 = Vec::new();
        for _ in 0..num_blocks {
            layer_2.push(load_block_info(&mut r)?);
        }

        Some(layer_2)
    } else {
        None
    };

    Ok(RoomData {
        layer_1: layer_1,
        bts: bts,
        layer_2: layer_2,
        num_doors: max_door_index + 1,
    })
}

fn load_door_data(data: &[u8]) -> Result<DoorData, Error> {
    let mut r = Cursor::new(data);
    let dest_room_ptr = r.read_u16::<LittleEndian>()?;
    let elevator_props = r.read_u8()?;
    let orientation = r.read_u8()?;
    let x0 = r.read_u8()?;
    let y0 = r.read_u8()?;
    let x1 = r.read_u8()?;
    let y1 = r.read_u8()?;
    let spawn_dist = r.read_u16::<LittleEndian>()?;
    let asm_ptr = r.read_u16::<LittleEndian>()?;

    Ok(DoorData {
        dest_room_ptr: dest_room_ptr,
        elevator_props: elevator_props,
        orientation: orientation,
        x: x0 as u16 + ((x1 as u16) << 8),
        y: y0 as u16 + ((y1 as u16) << 8),
        spawn_dist: spawn_dist,
        asm_ptr: asm_ptr,
    })
}

pub fn load(rom_data: &[u8]) -> Result<SuperMetroidData, Error> {
    if rom_data.len() != 0x300000 {
        return Err(format_err!("Rom data is wrong size."));
    }

    // TODO: verify checksum/crc/other hash.
    let mut rooms_to_check: HashSet<u16> = HashSet::new();
    let mut room_mdb = HashMap::new();
    let mut level_data_db: HashMap<u32, RoomData> = HashMap::new();

    rooms_to_check.insert((rommap::ROOM_MDB_START & 0x7fff) as u16 + 0x8000);
    while !rooms_to_check.is_empty() {
        let room_ptr = *(rooms_to_check.iter().next().unwrap());
        let rom_ptr = rom_addr!(0x8f, room_ptr);
        let mut mdb = load_room_mdb(rom_data, rom_ptr)?;

        // Load level data and calculate number of doors.
        let mut num_doors = 0;
        for state in &mdb.states {
            let level_data_ptr = state.data.level_data;
            let level_num_doors = if level_data_db.contains_key(&level_data_ptr) {
                let room_data = level_data_db.get(&level_data_ptr).unwrap();
                room_data.num_doors
            } else {
                let level_data =
                    compression::decompress(&rom_data[snes_to_rom_addr!(level_data_ptr)..])?;
                let room_data = load_room_data(&level_data)?;
                let num = room_data.num_doors;
                level_data_db.insert(level_data_ptr, room_data);
                num
            };
            num_doors = cmp::max(num_doors, level_num_doors);
        }

        // load door list.
        let mut r = Cursor::new(&rom_data[rom_addr!(0x8f, mdb.door_list_ptr)..]);
        for _ in 0..num_doors {
            let door_data_ptr = r.read_u16::<LittleEndian>()?;
            let door_data = load_door_data(&rom_data[rom_addr!(0x83, door_data_ptr)..])?;
            let dest_room_ptr = door_data.dest_room_ptr;
            if dest_room_ptr == 0 {
                continue;
            }
            mdb.door_list.push(door_data);
            if !room_mdb.contains_key(&dest_room_ptr) {
                rooms_to_check.insert(dest_room_ptr);
            }
        }

        rooms_to_check.remove(&room_ptr);
        room_mdb.insert(room_ptr, mdb);
    }

    Ok(SuperMetroidData {
        room_mdb: room_mdb,
        level_data: level_data_db,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
