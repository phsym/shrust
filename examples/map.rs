extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    let map = HashMap::new();
    let mut shell = Shell::new(map);

    shell.new_command("put", "Insert a value", 2, |_, map, args| {
        map.insert(usize::from_str(args[0])?, args[1].to_string());
        Ok(())
    });
    shell.new_command("get", "Get a value", 1, |io, map, args| {
        match map.get(&usize::from_str(args[0])?) {
            Some(val) => writeln!(io, "{}", val).unwrap(),
            None => writeln!(io, "Not found").unwrap()
        };
        Ok(())
    });
    shell.new_command("remove", "Remove a value", 1, |_, map, args| {
        map.remove(&usize::from_str(args[0])?);
        Ok(())
    });
    shell.new_command("list", "List all values", 0, |io, map, _| {
        for (k, v) in map {
            writeln!(io, "{} = {}", k, v).unwrap();
        }
        Ok(())
    });
    shell.new_command_noargs("clear", "Clear all values", |_, map| {
        map.clear();
        Ok(())
    });

    shell.run_loop(&mut ShellIO::default());
}
