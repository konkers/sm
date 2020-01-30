use failure::Error;
use ron;
use std::fs::File;
use std::io::prelude::*;

use super_metroid;

fn main() -> Result<(), Error> {
    let mut f = File::open("SuperMetroid.F8DF.sfc")?;
    let mut buffer = Vec::new();
    // read the whole file
    f.read_to_end(&mut buffer)?;

    let sm = super_metroid::load(&buffer)?;

    println!("{}", ron::ser::to_string_pretty(&sm, Default::default())?);

    Ok(())
}
