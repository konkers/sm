use dot;
use failure::{format_err, Error};
use num::FromPrimitive;
use parse_int::parse;
use regex::Regex;
use serde_json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use structopt::StructOpt;

use super_metroid::{
    self,
    graphics::{BYTES_PER_TILE, TILE_H, TILE_W},
    Color,
};

mod smjsondata;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(long, parse(from_os_str), default_value = "SuperMetroid.F8DF.sfc")]
    rom: PathBuf,
}

struct RoomPlm {
    states: Vec<usize>,
    plm: super_metroid::PlmPopulation,
}

type Nd = u16;
type Ed = (u16, u16);
struct Edges {
    sm: super_metroid::SuperMetroidData,
    edges: Vec<Ed>,
    room_names: HashMap<u16, String>,
    room_regions: HashMap<u16, String>,
    // [room_ptr][plm_id] -> RoomPlm
    room_plms: HashMap<u16, HashMap<(u16, u16), RoomPlm>>,
}

impl<'a> dot::Labeller<'a, Nd, Ed> for Edges {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("SuperMetroid").unwrap()
    }

    fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
        dot::Id::new(format!("room_{:04x}", *n)).unwrap()
    }

    fn node_label(&'a self, n: &Nd) -> dot::LabelText<'a> {
        let name = match self.room_names.get(n) {
            Some(name) => name.clone(),
            None => format!("room_{:04x}", *n),
        };
        let region = match self.room_regions.get(n) {
            Some(name) => name,
            None => "",
        };
        let plms = self
            .room_plms
            .get(n)
            .unwrap()
            .iter()
            .filter(|(plm_id, _)| super_metroid::PlmItemId::from_u16(plm_id.0).is_some())
            .map(|(plm_id, data)| {
                let states = data
                    .states
                    .iter()
                    .map(|state| format!("{}", state))
                    .collect::<Vec<String>>()
                    .join(", ");
                let item = super_metroid::PlmItemId::from_u16(plm_id.0).unwrap();
                format!(
                    "{:?}{}@({}, {}) states: {}",
                    item, data.plm.param, data.plm.x, data.plm.y, states
                )
            })
            .collect::<Vec<String>>()
            .join("<br />");
        dot::LabelText::html(format!(
            "{:02x}: {}<br />{}<br />addr: {:04x}<br />{}",
            self.sm.room_mdb[n].index, name, region, n, plms
        ))
    }

    fn node_color(&'a self, n: &Nd) -> Option<dot::LabelText<'a>> {
        if let Some(name) = self.room_regions.get(n) {
            if name.starts_with("brinstar") {
                return Some(dot::LabelText::label("chartreuse"));
            } else if name.starts_with("crateria") {
                return Some(dot::LabelText::label("gold"));
            } else if name.starts_with("lowernorfair") {
                return Some(dot::LabelText::label("crimson"));
            } else if name.starts_with("maridia") {
                return Some(dot::LabelText::label("deepskyblue"));
            } else if name.starts_with("norfair") {
                return Some(dot::LabelText::label("lightpink"));
            } else if name.starts_with("tourian") {
                return Some(dot::LabelText::label("hotpink"));
            } else if name.starts_with("wreckedship") {
                return Some(dot::LabelText::label("grey80"));
            }
        }

        None
    }

    fn node_style(&'a self, _n: &Nd) -> dot::Style {
        dot::Style::Filled
    }
}

impl<'a> dot::GraphWalk<'a, Nd, Ed> for Edges {
    fn nodes(&self) -> dot::Nodes<'a, Nd> {
        // (assumes that |N| \approxeq |E|)
        let ref v = self.edges;
        let mut nodes = Vec::with_capacity(v.len());
        for &(s, t) in v {
            nodes.push(s);
            nodes.push(t);
        }
        nodes.sort();
        nodes.dedup();
        Cow::Owned(nodes)
    }

    fn edges(&'a self) -> dot::Edges<'a, Ed> {
        let ref edges = self.edges;
        Cow::Borrowed(&edges[..])
    }

    fn source(&self, e: &Ed) -> Nd {
        e.0
    }

    fn target(&self, e: &Ed) -> Nd {
        e.1
    }
}

fn load_regions() -> Result<HashMap<String, smjsondata::Root>, Error> {
    let mut map = HashMap::new();
    for loc_str in &[
        "brinstar/blue",
        "brinstar/green",
        "brinstar/kraid",
        "brinstar/pink",
        "brinstar/red",
        "ceres/main",
        "crateria/central",
        "crateria/east",
        "crateria/west",
        "lowernorfair/east",
        "lowernorfair/west",
        "maridia/inner-green",
        "maridia/inner-pink",
        "maridia/inner-yellow",
        "maridia/outer",
        "norfair/crocomire",
        "norfair/east",
        "norfair/west",
        "tourian/main",
        "wreckedship/main",
    ] {
        let f = File::open(format!(
            "../third-party/sm-json-data/region/{}.json",
            loc_str
        ))?;
        let region: smjsondata::Root = serde_json::from_reader(BufReader::new(f))?;
        map.insert(String::from(*loc_str), region);
    }
    Ok(map)
}

pub fn get_pixel(data: &[u8], x: usize, y: usize) -> u8 {
    let b = data[(y * 4 + x / 2) as usize];
    if x & 0x1 == 0x1 {
        b >> 4
    } else {
        b & 0xf
    }
}

pub fn render_tile(
    tile: &[u8],
    img: &mut image::RgbaImage,
    colors: &[Color],
    x: usize,
    y: usize,
    flip_h: bool,
    flip_v: bool,
) {
    assert!(colors.len() >= 16);
    for y1 in 0..TILE_H {
        for x1 in 0..TILE_W {
            let src_x = if flip_h { 7 - x1 } else { x1 };
            let src_y = if flip_v { 7 - y1 } else { y1 };
            let val = get_pixel(tile, src_x, src_y);
            let color = &colors[val as usize];
            *img.get_pixel_mut((x + x1) as u32, (y + y1) as u32) =
                image::Rgba([color.r, color.g, color.b, if val == 0 { 0x0 } else { 0xff }]);
        }
    }
}

pub fn get_tile(index: u16, data: &[u8]) -> Result<&[u8], Error> {
    let num_tiles = data.len() / BYTES_PER_TILE;
    if index as usize >= num_tiles {
        return Err(format_err!("tile {} out of range.", index));
    }

    Ok(&data[(index as usize * BYTES_PER_TILE)..])
}
pub fn render_graphics(data: &[u8]) -> Result<image::RgbaImage, Error> {
    let num_tiles = data.len() / BYTES_PER_TILE;
    let tiles_w = 16;
    let tiles_h = num_tiles / tiles_w;
    let img_w = (tiles_w * TILE_W) as u32;
    let img_h = (tiles_h * TILE_H) as u32;

    let mut colors = Vec::new();
    for val in 0..16 {
        colors.push(Color {
            r: val << 4,
            g: val << 4,
            b: val << 4,
        });
    }

    let mut img = image::RgbaImage::new(img_w, img_h);
    for (_, _, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([0, 0, 0, 0]);
    }

    for i in 0..num_tiles {
        let tile = get_tile(i as u16, data)?;
        let x = i % tiles_w * 8;
        let y = i / tiles_w * 8;
        render_tile(tile, &mut img, &colors, x, y, false, false);
    }

    Ok(img)
}
fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let mut f = File::open(opt.rom)?;
    let mut buffer = Vec::new();
    // read the whole file
    f.read_to_end(&mut buffer)?;

    let sm = super_metroid::SuperMetroidData::new(&buffer)?;

    let mut rooms: Vec<u16> = sm.room_mdb.keys().cloned().collect();
    rooms.sort();

    let map = load_regions()?;
    let mut room_names: HashMap<u16, String> = HashMap::new();
    let mut room_regions: HashMap<u16, String> = HashMap::new();
    let mut room_plms: HashMap<u16, HashMap<(u16, u16), RoomPlm>> = HashMap::new();

    for (name, region) in &map {
        for room in &region.rooms {
            let addr = (parse::<u32>(&room.room_address)? & 0xffff) as u16;

            room_names.insert(addr, room.name.clone());
            room_regions.insert(addr, name.clone());
        }
    }

    for (addr, room) in &sm.room_mdb {
        let plms = room_plms.entry(*addr).or_insert(HashMap::new());
        for (state_idx, state) in room.states.iter().enumerate() {
            for plm in sm.plm_population.get(&state.data.plm_ptr).unwrap() {
                match plms.get_mut(&(plm.id, plm.param)) {
                    Some(ref mut data) => {
                        data.states.push(state_idx);
                    }
                    None => {
                        plms.insert(
                            (plm.id, plm.param),
                            RoomPlm {
                                states: vec![state_idx],
                                plm: plm.clone(),
                            },
                        );
                    }
                }
            }
        }
    }

    let cre_addr = super_metroid::rom_addr_to_snes!(super_metroid::rommap::CRE_TILES);
    let cre_tiles = &sm.tiles.get(&cre_addr).unwrap();

    let cre_table_addr = super_metroid::rom_addr_to_snes!(super_metroid::rommap::CRE_TILE_TABLE);
    let cre_table = &sm.tile_tables.get(&cre_table_addr).unwrap();

    let mut renderers = Vec::new();
    for (i, set) in sm.tile_sets.iter().enumerate() {
        let r = super_metroid::graphics::TileRenderer::new(
            cre_tiles,
            sm.tiles.get(&set.tiles_ptr).unwrap(),
            sm.palettes.get(&set.palette_ptr).unwrap(),
            cre_table,
            sm.tile_tables.get(&set.tile_table_ptr).unwrap(),
        )?;
        let img = r.render_graphics_sheet()?;
        img.save(format!("tileset/{:02x}_graphics_sheet.png", i))
            .unwrap();
        let img = r.render_palette()?;
        img.save(format!("tileset/{:02x}_pallete.png", i)).unwrap();
        let img = r.render_tile_table()?;
        img.save(format!("tileset/{:02x}_tile_table.png", i))
            .unwrap();
        renderers.push(r);
    }

    let tmp_addr = super_metroid::rom_addr!(0xad, 0xb600);
    let mut tile_data = buffer[tmp_addr..(tmp_addr + 0x1000)].to_owned();
    super_metroid::graphics::de_planar_tiles(&mut tile_data);
    let img = render_graphics(&tile_data)?;
    img.save(format!("test.png")).unwrap();

    let clean_file_re = Regex::new(r"[\./\\ ]").unwrap();
    for (addr, room) in &sm.room_mdb {
        for (i, state) in room.states.iter().enumerate() {
            let renderer = renderers.get(state.data.tile_set as usize).unwrap();
            let room_data = &sm.level_data.get(&state.data.level_data).unwrap();
            let img = renderer.render_room(i, room, room_data)?;

            let room_name = match room_names.get(&addr) {
                Some(n) => format!("_{}", clean_file_re.replace_all(n, "_")),
                None => "".to_string(),
            };
            img.save(format!("room/{:04x}_{}{}.png", addr, i, room_name))
                .unwrap();
        }
    }

    let map: std::collections::BTreeMap<_, _> = sm.enemies.iter().collect();
    for (addr, enemy) in &map {
        println!(
            "{:04x}: {:04x} {}x{} {:06x} '{}'",
            addr,
            &enemy.data.tile_data_size,
            &enemy.data.width,
            &enemy.data.height,
            &enemy.data.tile_data_ptr,
            &enemy.name
        );
    }

    // Build up edges graph for dot.
    let mut edges = Edges {
        sm: sm,
        edges: Vec::new(),
        room_names: room_names,
        room_regions: room_regions,
        room_plms: room_plms,
    };
    for (addr, room) in &edges.sm.room_mdb {
        for door in &room.door_list {
            edges.edges.push((*addr, door.dest_room_ptr));
        }
    }

    let mut f = File::create("rooms.dot")?;
    dot::render(&edges, &mut f)?;
    Ok(())
}
