pub mod compression;
pub mod graphics;
pub mod rommap;
mod util;

use byteorder::{LittleEndian, ReadBytesExt};
use failure::{format_err, Error};
use num::FromPrimitive;
use num_derive::FromPrimitive;
use serde::Serialize;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

use graphics::de_planar_tiles;

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

#[derive(Clone, Copy, Debug, FromPrimitive, Serialize)]
#[repr(u8)]
pub enum TileSet {
    UpperCrateria = 0x00,
    RedCrateria = 0x01,
    LowerCrateria = 0x02,
    OldTourian = 0x03,
    WreckedShipOff = 0x04,
    WreckedShipOn = 0x05,
    GreenBlueBrinstart = 0x06,
    RedBrinstar = 0x07,
    PreTourian = 0x08,
    HeatedNorfair = 0x09,
    UnheatedNorfiar = 0x0a,
    SandlessMaridia = 0x0b,
    SandyMaridia = 0x0c,
    Tourian = 0x0d,
    MotherBrainRoom = 0x0e,
    BlueCeres = 0x0f,
    WhiteCeres = 0x10,
    BlueCeresElevator = 0x11,
    WhiteCeresElevator = 0x12,
    BlueCeresRidley = 0x13,
    WhiteCeresRidley = 0x14,
    MapRoom = 0x15,
    WreckedShipMapRoomOff = 0x16,
    BlueRefillRoom = 0x17,
    YellowRefillRoom = 0x18,
    SaveRoom = 0x19,
    KraidRoom = 0x1a,
    CrocomireRoom = 0x1b,
    DraygonRoom = 0x1c,
}

#[derive(Debug, Serialize)]
pub struct StateData {
    pub level_data: u32,
    pub tile_set: TileSet,
    pub music_data_index: u8,
    pub music_track: u8,
    pub fx_ptr: u16,           // Bank $83
    pub enemy_population: u16, // bank $a1
    pub enemy_set: u16,        // bank $b4
    pub layer_2_scroll_x: u8,
    pub layer_2_scroll_y: u8,
    pub scroll_ptr: u16,      // bank $8f?
    pub x_ray_block_ptr: u16, // bank ??
    pub main_asm_ptr: u16,    // bank ??
    pub plm_ptr: u16,         // bank ??
    pub bg_ptr: u16,          // bank ??
    pub setup_asm_ptr: u16,   // bank ??
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

#[derive(Clone, Debug, Serialize)]
pub struct PlmPopulation {
    pub id: u16,
    pub x: u8,
    pub y: u8,
    pub param: u16,
}

#[derive(Debug, FromPrimitive, Serialize)]
#[repr(u16)]
pub enum PlmItemId {
    ETank = 0xeed7,
    Missile = 0xeedb,
    SuperMissile = 0xeedf,
    PowerBomb = 0xeee3,
    Bomb = 0xeee7,
    Charge = 0xeeeb,
    Ice = 0xeeef,
    HiJump = 0xeef3,
    SpeedBooster = 0xeef7,
    Wave = 0xeefb,
    Spazer = 0xeeff,
    SpringBall = 0xef03,
    Varia = 0xef07,
    Gravity = 0xef0b,
    Plasma = 0xef13,
    XRayScope = 0xef0f,
    Grapple = 0xef17,
    SpaceJump = 0xef1b,
    ScrewAttack = 0xef1f,
    Morph = 0xef23,
    Reserve = 0xef27,

    ETankChozo = 0xef2b,
    MissileChozo = 0xef2f,
    SuperMissileChozo = 0xef33,
    PowerBombChozo = 0xef37,
    BombChozo = 0xef3b,
    ChargeChozo = 0xef3f,
    HiJumpChozo = 0xef47,
    SpeedBoosterChozo = 0xef4b,
    IceChozo = 0xef4e,
    WaveChozo = 0xef4f,
    SpazerChozo = 0xef53,
    SpringBallChozo = 0xef57,
    VariaChozo = 0xef5b,
    GravityChozo = 0xef5f,
    XRayScopeChozo = 0xef63,
    PlasmaChozo = 0xef67,
    GrappleChozo = 0xef6b,
    SpaceJumpChozo = 0xef6f,
    ScrewAttackChozo = 0xef73,
    MorphChozo = 0xef77,
    ReserveChozo = 0xef7b,

    ETankHidden = 0xef7f,
    MissileHidden = 0xef83,
    SuperMissileHidden = 0xef87,
    PowerBombHidden = 0xef8b,
    BombHidden = 0xef8f,
    ChargeHidden = 0xef93,
    IceHidden = 0xef97,
    HiJumpHidden = 0xef9b,
    SpeedBoosterHidden = 0xef9f,
    WaveHidden = 0xefa3,
    SpazerHidden = 0xefa7,
    SpringBallHidden = 0xefab,
    VariaHidden = 0xefaf,
    GravityHidden = 0xefb3,
    XRayScopeHidden = 0xefb7,
    PlasmaHidden = 0xefbb,
    GrappleHidden = 0xefbf,
    SpaceJumpHidden = 0xefc3,
    ScrewAttackHidden = 0xefc7,
    MorphHidden = 0xefcb,
    ReserveHidden = 0xefcf,
}

#[derive(Clone, Debug, Serialize)]
pub struct TileSetEntry {
    pub tile_table_ptr: u32,
    pub tiles_ptr: u32,
    pub palette_ptr: u32,
}

#[derive(Debug, Serialize)]
pub struct Tiles {
    // Data is de-planarized and stored as 4bpp tiled.
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct TileTableEntry {
    index: u16,
    palette: u8,
    priority: bool,
    flip_h: bool,
    flip_v: bool,
}

#[derive(Debug, Serialize)]
pub struct TileTable {
    pub entries: Vec<TileTableEntry>,
}

#[derive(Debug, Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const PALETTE_ENTRIES: usize = 16 * 8;

#[derive(Debug, Serialize)]
pub struct Palette {
    colors: Vec<Color>,
}

#[derive(Debug, Serialize)]
pub struct SuperMetroidData {
    pub room_mdb: HashMap<u16, RoomMdb>,
    pub level_data: HashMap<u32, RoomData>,
    pub plm_population: HashMap<u16, Vec<PlmPopulation>>,
    pub tile_sets: Vec<TileSetEntry>,
    pub tiles: HashMap<u32, Tiles>,
    pub tile_tables: HashMap<u32, TileTable>,
    pub palettes: HashMap<u32, Palette>,
}

struct Loader<'a> {
    rom_data: &'a [u8],
    rooms_to_check: HashSet<u16>,
    sm: SuperMetroidData,
}

impl<'a> Loader<'a> {
    pub fn new(rom_data: &'a [u8]) -> Loader {
        let mut loader = Loader {
            rom_data: rom_data,
            rooms_to_check: HashSet::new(),
            sm: SuperMetroidData {
                room_mdb: HashMap::new(),
                level_data: HashMap::new(),
                plm_population: HashMap::new(),
                tile_sets: Vec::new(),
                tiles: HashMap::new(),
                tile_tables: HashMap::new(),
                palettes: HashMap::new(),
            },
        };
        loader
            .rooms_to_check
            .insert((rommap::ROOM_MDB_START & 0x7fff) as u16 + 0x8000);
        loader
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
    fn load_state_condition(r: &mut dyn Read) -> Result<StateCondition, Error> {
        let condition_value_raw = r.read_u16::<LittleEndian>()?;
        let condition_value = StateConditionValue::from_u16(condition_value_raw).ok_or(
            format_err!("unknown state condition {:04x}", condition_value_raw),
        )?;

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

        Ok(condition)
    }

    fn load_state_data(data: &[u8]) -> Result<StateData, Error> {
        let mut r = Cursor::new(data);
        Ok(StateData {
            level_data: r.read_u24::<LittleEndian>()?,
            tile_set: TileSet::from_u8(r.read_u8()?).ok_or(format_err!("unknown tile set"))?,
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

    fn load_states(self: &Self, state_offset: usize, states: &mut Vec<State>) -> Result<(), Error> {
        // Create a mutable shadow so we can increment state_offset in this function's scope.
        let mut r = Cursor::new(&self.rom_data[state_offset..]);
        loop {
            let condition = Self::load_state_condition(&mut r)?;

            let (data_ptr, done) = match condition {
                // The default condition's state data immediately follows the end of the
                // end of the state condition list.  It also signifies the end of that
                // list.
                StateCondition::Default => (
                    rom_addr_to_snes16!(state_offset + r.position() as usize),
                    true,
                ),

                // For all other conditions, the state data is pointed to by the next u16.
                _ => (r.read_u16::<LittleEndian>()?, false),
            };

            states.push(State {
                condition: condition,
                data: Self::load_state_data(&self.rom_data[rom_addr!(0x8f, data_ptr)..])?,
            });
            if done {
                break;
            }
        }

        Ok(())
    }

    fn load_room_mdb(self: &mut Self, offset: usize) -> Result<RoomMdb, Error> {
        let mut mdb = Self::load_room_mdb_header(&self.rom_data[offset..])?;
        self.load_states(offset + 0xb, &mut mdb.states)?;

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
            layer_1.push(Self::load_block_info(&mut r)?);
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
                layer_2.push(Self::load_block_info(&mut r)?);
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

    fn get_or_load_level_data(self: &mut Self, level_data_ptr: u32) -> Result<&RoomData, Error> {
        Ok(self.sm.level_data.entry(level_data_ptr).or_insert({
            let level_data =
                compression::decompress(&self.rom_data[snes_to_rom_addr!(level_data_ptr)..])?;
            Self::load_room_data(&level_data)?
        }))
    }

    fn get_or_load_plm_list(self: &mut Self, plm_ptr: u16) -> Result<&Vec<PlmPopulation>, Error> {
        Ok(self.sm.plm_population.entry(plm_ptr).or_insert({
            let mut r = Cursor::new(&self.rom_data[rom_addr!(0x8f, plm_ptr)..]);
            let mut plms = Vec::new();
            loop {
                let id = r.read_u16::<LittleEndian>()?;
                if id == 0x0000 {
                    break;
                }
                plms.push(PlmPopulation {
                    id: id,
                    x: r.read_u8()?,
                    y: r.read_u8()?,
                    param: r.read_u16::<LittleEndian>()?,
                });
            }
            plms
        }))
    }

    fn load_level_data(self: &mut Self, mdb: &RoomMdb) -> Result<usize, Error> {
        // Load level data and calculate number of doors.
        let mut num_doors = 0;
        for state in &mdb.states {
            let level_data_ptr = state.data.level_data;
            let level_data = self.get_or_load_level_data(level_data_ptr)?;
            num_doors = cmp::max(num_doors, level_data.num_doors);
            self.get_or_load_plm_list(state.data.plm_ptr)?;
        }
        Ok(num_doors)
    }

    fn load_door_list(self: &mut Self, mdb: &mut RoomMdb, num_doors: usize) -> Result<(), Error> {
        // load door list.
        let mut r = Cursor::new(&self.rom_data[rom_addr!(0x8f, mdb.door_list_ptr)..]);
        for _ in 0..num_doors {
            let door_data_ptr = r.read_u16::<LittleEndian>()?;
            let door_data = Self::load_door_data(&self.rom_data[rom_addr!(0x83, door_data_ptr)..])?;
            let dest_room_ptr = door_data.dest_room_ptr;
            if dest_room_ptr == 0 {
                continue;
            }
            mdb.door_list.push(door_data);
            if !self.sm.room_mdb.contains_key(&dest_room_ptr) {
                self.rooms_to_check.insert(dest_room_ptr);
            }
        }

        Ok(())
    }

    fn load_tileset_table(self: &mut Self) -> Result<(), Error> {
        let mut ptr_table_r = Cursor::new(&self.rom_data[rommap::TILESET_POINTER_TABLE..]);
        for _ in 0..rommap::TILESET_POINTER_TABLE_COUNT {
            let ptr = ptr_table_r.read_u16::<LittleEndian>()?;
            let entry_addr = rom_addr!(rommap::TILESET_ENTRY_BANK, ptr);
            let mut entry_r = Cursor::new(&self.rom_data[entry_addr..]);
            self.sm.tile_sets.push(TileSetEntry {
                tile_table_ptr: entry_r.read_u24::<LittleEndian>()?,
                tiles_ptr: entry_r.read_u24::<LittleEndian>()?,
                palette_ptr: entry_r.read_u24::<LittleEndian>()?,
            });
        }
        Ok(())
    }

    fn load_tiles(self: &mut Self, addr: u32) -> Result<(), Error> {
        if self.sm.tiles.contains_key(&addr) {
            return Ok(());
        }

        let rom_addr = snes_to_rom_addr!(addr);
        let mut data = compression::decompress(&self.rom_data[(rom_addr as usize)..])?;

        de_planar_tiles(&mut data);
        self.sm.tiles.insert(addr, Tiles { data: data });

        Ok(())
    }

    fn load_tile_table(self: &mut Self, addr: u32) -> Result<(), Error> {
        if self.sm.tile_tables.contains_key(&addr) {
            return Ok(());
        }

        let rom_addr = snes_to_rom_addr!(addr);
        let data = compression::decompress(&self.rom_data[(rom_addr as usize)..])?;

        let num_entries = data.len() / 2;
        let mut entries = Vec::with_capacity(num_entries);
        let mut r = Cursor::new(data);
        for _ in 0..num_entries {
            let v = r.read_u16::<LittleEndian>()?;
            entries.push(TileTableEntry {
                index: v & 0x3ff,
                palette: ((v >> 10) & 0x7) as u8,
                priority: (v & (1 << 13)) != 0,
                flip_h: (v & (1 << 14)) != 0,
                flip_v: (v & (1 << 15)) != 0,
            });
        }

        self.sm
            .tile_tables
            .insert(addr, TileTable { entries: entries });

        Ok(())
    }

    fn load_palette(self: &mut Self, addr: u32) -> Result<(), Error> {
        if self.sm.palettes.contains_key(&addr) {
            return Ok(());
        }

        let rom_addr = snes_to_rom_addr!(addr);
        let data = compression::decompress(&self.rom_data[(rom_addr as usize)..])?;
        let mut r = Cursor::new(&data);
        let mut colors = Vec::with_capacity(PALETTE_ENTRIES);
        for _ in 0..PALETTE_ENTRIES {
            let val = r.read_u16::<LittleEndian>()?;
            colors.push(Color {
                r: (((val >> 0) & 0x1f) << 3) as u8,
                g: (((val >> 5) & 0x1f) << 3) as u8,
                b: (((val >> 10) & 0x1f) << 3) as u8,
            });
        }
        self.sm.palettes.insert(addr, Palette { colors: colors });
        Ok(())
    }

    pub fn load(mut self: Self) -> Result<SuperMetroidData, Error> {
        while !self.rooms_to_check.is_empty() {
            let room_ptr = *(self.rooms_to_check.iter().next().unwrap());
            let rom_ptr = rom_addr!(0x8f, room_ptr);
            let mut mdb = self.load_room_mdb(rom_ptr)?;

            let num_doors = self.load_level_data(&mut mdb)?;
            self.load_door_list(&mut mdb, num_doors)?;

            self.rooms_to_check.remove(&room_ptr);
            self.sm.room_mdb.insert(room_ptr, mdb);
        }

        self.load_tileset_table()?;

        // Copy the tile sets so we can modify sm while iterating.
        let tile_sets = self.sm.tile_sets.clone();
        for entry in tile_sets {
            self.load_tiles(entry.tiles_ptr)?;
            self.load_tile_table(entry.tile_table_ptr)?;
            self.load_palette(entry.palette_ptr)?;
        }
        self.load_tiles(rom_addr_to_snes!(rommap::CRE_TILES))?;
        self.load_tile_table(rom_addr_to_snes!(rommap::CRE_TILE_TABLE))?;

        Ok(self.sm)
    }
}

impl SuperMetroidData {
    pub fn new(rom_data: &[u8]) -> Result<SuperMetroidData, Error> {
        if rom_data.len() != 0x300000 {
            return Err(format_err!("Rom data is wrong size."));
        }

        // TODO: verify checksum/crc/other hash.
        let loader = Loader::new(rom_data);

        Ok(loader.load()?)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
