extern crate shrust;
use shrust::Shell;

use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    let map = HashMap::new();
    let mut reg = Shell::new(map);

    reg.new_command("put", "Insert a value", 2, |map, args| {
        map.insert(try!(usize::from_str(args[0])), args[1].to_string());
        Ok(())
    });
    reg.new_command("get", "Get a value", 1, |map, args| {
        match map.get(&try!(usize::from_str(args[0]))) {
            Some(val) => println!("{}", val),
            None => println!("Not found")
        };
        Ok(())
    });
    reg.new_command("list", "List all values", 0, |map, _| {
        for (k, v) in map {
            println!("{} = {}", k, v);
        }
        Ok(())
    });

    reg.run_loop();
}
