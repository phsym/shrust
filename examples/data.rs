extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

fn main() {
    let v = Vec::new();
    let mut shell = Shell::new(v);
    shell.new_command("push", "Add string to the list", 1, |io, v, s| {
        r#try!(writeln!(io, "Pushing {}", s[0]));
        v.push(s[0].to_string());
        Ok(())
    });
    shell.new_command_noargs("list", "List strings", |io, v| {
        for s in v {
            r#try!(writeln!(io, "{}", s));
        }
        Ok(())
    });

    shell.run_loop(&mut ShellIO::default());
}
