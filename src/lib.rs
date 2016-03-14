#[macro_use] extern crate prettytable;
use prettytable::Table;
use prettytable::row::Row;
use prettytable::format;

use std::io;
use std::io::prelude::*;
use std::string::ToString;
use std::rc::Rc;
use std::error::Error;
use std::fmt;

use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ExecError {
    Empty,
    Quit,
    MissingArgs,
    UnknownCommand(String),
    Other(Box<Error>),
}
use ExecError::*;

impl fmt::Display for ExecError {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        return match self {
            &Empty => write!(format, "No command provided"),
            &Quit => write!(format, "Quit"),
            &UnknownCommand(ref cmd) => write!(format, "Unknown Command {}", cmd),
            &MissingArgs => write!(format, "Not enough arguments"),
            &Other(ref string) => write!(format, "{}", string)
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

pub type CmdFn<T> = Box<Fn(&mut Shell<T>, &[&str]) -> ExecResult>;

pub struct Command<T> {
    name: String,
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
            return Err(MissingArgs);
        }
        return (self.func)(shell, args);
    }
}

pub struct Shell<T> {
    commands: BTreeMap<String, Rc<Command<T>>>,
    data: T,
    prompt: String,
    history: Vec<String>,
    history_size: usize
}

impl <T> Shell<T> {
    pub fn new(data: T) -> Shell<T> {
        let mut sh = Shell {
            commands: BTreeMap::new(),
            data: data,
            prompt: String::from(">"),
            history: Vec::new(),
            history_size: 10
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

    pub fn set_history_size(&mut self, size: usize) {
        self.history_size = size;
        while self.history.len() > size {
            self.history.remove(0);
        }
    }

    pub fn register_command(&mut self, cmd: Command<T>) {
        self.commands.insert(cmd.name.clone(), Rc::new(cmd));
    }

    pub fn new_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut Shell<T>, &[&str]) -> ExecResult + 'static
    {
        self.register_command(Command::new(name.to_string(), description.to_string(), nargs, Box::new(func)));
    }

    pub fn new_command_noargs<S, F>(&mut self, name: S, description: S, func: F)
        where S: ToString, F: Fn(&mut Shell<T>) -> ExecResult + 'static
    {
        self.register_command(Command::new(name.to_string(), description.to_string(), 0, Box::new(move |val, _| func(val))));
    }

    pub fn help(&self) -> ExecResult {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_CLEAN);
        for cmd in self.commands.values() {
            table.add_row(cmd.help());
        }
        table.printstd();
        return Ok(());
    }

    pub fn print_history(&self) -> ExecResult {
        let mut cnt = 0;
        for s in &self.history {
            println!("{}: {}", cnt, s);
            cnt += 1;
        }
        return Ok(());
    }

    fn push_history(&mut self, line: String) {
        self.history.push(line);
        if self.history.len() > 10 {
            self.history.remove(0);
        }
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
                self.push_history(line);
            }
            self.print_prompt();
        }
    }
}

mod builtins {
    use super::Command;
    use super::ExecError;

    pub fn help_cmd<T>() -> Command<T> {
    return Command::new("help".to_string(), "Print this help".to_string(), 0, Box::new(|shell, _| shell.help()));
    }

    pub fn quit_cmd<T>() -> Command<T> {
        return Command::new("quit".to_string(), "Quit".to_string(), 0, Box::new(|_, _| Err(ExecError::Quit)));
    }

    pub fn history_cmd<T>() -> Command<T> {
        return Command::new("history".to_string(), "Print commands history".to_string(), 0, Box::new(|shell, _| shell.print_history()));
    }
}
