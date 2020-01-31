use failure::Error;
use std::fs::File;
use std::io::prelude::*;

use super_metroid;

fn main() -> Result<(), Error> {
    let mut f = File::open("SuperMetroid.F8DF.sfc")?;
    let mut buffer = Vec::new();
    // read the whole file
    f.read_to_end(&mut buffer)?;

    let sm = super_metroid::load(&buffer)?;

    let mut rooms: Vec<u16> = sm.room_mdb.keys().cloned().collect();
    rooms.sort();

    for addr in &rooms {
        println!("{:x}", addr);
    }

    Ok(())
}
