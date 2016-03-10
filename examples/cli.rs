extern crate shrust;
use shrust::CommandRegistry;

use std::collections::HashMap;

fn main() {
    let map = HashMap::new();
    let mut reg = CommandRegistry::new(map);

    reg.new_command("put", 2, |map, args| {
        map.insert(args[0].to_string(), args[1].to_string());
    });
    reg.new_command("get", 1, |map, args| {
        match map.get(args[0]) {
            Some(val) => println!("{}", val),
            None => println!("Not found")
        };
    });
    reg.new_command("list", 0, |map, _| {
        for (k, v) in map {
            println!("{} = {}", k, v);
        }
    });

    reg.run_loop();
}
