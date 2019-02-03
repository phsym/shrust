//! A library for creating interactive command line shells
#![feature(trait_alias)]
#[macro_use]
extern crate prettytable;
extern crate futures;
extern crate tokio;
use prettytable::format;
use prettytable::Table;

use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::io::prelude::*;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::string::ToString;
use std::sync::{Arc, Mutex};

use futures::future;
use futures::prelude::*;
use std::collections::BTreeMap;
use tokio::io::{AsyncRead, AsyncWrite};

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
use crate::ExecError::*;

impl fmt::Display for ExecError {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        return match *self {
            Empty => write!(format, "No command provided"),
            Quit => write!(format, "Quit"),
            UnknownCommand(ref cmd) => write!(format, "Unknown Command {}", cmd),
            InvalidHistory(i) => write!(format, "Invalid history entry {}", i),
            MissingArgs => write!(format, "Not enough arguments"),
            Other(ref e) => write!(format, "{}", e),
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

impl<E: Error + 'static> From<E> for ExecError {
    fn from(e: E) -> ExecError {
        return Other(Box::new(e));
    }
}

/// Input / Output for shell execution
#[derive(Clone)]
pub struct ShellIO {
    input: Rc<Mutex<AsyncRead>>,
    output: Rc<Mutex<AsyncWrite>>,
}

impl ShellIO {
    /// Create a new Shell I/O wrapping provided Input and Output
    pub fn new<I, O>(input: I, output: O) -> ShellIO
    where
        I: AsyncRead + 'static,
        O: AsyncWrite + 'static,
    {
        return ShellIO {
            input: Rc::new(Mutex::new(input)),
            output: Rc::new(Mutex::new(output)),
        };
    }

    /// Create a new Shell I/O wrapping provided Read/Write io
    pub fn new_io<T>(io: T) -> ShellIO
    where
        T: AsyncRead + AsyncWrite + 'static,
    {
        let io = Rc::new(Mutex::new(io));
        return ShellIO {
            input: io.clone(),
            output: io,
        };
    }
}

impl AsyncRead for ShellIO {}
impl Read for ShellIO {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        return self
            .input
            .lock()
            .expect("Cannot get handle to console input")
            .read(buf);
    }
}

impl AsyncWrite for ShellIO {
    fn shutdown(&mut self) -> Poll<(), std::io::Error> {
        return self
            .output
            .lock()
            .expect("Cannot get handle to console output")
            .shutdown();
    }
}
impl Write for ShellIO {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        return self
            .output
            .lock()
            .expect("Cannot get handle to console output")
            .write(buf);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        return self
            .output
            .lock()
            .expect("Cannot get handle to console output")
            .flush();
    }
}

/// Result from command execution
pub type ExecResult = Result<(), ExecError>;

/// A shell
pub struct Shell<T: 'static> {
    commands: BTreeMap<String, Arc<builtins::Command<T>>>,
    default:
        Arc<Fn(&mut ShellIO, &mut Shell<T>, &str) -> Box<dyn Future<Item = (), Error = ExecError>>>,
    data: T,
    prompt: String,
    unclosed_prompt: String,
    history: History,
}
impl<T> Shell<T> {
    /// Create a new shell, wrapping `data`, using provided IO
    pub fn new(data: T) -> Shell<T> {
        let mut sh = Shell {
            commands: BTreeMap::new(),
            default: Arc::new(|_, _, cmd| Box::new(future::err(UnknownCommand(cmd.to_string())))),
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
    where
        F: Fn(&mut ShellIO, &mut Shell<T>, &str) -> Box<dyn Future<Item = (), Error = ExecError>>
            + 'static,
    {
        self.default = Arc::new(func);
    }

    /// Register a shell command.
    /// Shell commands get called with a reference to the current shell
    pub fn new_shell_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
    where
        S: ToString,
        F: (Fn(
                &mut ShellIO,
                &mut Shell<T>,
                &[&str],
            ) -> Box<dyn Future<Item = (), Error = ExecError>>)
            + 'static,
    {
        self.register_command(builtins::Command::new(
            name.to_string(),
            description.to_string(),
            nargs,
            Box::new(func),
        ));
    }

    /// Register a command
    pub fn new_command<S, F>(&mut self, name: S, description: S, nargs: usize, func: F)
    where
        S: ToString,
        F: (Fn(&mut ShellIO, &mut T, &[&str]) -> Box<dyn Future<Item = (), Error = ExecError>>)
            + 'static,
    {
        self.new_shell_command(name, description, nargs, move |io, sh, args| {
            func(io, sh.data(), args)
        });
    }

    /// Register a command that do not accept any argument
    pub fn new_command_noargs<S, F>(&mut self, name: S, description: S, func: F)
    where
        S: ToString,
        F: (Fn(&mut ShellIO, &mut T) -> Box<dyn Future<Item = (), Error = ExecError>>) + 'static,
    {
        self.new_shell_command(name, description, 0, move |io, sh, _| func(io, sh.data()));
    }

    /// Print the help to stdout
    pub fn print_help(&self, io: &mut ShellIO) -> Box<dyn Future<Item = (), Error = ExecError>> {
        let mut func = move || {
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_CLEAN);
            for cmd in self.commands.values() {
                table.add_row(cmd.help());
            }
            table.print(io)?;
            Ok(())
        };
        Box::new(future::result(func()))
    }

    /// Return the command history
    pub fn get_history(&self) -> &History {
        return &self.history;
    }

    /// Evaluate a command line
    pub fn eval(
        &mut self,
        io: &mut ShellIO,
        line: &str,
    ) -> Box<dyn Future<Item = (), Error = ExecError>> {
        let mut splt = line.trim().split_whitespace();
        match splt.next() {
            None => Box::new(future::err(Empty)),
            Some(cmd) => match self.commands.get(cmd).cloned() {
                None => self.default.clone()(io, self, line),
                Some(c) => c.run(io, self, &splt.collect::<Vec<&str>>()),
            },
        }
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
    pub fn run_loop<R: AsyncRead + 'static, W: AsyncWrite + 'static>(
        self,
        read: R,
        write: W,
    ) -> Box<dyn Future<Item = (), Error = ExecError>> {
        let mut io = ShellIO::new(read, write);
        let framed_read =
            tokio::codec::FramedRead::new(io.clone(), tokio::codec::LinesCodec::new());
        self.print_prompt(&mut io, false);
        let shell = Rc::new(RefCell::new(self));
        let stream_read_future = framed_read.map_err(|e| Other(Box::new(e))).for_each(
            move |line| -> Box<dyn Future<Item = (), Error = ExecError>> {
                let inner_shell = shell.clone();
                let mut io = io.clone();
                Box::new(shell.borrow_mut().eval(&mut io, &line).then(
                    move |result| -> Box<dyn Future<Item = (), Error = ExecError>> {
                        match result {
                            Ok(_) => {
                                inner_shell.borrow_mut().get_history().push(line);
                            }
                            Err(Empty) => {}
                            Err(Quit) => return Box::new(future::err(Quit)),
                            Err(e) => {
                                writeln!(io, "{}", e).unwrap();
                            }
                        };
                        inner_shell.borrow_mut().print_prompt(&mut io, false);
                        Box::new(future::ok(()))
                    },
                ))
            },
        );
        Box::new(stream_read_future)
    }
}

impl<T> Deref for Shell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        return &self.data;
    }
}

impl<T> DerefMut for Shell<T> {
    fn deref_mut(&mut self) -> &mut T {
        return &mut self.data;
    }
}

impl<T> Clone for Shell<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        return Shell {
            commands: self.commands.clone(),
            default: self.default.clone(),
            data: self.data.clone(),
            prompt: self.prompt.clone(),
            unclosed_prompt: self.unclosed_prompt.clone(),
            history: self.history.clone(),
        };
    }
}

/// Wrap the command history from a shell.
/// It has a maximum capacity, and when max capacity is reached,
/// less recent command is removed from history
#[derive(Clone)]
pub struct History {
    history: Arc<Mutex<Vec<String>>>,
    capacity: usize,
}

impl History {
    /// Create a new history with the given capacity
    fn new(capacity: usize) -> History {
        return History {
            history: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            capacity,
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
    pub fn print(&self, out: &mut ShellIO) {
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
    use super::{ExecError, Shell, ShellIO};
    use futures::future;
    use futures::prelude::*;
    use prettytable::Row;
    use std::str::FromStr;

    pub type CmdFn<T> = Box<
        Fn(&mut ShellIO, &mut Shell<T>, &[&str]) -> Box<dyn Future<Item = (), Error = ExecError>>,
    >;

    pub struct Command<T: 'static> {
        pub name: String,
        description: String,
        nargs: usize,
        func: CmdFn<T>,
    }

    impl<T> Command<T> {
        pub fn new(name: String, description: String, nargs: usize, func: CmdFn<T>) -> Command<T> {
            return Command {
                name,
                description,
                nargs,
                func,
            };
        }

        pub fn help(&self) -> Row {
            return row![self.name, ":", self.description];
        }

        pub fn run(
            &self,
            io: &mut ShellIO,
            shell: &mut Shell<T>,
            args: &[&str],
        ) -> Box<dyn Future<Item = (), Error = ExecError>> {
            if args.len() < self.nargs {
                return Box::new(future::err(ExecError::MissingArgs));
            }
            return (self.func)(io, shell, args);
        }
    }

    pub fn help_cmd<T>() -> Command<T> {
        return Command::new(
            "help".to_string(),
            "Print this help".to_string(),
            0,
            Box::new(|io, shell, _| shell.print_help(io)),
        );
    }

    pub fn quit_cmd<T>() -> Command<T> {
        return Command::new(
            "quit".to_string(),
            "Quit".to_string(),
            0,
            Box::new(|_, _, _| Box::new(future::err(ExecError::Quit))),
        );
    }

    pub fn history_cmd<T>() -> Command<T> {
        return Command::new(
            "history".to_string(),
            "Print commands history or run a command from it".to_string(),
            0,
            Box::new(|io, shell, args| {
                let mut func =
                    move || -> Result<Box<dyn Future<Item = (), Error = ExecError>>, ExecError> {
                        if !args.is_empty() {
                            let i = usize::from_str(args[0])?;
                            let cmd = shell
                                .get_history()
                                .get(i)
                                .ok_or_else(|| ExecError::InvalidHistory(i))?;
                            Ok(shell.eval(io, &cmd))
                        } else {
                            shell.get_history().print(io);
                            Ok(Box::new(future::ok(())))
                        }
                    };
                return func().unwrap_or_else(|e| Box::new(future::err(e)));
            }),
        );
    }
}
