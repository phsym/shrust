extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

fn main() {
    let mut shell = Shell::new(());
    shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
        r#try!(writeln!(io, "Hello World !!!"));
        Ok(())
    });

    shell.run_loop(&mut ShellIO::default());
}
