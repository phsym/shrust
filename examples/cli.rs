extern crate shrust;
use shrust::Shell;

use std::collections::HashMap;
use std::str::FromStr;

fn main() {
    let map = HashMap::new();
    let mut reg = Shell::new(map);

    reg.new_command("put", "Insert a value", 2, |shell, args| {
        shell.insert(try!(usize::from_str(args[0])), args[1].to_string());
        Ok(())
    });
    reg.new_command("get", "Get a value", 1, |shell, args| {
        match shell.get(&try!(usize::from_str(args[0]))) {
            Some(val) => println!("{}", val),
            None => println!("Not found")
        };
        Ok(())
    });
    reg.new_command("remove", "Remove a value", 1, |shell, args| {
        shell.remove(&try!(usize::from_str(args[0])));
        Ok(())
    });
    reg.new_command("list", "List all values", 0, |shell, _| {
        for (k, v) in shell.data() {
            println!("{} = {}", k, v);
        }
        Ok(())
    });
    reg.new_command_noargs("clear", "Clear all values", |shell| {
        shell.clear();
        Ok(())
    });

    reg.run_loop();
}
