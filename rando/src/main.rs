use dot;
use failure::Error;
use num::FromPrimitive;
use parse_int::parse;
use serde_json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use super_metroid;

mod smjsondata;

struct RoomPlm {
    states: Vec<usize>,
    plm: super_metroid::PlmPopulation,
}

type Nd = u16;
type Ed = (u16, u16);
struct Edges {
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
            "{}<br />{}<br />addr: {:04x}<br />{}",
            name, region, n, plms
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

fn main() -> Result<(), Error> {
    let mut f = File::open("SuperMetroid.F8DF.sfc")?;
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

    // Build up edges graph for dot.
    let mut edges = Edges {
        edges: Vec::new(),
        room_names: room_names,
        room_regions: room_regions,
        room_plms: room_plms,
    };
    for (addr, room) in &sm.room_mdb {
        for door in &room.door_list {
            edges.edges.push((*addr, door.dest_room_ptr));
        }
    }

    let mut f = File::create("rooms.dot")?;
    dot::render(&edges, &mut f)?;
    Ok(())
}
