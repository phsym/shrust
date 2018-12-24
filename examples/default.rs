extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

fn main() {
    let mut shell = Shell::new(());
    shell.set_default(|io, _, cmd| {
        r#try!(writeln!(io, "Hello from default handler !!! Received: {}", cmd));
        Ok(())
    });
    shell.run_loop(&mut ShellIO::default());
}
