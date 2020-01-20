![License](http://img.shields.io/badge/license-MIT-lightgrey.svg)
[![Build Status](https://travis-ci.org/phsym/shrust.svg)](https://travis-ci.org/phsym/shrust)
[![Coverage Status](https://coveralls.io/repos/phsym/shrust/badge.svg?branch=master)](https://coveralls.io/github/phsym/shrust?branch=master)
[![Crates.io](https://img.shields.io/crates/v/shrust.svg)](https://crates.io/crates/shrust)

# shrust

Rust library to create interactive command line shells

[Documentation](http://phsym.github.io/shrust)

*Copyright &copy; 2019 Pierre-Henri Symoneaux*

> THIS SOFTWARE IS DISTRIBUTED WITHOUT ANY WARRANTY <br>
> Check LICENSE.txt file for more information. <br>


This is currently a work in progress, and the API should be consider unstable. I'll start documenting and releasing to **crates.io** once a first level of stability has been reached

# How to use

## Including

More often, you will include the library as a dependency to your project. In order to do this, add the following lines to your **Cargo.toml** file :

```toml
[dependencies]
shrust = "0.0.7"
```

## Basic usage

Let's have a look at example [dummy.rs](./examples/dummy.rs) :
```rust
extern crate shrust;
use shrust::{Shell, ShellIO};
use std::io::prelude::*;

fn main() {
    let mut shell = Shell::new(());
    shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
        writeln!(io, "Hello World !!!")?;
        Ok(())
    });

    shell.run_loop(&mut ShellIO::default());
}
```

The output of this program would be
```
λ cargo run --example dummy
     Running `target\debug\examples\dummy.exe`
>help
 hello    :  Say 'hello' to the world
 help     :  Print this help
 history  :  Print commands history or run a command from it
 quit     :  Quit
>hello
Hello World !!!
>quit
```

## Attaching data

You can attach data to the shell for usage by commands as seen in [data.rs](./examples/data.rs):
```rust
let v = Vec::new();
let mut shell = Shell::new(v);
shell.new_command("push", "Add string to the list", 1, |io, v, s| {
    writeln!(io, "Pushing {}", s[0])?;
    v.push(s[0].to_string());
    Ok(())
});
shell.new_command_noargs("list", "List strings", |io, v| {
    for s in v {
        writeln!(io, "{}", s)?;
    }
    Ok(())
});

shell.run_loop(&mut ShellIO::default());
```
Output:
```
λ cargo run --example dummy
     Running `target\debug\examples\dummy.exe`
>help
 help     :  Print this help
 history  :  Print commands history or run a command from it
 list     :  List strings
 push     :  Add string to the list
 quit     :  Quit
>push foo
Pushing foo
>push bar
Pushing bar
>list
foo
bar
>quit
```

## Using custom I/O
In previous examples, the shell's loop was run the following way:
```rust
shell.run_loop(&mut ShellIO::default());
```
`ShellIO::default()` returns an stdin/stdout IO.

It's possible to create a `ShellIO` instance around user-defined I/O. For example to connect a `Shell` on a socket,
the `ShellIO` would be created with
```rust
let mut io = ShellIO::new_io(sock);
```
where `sock` is the socket, then the shell can be started with
```rust
shell.run_loop(&mut io);
```
This is applied in example [socket.rs](./examples/socket.rs).

## Default handler

By default, when a command is not found, the evaluation returns an `UnknownCommand` error. This behavior can be customized
by providing a custom default handler to be invoked on not found command.
```rust
let mut shell = Shell::new(());
shell.set_default(|io, _, cmd| {
    writeln!(io, "Hello from default handler !!! Received: {}", cmd)?;
    Ok(())
});
shell.run_loop(&mut ShellIO::default());
```
Output:
```
λ cargo run --example default
     Running `target\debug\examples\default.exe`
>foo
Hello from default handler !!! Received: foo
>quit
```
This is applied in example [default.rs](./examples/default.rs).

## Dynamic prompt

Sometimes it's useful to show additional context information.
This can be achieved by providing a dynamic prompt function.
```rust
struct ShellEnv {
    context: Option<String>,
};
let mut shell = Shell::new(ShellEnv { context: None });
shell.new_command("enter", "Enter context", 1, |_, env, args| {
    env.context = Some(String::from(args[0]));
    Ok(())
});
shell.new_command_noargs("leave", "Leave current context", |_, env| {
    env.context = None;
    Ok(())
});
shell.set_dynamic_prompt(|env| match &env.context {
    Some(name) => format!("[{}] >", name),
    None => String::from(">"),
});
shell.run_loop(&mut ShellIO::default());
```
Output:
```
λ cargo run --example dynamic_prompt
     Running `target/debug/examples/dynamic_prompt`
> enter main
[main] > leave
> quit
```
This is applied in example [dynamic_prompt.rs](./examples/dynamic_prompt.rs).

## Multithreading
A shell instance itself cannot be shared across threads, it needs to be cloned. A shell is clonable only if the wrapped data
is clonable too. However, the wrapped data can be easily shared if (for example) it's an `Arc` around a `Sync+Send` value.

TBD...

Additional examples are provided in documentation and in [examples](./examples/) directory
