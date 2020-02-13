use clap::{App, Arg};
use std::error::Error;

mod converter;

fn main() -> Result<(), Box<dyn Error>> {
     let matches = App::new("gltf2glb")
          .version("0.1")
          .author("Andreas Streichardt <andreas@mop.koeln>")
          .about("Does one thing and one thing only: convert a gltf file to glb")
          .arg(Arg::with_name("gltf")
               .help("input gltf file")
               .required(true))
          .arg(Arg::with_name("glb").help("output glb file").required(true))
          .get_matches();
     converter::convert(
          matches.value_of("gltf").unwrap(),
          matches.value_of("glb").unwrap(),
     )?;

     Ok(())
}
