use dot;
use failure::Error;
use std::borrow::Cow;
use std::fs::File;
use std::io::prelude::*;

use super_metroid;

type Nd = u16;
type Ed = (u16, u16);
struct Edges(Vec<Ed>);

impl<'a> dot::Labeller<'a, Nd, Ed> for Edges {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("SuperMetroid").unwrap()
    }

    fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
        dot::Id::new(format!("room_{:04x}", *n)).unwrap()
    }
}

impl<'a> dot::GraphWalk<'a, Nd, Ed> for Edges {
    fn nodes(&self) -> dot::Nodes<'a, Nd> {
        // (assumes that |N| \approxeq |E|)
        let &Edges(ref v) = self;
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
        let &Edges(ref edges) = self;
        Cow::Borrowed(&edges[..])
    }

    fn source(&self, e: &Ed) -> Nd {
        e.0
    }

    fn target(&self, e: &Ed) -> Nd {
        e.1
    }
}

fn main() -> Result<(), Error> {
    let mut f = File::open("SuperMetroid.F8DF.sfc")?;
    let mut buffer = Vec::new();
    // read the whole file
    f.read_to_end(&mut buffer)?;

    let sm = super_metroid::load(&buffer)?;

    let mut rooms: Vec<u16> = sm.room_mdb.keys().cloned().collect();
    rooms.sort();

    // Build up edges graph for dot.
    let mut edges = Edges(Vec::new());
    for (addr, room) in &sm.room_mdb {
        for door in &room.door_list {
            edges.0.push((*addr, door.dest_room_ptr));
        }
    }

    let mut f = File::create("rooms.dot")?;
    dot::render(&edges, &mut f)?;
    Ok(())
}
