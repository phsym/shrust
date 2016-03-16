extern crate shrust;
use shrust::Shell;

use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    let map = HashMap::new();
    let mut shell = Shell::new(map);

    shell.new_command("put", "Insert a value", 2, |map, args| {
        map.insert(try!(usize::from_str(args[0])), args[1].to_string());
        Ok(())
    });
    shell.new_command("get", "Get a value", 1, |map, args| {
        match map.get(&try!(usize::from_str(args[0]))) {
            Some(val) => println!("{}", val),
            None => println!("Not found")
        };
        Ok(())
    });
    shell.new_command("remove", "Remove a value", 1, |map, args| {
        map.remove(&try!(usize::from_str(args[0])));
        Ok(())
    });
    shell.new_command("list", "List all values", 0, |map, _| {
        for (k, v) in map {
            println!("{} = {}", k, v);
        }
        Ok(())
    });
    shell.new_command_noargs("clear", "Clear all values", |map| {
        map.clear();
        Ok(())
    });

    shell.run_loop();
}
