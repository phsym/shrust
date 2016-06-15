extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

use std::sync::{Arc, Mutex};
use std::thread;

use std::collections::HashMap;
use std::str::FromStr;

use std::net::TcpListener;

fn main() {
    let map = Arc::new(Mutex::new(HashMap::new()));

    let mut shell = Shell::new(map);

    shell.new_command("put", "Insert a value", 2, |_, map, args| {
        map.lock().unwrap().insert(try!(usize::from_str(args[0])), args[1].to_string());
        Ok(())
    });
    shell.new_command("get", "Get a value", 1, |mut io, map, args| {
        match map.lock().unwrap().get(&try!(usize::from_str(args[0]))) {
            Some(val) => writeln!(io, "{}", val).unwrap(),
            None => writeln!(io, "Not found").unwrap()
        };
        Ok(())
    });
    shell.new_command("remove", "Remove a value", 1, |_, map, args| {
        map.lock().unwrap().remove(&try!(usize::from_str(args[0])));
        Ok(())
    });
    shell.new_command("list", "List all values", 0, |mut io, map, _| {
        for (k, v) in &*map.lock().unwrap() {
            writeln!(io, "{} = {}", k, v).unwrap();
        }
        Ok(())
    });
    shell.new_command_noargs("clear", "Clear all values", |_, map| {
        map.lock().unwrap().clear();
        Ok(())
    });

    let serv = TcpListener::bind("0.0.0.0:1234").expect("Cannot open socket");
    for sock in serv.incoming() {
        let sock = sock.unwrap();
        let mut shell = shell.clone();
        let io = ShellIO::new(sock.try_clone().unwrap(), sock.try_clone().unwrap());
        shell.set_io(io);
        thread::spawn(move || shell.run_loop());
    }
}
