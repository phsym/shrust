extern crate shrust;
use shrust::{Shell, ShellIO};

fn main() {
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
}
