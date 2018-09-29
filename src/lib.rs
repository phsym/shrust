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
        return match *self {
            Empty => write!(format, "No command provided"),
            Quit => write!(format, "Quit"),
            UnknownCommand(ref cmd) => write!(format, "Unknown Command {}", cmd),
            InvalidHistory(i) => write!(format, "Invalid history entry {}", i),
            MissingArgs => write!(format, "Not enough arguments"),
            Other(ref e) => write!(format, "{}", e)
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
    default: Arc<Fn(&mut ShellIO, &mut Shell<T>, &str) -> ExecResult + Send + Sync>,
    data: T,
    prompt: String,
    unclosed_prompt: String,
    history: History
}

impl <T> Shell<T> {
    /// Create a new shell, wrapping `data`, using provided IO
    pub fn new(data: T) -> Shell<T> {
        let mut sh = Shell {
            commands: BTreeMap::new(),
            default: Arc::new(|_, _, cmd| Err(UnknownCommand(cmd.to_string()))),
            data,
            prompt: String::from(">"),
            unclosed_prompt: String::from(">"),
            history: History::new(10),
        };
        sh.register_command(builtins::help_cmd());
        sh.register_command(builtins::quit_cmd());
        sh.register_command(builtins::history_cmd());
        return sh;
    }

    /// Get a mutable pointer to the inner data
    pub fn data(&mut self) -> &mut T {
        return &mut self.data;
    }

    /// Change the current prompt
    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    /// Change the current unclosed prompt
    pub fn set_unclosed_prompt(&mut self, prompt: String) {
        self.unclosed_prompt = prompt;
    }

    fn register_command(&mut self, cmd: builtins::Command<T>) {
        self.commands.insert(cmd.name.clone(), Arc::new(cmd));
    }

    // Set a custom default handler, invoked when a command is not found
    pub fn set_default<F>(&mut self, func: F)
        where F: Fn(&mut ShellIO, &mut Shell<T>, &str) -> ExecResult + Send + Sync + 'static
    {
        self.default = Arc::new(func);
    }

    /// Register a shell command.
    /// Shell commands get called with a reference to the current shell
    pub fn new_shell_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut ShellIO, &mut Shell<T>, &[&str]) -> ExecResult + Send + Sync + 'static
    {
        self.register_command(builtins::Command::new(name.to_string(), description.to_string(), nargs, Box::new(func)));
    }

    /// Register a command
    pub fn new_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
        where S: ToString, F: Fn(&mut ShellIO, &mut T, &[&str]) -> ExecResult + Send + Sync + 'static
    {
        self.new_shell_command(name, description, nargs, move |io, sh, args| func(io, sh.data(), args));
    }

    /// Register a command that do not accept any argument
    pub fn new_command_noargs<S, F>(&mut self, name: S, description: S, func: F)
        where S: ToString, F: Fn(&mut ShellIO, &mut T) -> ExecResult + Send + Sync + 'static
    {
        self.new_shell_command(name, description, 0, move |io, sh, _| func(io, sh.data()));
    }

    /// Print the help to stdout
    pub fn print_help(&self, io: &mut ShellIO) -> ExecResult {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_CLEAN);
        for cmd in self.commands.values() {
            table.add_row(cmd.help());
        }
        table.print(io)?;
        Ok(())
    }

    /// Return the command history
    pub fn get_history(&self) -> &History {
        return &self.history;
    }

    /// Evaluate a command line
    pub fn eval(&mut self, io: &mut ShellIO, line: &str) -> ExecResult {
        let mut splt = line.trim().split_whitespace();
        return match splt.next() {
            None => Err(Empty),
            Some(cmd) => match self.commands.get(cmd).cloned() {
                None => self.default.clone()(io, self, line),
                Some(c) => c.run(io, self, &splt.collect::<Vec<&str>>())
            }
        };
    }

    fn print_prompt(&self, io: &mut ShellIO, unclosed: bool) {
        if unclosed {
            write!(io, "{} ", self.unclosed_prompt).unwrap();
        } else {
            write!(io, "{} ", self.prompt).unwrap();
        }
        io.flush().unwrap();
    }

    /// Enter the shell main loop, exiting only when
    /// the "quit" command is called
    pub fn run_loop(&mut self, io: &mut ShellIO) {
        self.print_prompt(io, false);
        let stdin = io::BufReader::new(io.clone());
        let mut iter = stdin.lines().map(|l| l.unwrap());
        while let Some(mut line) = iter.next() {
            while !line.is_empty() && &line[line.len()-1 ..] == "\\" {
                self.print_prompt(io, true);
                line.pop();
                line.push_str(&iter.next().unwrap())
            }
            if let Err(e) = self.eval(io, &line) {
                match e {
                    Empty => {},
                    Quit => return,
                    e => writeln!(io, "Error : {}", e).unwrap()
                };
            } else {
                self.get_history().push(line);
            }
            self.print_prompt(io, false);
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
            default: self.default.clone(),
            data: self.data.clone(),
            prompt: self.prompt.clone(),
            unclosed_prompt: self.unclosed_prompt.clone(),
            history: self.history.clone()
        };
    }
}

/// Wrap the command history from a shell.
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
            capacity
        };
    }

    /// Push a command to the history, removing the oldest
    /// one if maximum capacity has been reached
    fn push(&self, cmd: String) {
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
        return self.history.lock().unwrap().get(i).cloned();
    }
}

mod builtins {
    use std::str::FromStr;
    use prettytable::Row;
    use super::{Shell, ShellIO, ExecError, ExecResult};

    pub type CmdFn<T> = Box<Fn(&mut ShellIO, &mut Shell<T>, &[&str]) -> ExecResult + Send + Sync>;

    pub struct Command<T> {
        pub name: String,
        description: String,
        nargs: usize,
        func: CmdFn<T>
    }

    impl <T> Command<T> {
        pub fn new(name: String, description: String, nargs: usize, func: CmdFn<T>) -> Command<T> {
            return Command {
                name,
                description,
                nargs,
                func
            };
        }

        pub fn help(&self) -> Row {
            return row![self.name, ":", self.description];
        }

        pub fn run(&self, io: &mut ShellIO, shell: &mut Shell<T>, args: &[&str]) -> ExecResult {
            if args.len() < self.nargs {
                return Err(ExecError::MissingArgs);
            }
            return (self.func)(io, shell, args);
        }
    }

    pub fn help_cmd<T>() -> Command<T> {
        return Command::new("help".to_string(), "Print this help".to_string(), 0, Box::new(|io, shell, _| shell.print_help(io)));
    }

    pub fn quit_cmd<T>() -> Command<T> {
        return Command::new("quit".to_string(), "Quit".to_string(), 0, Box::new(|_, _, _| Err(ExecError::Quit)));
    }

    pub fn history_cmd<T>() -> Command<T> {
        return Command::new("history".to_string(), "Print commands history or run a command from it".to_string(), 0, Box::new(|io, shell, args| {
            if !args.is_empty() {
                let i = try!(usize::from_str(args[0]));
                let cmd = try!(shell.get_history().get(i).ok_or_else(|| ExecError::InvalidHistory(i)));
                return shell.eval(io, &cmd);
            } else {
                shell.get_history().print(io);
                return Ok(());
            }
        }));
    }
}
