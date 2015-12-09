use std::io::prelude::*;
use std::fs::File;

const C: f64 = 1.7;

fn main() {
    let mut lut = Vec::new();
    for i in 0.. {
        let value = (0.5 + ((i as f64) * (2.0f64.ln()) / C).exp()).ceil();
        if value > (std::usize::MAX as f64) { break ; }
        lut.push(value as usize);
    }
    match File::open("src/lut.rs") {
        Ok(mut inf) => {
            let new_first_line =
                format!("const MAXDEPTH: usize = {};" , lut . len (  ));
            let mut contents = String::new();
            inf.read_to_string(&mut contents).unwrap();
            contents.truncate(new_first_line.len());
            if contents == new_first_line { return; };
        }
        _ => (),
    }


    let mut f = File::create("src/lut.rs").unwrap();

    writeln!(f , "const MAXDEPTH: usize = {};" , lut . len (  )).unwrap();
    f.write_all(b"const MINWEIGHT: [usize; MAXDEPTH] = [").unwrap();
    for i in &lut { write!(f , "{}, " , i).unwrap(); }
    writeln!(f , "];").unwrap();
}

