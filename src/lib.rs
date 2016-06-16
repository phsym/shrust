//! <a href="https://github.com/phsym/shrust"><img style="position: absolute; top: 0; left: 0; border: 0;" src="https://camo.githubusercontent.com/121cd7cbdc3e4855075ea8b558508b91ac463ac2/68747470733a2f2f73332e616d617a6f6e6177732e636f6d2f6769746875622f726962626f6e732f666f726b6d655f6c6566745f677265656e5f3030373230302e706e67" alt="Fork me on GitHub" data-canonical-src="https://s3.amazonaws.com/github/ribbons/forkme_left_green_007200.png"></a>
//! <style>.sidebar { margin-top: 53px }</style>
//! A library for creating interactive command line shells
#[macro_use] extern crate prettytable;
use prettytable::Table;
use prettytable::format;

use std::io;
use std::io::prelude::*;
use std::string::ToString;
use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use std::collections::BTreeMap;

/// Command execution error
#[derive(Debug)]
pub enum ExecError {
    /// Empty command provided
    Empty,
    /// Exit from the shell loop
    Quit,
    /// Some arguments are missing
    MissingArgs,
    /// The provided command is unknown
    UnknownCommand(String),
    /// The history index is not valid
    InvalidHistory(usize),
    /// Other error that may have happen during command execution
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

/// Input / Output for shell execution
#[derive(Clone)]
pub struct ShellIO {
    input: Arc<Mutex<io::Read + Send>>,
    output: Arc<Mutex<io::Write + Send>>
}

impl ShellIO {
    /// Create a new Shell I/O wrapping provided Input and Output
    pub fn new<I, O>(input: I, output: O) -> ShellIO
        where I: Read + Send + 'static, O: Write + Send + 'static
    {
        return ShellIO {
            input: Arc::new(Mutex::new(input)),
            output: Arc::new(Mutex::new(output))
        };
    }

    /// Create a new Shell I/O wrapping provided Read/Write io
    pub fn new_io<T>(io: T) -> ShellIO
        where T: Read + Write + Send + 'static
    {
        let io = Arc::new(Mutex::new(io));
        return ShellIO {
            input: io.clone(),
            output: io
        };
    }
}

impl Read for ShellIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        return self.input.lock().expect("Cannot get handle to console input").read(buf);
    }
}

impl Write for ShellIO {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        return self.output.lock().expect("Cannot get handle to console output").write(buf);
    }

    fn flush(&mut self) -> io::Result<()> {
        return self.output.lock().expect("Cannot get handle to console output").flush();
    }
}

impl Default for ShellIO {
    fn default() -> Self {
        return Self::new(io::stdin(), io::stdout());
    }
}


/// Result from command execution
pub type ExecResult = Result<(), ExecError>;

/// A shell
pub struct Shell<T> {
    commands: BTreeMap<String, Arc<builtins::Command<T>>>,
    io: ShellIO,
    data: T,
    prompt: String,
    history: History
}

impl <T> Shell<T> {
    /// Create a new shell, wrapping `data`, using provided IO
    pub fn new_io(data: T, io: ShellIO) -> Shell<T> {
        let mut sh = Shell {
            commands: BTreeMap::new(),
            io: io,
            data: data,
            prompt: String::from(">"),
            history: History::new(10),
        };
        sh.register_command(builtins::help_cmd());
        sh.register_command(builtins::quit_cmd());
        sh.register_command(builtins::history_cmd());
        return sh;
    }

    /// Create a new shell, wrapping `data`, using standard input/outpu
    pub fn new(data: T) -> Shell<T> {
        return Shell::new_io(data, Default::default());
    }

    /// Get a mutable pointer to the inner data
    pub fn data(&mut self) -> &mut T {
        return &mut self.data;
    }

    /// Change the current prompt
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    ///Configure Input / Output for shell
    pub fn set_io(&mut self, io: ShellIO) {
        self.io = io;
    }

    fn register_command(&mut self, cmd: builtins::Command<T>) {
        self.commands.insert(cmd.name.clone(), Arc::new(cmd));
    }

    /// Register a shell command.
    /// Shell commands get called with a reference to the current shell
    pub fn new_shell_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut Shell<T>, &[&str]) -> ExecResult + Send + Sync + 'static
    {
        self.register_command(builtins::Command::new(name.to_string(), description.to_string(), nargs, Box::new(func)));
    }

    /// Register a command
    pub fn new_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(ShellIO, &mut T, &[&str]) -> ExecResult + Send + Sync + 'static
    {
        self.new_shell_command(name, description, nargs, move |sh, args| func(sh.io.clone(), sh.data(), args));
    }

    /// Register a command that do not accept any argument
    pub fn new_command_noargs<S, F>(&mut self, name: S, description: S, func: F)
        where S: ToString, F: Fn(ShellIO, &mut T) -> ExecResult + Send + Sync + 'static
    {
        self.new_shell_command(name, description, 0, move |sh, _| func(sh.io.clone(), sh.data()));
    }

    /// Print the help to stdout
    pub fn print_help(&mut self) -> ExecResult {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_CLEAN);
        for cmd in self.commands.values() {
            table.add_row(cmd.help());
        }
        return table.print(&mut self.io).map_err(|e| From::from(e))
    }

    /// Return the command history
    pub fn get_history(&mut self) -> &mut History {
        return &mut self.history;
    }

    /// Evaluate a command line
    pub fn eval(&mut self, line: &str) -> ExecResult {
        let mut splt = line.trim().split_whitespace();
        return match splt.next() {
            None => Err(Empty),
            Some(cmd) => match self.commands.get(cmd).map(|i| i.clone()) {
                None => Err(UnknownCommand(cmd.to_string())),
                Some(c) => c.run(self, &splt.collect::<Vec<&str>>())
            }
        };
    }

    fn print_prompt(&mut self) {
        write!(self.io, "{}", self.prompt).unwrap();
        self.io.flush().unwrap();
    }

    /// Enter the shell main loop, exiting only when
    /// the "quit" command is called
    pub fn run_loop(&mut self) {
        self.print_prompt();
        let stdin = io::BufReader::new(self.io.clone());

        for line in stdin.lines().map(|l| l.unwrap()) {
            if let Err(e) = self.eval(&line) {
                match e {
                    Empty => {},
                    Quit => return,
                    e @ _ => writeln!(self.io, "Error : {}", e).unwrap()
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

impl <T> Clone for Shell<T> where T: Clone {
    fn clone(&self) -> Self {
        return Shell {
            commands: self.commands.clone(),
            io: self.io.clone(),
            data: self.data.clone(),
            prompt: self.prompt.clone(),
            history: self.history.clone()
        };
    }
}

/// Wrap the command histroy from a shell.
/// It has a maximum capacity, and when max capacity is reached,
/// less recent command is removed from history
#[derive(Clone)]
pub struct History {
    history: Arc<Mutex<Vec<String>>>,
    capacity: usize
}

impl History {
    /// Create a new history with the given capacity
    fn new(capacity: usize) -> History {
        return History {
            history: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            capacity: capacity
        };
    }

    /// Push a command to the history, removing the oldest
    /// one if maximum capacity has been reached
    fn push(&mut self, cmd: String) {
        let mut hist = self.history.lock().unwrap();
        if hist.len() >= self.capacity {
            hist.remove(0);
        }
        hist.push(cmd);
    }

    /// Print the history to stdout
    pub fn print<T: Write>(&self, out: &mut T) {
        let mut cnt = 0;
        for s in &*self.history.lock().unwrap() {
            writeln!(out, "{}: {}", cnt, s).expect("Cannot write to output");
            cnt += 1;
        }
    }

    /// Get a command from history by its index
    pub fn get(&self, i: usize) -> Option<String> {
        return self.history.lock().unwrap().get(i).map(|s| s.clone());
    }
}

mod builtins {
    use std::str::FromStr;
    use prettytable::row::Row;
    use super::{Shell, ExecError, ExecResult};

    pub type CmdFn<T> = Box<Fn(&mut Shell<T>, &[&str]) -> ExecResult + Send + Sync>;

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
                return shell.eval(&cmd);
            } else {
                let mut io = shell.io.clone();
                shell.get_history().print(&mut io);
                return Ok(());
            }
        }));
    }
}
