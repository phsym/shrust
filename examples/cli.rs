extern crate shrust;
use shrust::Shell;

use std::collections::HashMap;

fn main() {
    let map = HashMap::new();
    let mut reg = Shell::new(map);

    reg.new_command("put", "Insert a value", 2, |map, args| {
        map.insert(args[0].to_string(), args[1].to_string());
    });
    reg.new_command("get", "Get a value", 1, |map, args| {
        match map.get(args[0]) {
            Some(val) => println!("{}", val),
            None => println!("Not found")
        };
    });
    reg.new_command("list", "List all values", 0, |map, _| {
        for (k, v) in map {
            println!("{} = {}", k, v);
        }
    });

    reg.run_loop();
}
