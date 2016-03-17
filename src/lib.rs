#[macro_use] extern crate prettytable;
use prettytable::Table;
use prettytable::format;

use std::io;
use std::io::prelude::*;
use std::string::ToString;
use std::rc::Rc;
use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};

use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ExecError {
    Empty,
    Quit,
    MissingArgs,
    UnknownCommand(String),
    InvalidHistory(usize),
    Other(Box<Error>),
}
use ExecError::*;

impl fmt::Display for ExecError {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        return match self {
            &Empty => write!(format, "No command provided"),
            &Quit => write!(format, "Quit"),
            &UnknownCommand(ref cmd) => write!(format, "Unknown Command {}", cmd),
            &InvalidHistory(i) => write!(format, "Invalid history entry {}", i),
            &MissingArgs => write!(format, "Not enough arguments"),
            &Other(ref e) => write!(format, "{}", e)
        };
    }
}

// impl Error for ExecError {
//     fn description(&self) -> &str {
//         return match self {
//             &Quit => "The command requested to quit",
//             &UnknownCommand(..) => "The provided command is unknown",
//             &MissingArgs => "Not enough arguments have been provided",
//             &Other(..) => "Other error occured"
//         };
//     }
// }

impl <E: Error + 'static> From<E> for ExecError {
    fn from(e: E) -> ExecError {
        return Other(Box::new(e));
    }
}

pub type ExecResult = Result<(), ExecError>;

pub struct Shell<T> {
    commands: BTreeMap<String, Rc<builtins::Command<T>>>,
    data: T,
    prompt: String,
    history: History
}

impl <T> Shell<T> {
    pub fn new(data: T) -> Shell<T> {
        let mut sh = Shell {
            commands: BTreeMap::new(),
            data: data,
            prompt: String::from(">"),
            history: History::new(10),
        };
        sh.register_command(builtins::help_cmd());
        sh.register_command(builtins::quit_cmd());
        sh.register_command(builtins::history_cmd());
        return sh;
    }

    pub fn data(&mut self) -> &mut T {
        return &mut self.data;
    }

    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    fn register_command(&mut self, cmd: builtins::Command<T>) {
        self.commands.insert(cmd.name.clone(), Rc::new(cmd));
    }

    pub fn new_shell_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut Shell<T>, &[&str]) -> ExecResult + 'static
    {
        self.register_command(builtins::Command::new(name.to_string(), description.to_string(), nargs, Box::new(func)));
    }

    pub fn new_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut T, &[&str]) -> ExecResult + 'static
    {
        self.new_shell_command(name, description, nargs, move |sh, args| func(sh.data(), args));
    }

    pub fn new_command_noargs<S, F>(&mut self, name: S, description: S, func: F)
        where S: ToString, F: Fn(&mut T) -> ExecResult + 'static
    {
        self.new_shell_command(name, description, 0, move |sh, _| func(sh.data()));
    }

    pub fn print_help(&self) -> ExecResult {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_CLEAN);
        for cmd in self.commands.values() {
            table.add_row(cmd.help());
        }
        table.printstd();
        return Ok(());
    }

    pub fn get_history(&mut self) -> &mut History {
        return &mut self.history;
    }

    pub fn run(&mut self, line: &str) -> ExecResult {
        let mut splt = line.trim().split_whitespace();
        return match splt.next() {
            None => Err(Empty),
            Some(cmd) => match self.commands.get(cmd).map(|i| i.clone()) {
                None => Err(UnknownCommand(cmd.to_string())),
                Some(c) => c.run(self, &splt.collect::<Vec<&str>>())
            }
        };
    }

    fn print_prompt(&self) {
        let mut stdout = io::stdout();
        write!(stdout, "{}", self.prompt).unwrap();
        stdout.flush().unwrap();
    }

    pub fn run_loop(&mut self) {
        let stdin = io::stdin();
        self.print_prompt();
        for line in stdin.lock().lines().map(|l| l.unwrap()) {
            if let Err(e) =  self.run(&line) {
                match e {
                    Empty => {},
                    Quit => return,
                    e @ _ => println!("Error : {}", e)
                };
            } else {
                self.get_history().push(line);
            }
            self.print_prompt();
        }
    }
}

impl <T> Deref for Shell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        return &self.data;
    }
}

impl <T> DerefMut for Shell<T> {
    fn deref_mut(&mut self) -> &mut T {
        return &mut self.data;
    }
}

pub struct History {
    history: Vec<String>,
    capacity: usize
}

impl History {
    fn new(capacity: usize) -> History {
        return History {
            history: Vec::with_capacity(capacity),
            capacity: capacity
        };
    }

    fn push(&mut self, cmd: String) {
        if self.history.len() >= self.capacity {
            self.history.remove(0);
        }
        self.history.push(cmd);
    }

    pub fn print(&self) {
        let mut cnt = 0;
        for s in &self.history {
            println!("{}: {}", cnt, s);
            cnt += 1;
        }
    }

    pub fn get(&self, i: usize) -> Option<String> {
        return self.history.get(i).map(|s| s.clone());
    }
}

mod builtins {
    use std::str::FromStr;
    use prettytable::row::Row;
    use super::{Shell, ExecError, ExecResult};

    pub type CmdFn<T> = Box<Fn(&mut Shell<T>, &[&str]) -> ExecResult>;

    pub struct Command<T> {
        pub name: String,
        description: String,
        nargs: usize,
        func: CmdFn<T>
    }

    impl <T> Command<T> {
        pub fn new(name: String, description: String, nargs: usize, func: CmdFn<T>) -> Command<T> {
            return Command {
                name: name,
                description: description,
                nargs: nargs,
                func: func
            };
        }

        pub fn help(&self) -> Row {
            return row![self.name, ":", self.description];
        }

        pub fn run(&self, shell: &mut Shell<T>, args: &[&str]) -> ExecResult {
            if args.len() < self.nargs {
                return Err(ExecError::MissingArgs);
            }
            return (self.func)(shell, args);
        }
    }

    pub fn help_cmd<T>() -> Command<T> {
        return Command::new("help".to_string(), "Print this help".to_string(), 0, Box::new(|shell, _| shell.print_help()));
    }

    pub fn quit_cmd<T>() -> Command<T> {
        return Command::new("quit".to_string(), "Quit".to_string(), 0, Box::new(|_, _| Err(ExecError::Quit)));
    }

    pub fn history_cmd<T>() -> Command<T> {
        return Command::new("history".to_string(), "Print commands history or run a command from it".to_string(), 0, Box::new(|shell, args| {
            if args.len() > 0 {
                let i = try!(usize::from_str(args[0]));
                let cmd = try!(shell.get_history().get(i).ok_or(ExecError::InvalidHistory(i)));
                return shell.run(&cmd);
            } else {
                shell.get_history().print();
                return Ok(());
            }
        }));
    }
}
