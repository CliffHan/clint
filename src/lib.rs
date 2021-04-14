use crossterm::cursor::{MoveToColumn};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, Result};
use std::io::{stdout, Write};
use std::time::Duration;
use std::sync::mpsc::{channel, Sender, Receiver};

macro_rules! print_now {
    ($t:ident) => {
        print!("{}", $t);
        let _ = stdout().flush();
    };
    ($fmt:expr, $($arg:tt), +) => {
        print!($fmt, $($arg), +);
        let _ = stdout().flush();
    };
}

macro_rules! clear_line {
    () => {
        let _ = execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0));
    };
}

pub struct Config {
    pub prompt: Option<String>,
    pub info_prompt: Option<String>,
    pub exit_command: Option<String>,
    pub exit_on_esc: bool,
    pub exit_on_ctrl_c: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt: Some("User>".into()),
            info_prompt: Some("Computer>".into()),
            exit_command: Some("exit".into()),
            exit_on_esc: true,
            exit_on_ctrl_c: true,
        }
    }
}

pub struct Emitter {
    pub printer: Receiver<String>,
    pub notifier: Sender<String>,
}

impl Emitter {
    pub fn new(printer: Receiver<String>, notifier: Sender<String>) -> Self {
        Emitter { printer, notifier }
    }
}

pub fn create_channel() -> (Emitter, Sender<String>, Receiver<String>) {
    let (s1, r1) = channel();
    let (s2, r2) = channel();
    let e = Emitter::new(r1, s2);
    (e, s1, r2)
}

pub fn interact(emitter: Emitter, interval: Duration, config: Config) -> Result<()> {
    enable_raw_mode()?;

    let result = interact_sync(emitter, interval, config);
    if result.is_err() {
        let _ = disable_raw_mode();
        return result;
    }

    disable_raw_mode()
}

fn exit_on_key(config: &Config, event: &KeyEvent) -> bool {
    use KeyCode::{Esc, Char};
    match (event.code, event.modifiers) {
        (Esc, _) => config.exit_on_esc,
        (Char('c'), m) | (Char('C'), m) => config.exit_on_ctrl_c && m.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

fn output_prompt(prompt: &Option<String>) {
    if let Some(s) = prompt {
        print_now!("{} ", s);
    }
}

fn output_with_prompt(prompt: &Option<String>, command: &str, newline: bool) {
    match (prompt, newline) {
        (Some(s), true) => println!("{} {}", s, command),
        (None, true) => println!("{}", command),
        (Some(s), false) => { print_now!("{} {}", s, command); },
        (None, false) => { print_now!(command); },
    }
}

fn output_on_key(config: &Config, event: &KeyEvent, cmd: &mut String, sender: &Sender<String>) -> bool {
    match event.code {
        KeyCode::Enter => {
            println!("\r");
            if !cmd.is_empty() {
                if let Some(exit_command) = &config.exit_command {
                    if cmd == exit_command {
                        return true;
                    }
                }
                sender.send(cmd.to_string()).unwrap();
                cmd.clear();
            }
            output_prompt(&config.prompt);
        }
        KeyCode::Backspace => {
            cmd.pop();
            clear_line!();
            output_with_prompt(&config.prompt, cmd, false);
        }
        KeyCode::Char(c) => {
            cmd.push(c);
            print_now!(c);
        },
        _ => {},
    }
    false
}

fn output_on_info(config: &Config, info: &String, cmd: &str) {
    clear_line!();
    output_with_prompt(&config.info_prompt, info, true);
    clear_line!();
    output_with_prompt(&config.prompt, cmd, false);
}

fn interact_sync(emitter: Emitter, interval: Duration, config: Config) -> Result<()> {
    // let mut writer = stdout();
    let mut cmd = String::new();
    println!("\r");
    output_prompt(&config.prompt);
    loop {
        if poll(interval)? {
            let event = read()?;
            // println!("Debug: Event::{:?}\r", event);
            match event {
                Event::Key(keyevent) => {
                    if exit_on_key(&config, &keyevent) {
                        break;
                    }
                    if output_on_key(&config, &keyevent, &mut cmd, &emitter.notifier) {
                        break;
                    }
                }
                _ => {}
            }
        } else {
            if let Ok(info) = emitter.printer.try_recv() {
                output_on_info(&config, &info, &cmd);
            }
        }
    }
    Ok(())
}
