use crossterm::cursor::MoveToColumn;
#[cfg(feature = "sync")]
use crossterm::event::{poll, read, Event};
#[cfg(any(feature = "sync", feature = "async-tokio"))]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[cfg(any(feature = "sync", feature = "async-tokio"))]
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute};
#[cfg(any(feature = "sync", feature = "async-tokio"))]
use crossterm::{Result as CrossTermResult};
use std::io::stdout;
#[cfg(any(feature = "sync", feature = "async-tokio"))]
use std::io::Write;
#[cfg(any(feature = "sync", feature = "async-tokio"))]
use std::time::Duration;
#[cfg(feature = "sync")]
use std::sync::mpsc::Receiver as SyncReceiver;
#[cfg(feature = "async-tokio")]
use tokio::runtime::Runtime;
#[cfg(feature = "async-tokio")]
use tokio::sync::mpsc::UnboundedReceiver as TokioReceiver;
#[cfg(feature = "async-tokio")]
use futures::StreamExt;
#[cfg(feature = "async-tokio")]
use futures_timer::Delay;
#[cfg(feature = "async-tokio")]
use crossterm::event::{Event, EventStream};

pub type CmdFunc = fn(String);

#[derive(Clone)]
pub struct Config {
    pub input_prompt: Option<String>,
    pub output_prompt: Option<String>,
    pub exit_command: Option<String>,
    pub exit_on_esc: bool,
    pub exit_on_ctrl_c: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_prompt: Some("Input>".into()),
            output_prompt: Some("Output>".into()),
            exit_command: Some("exit".into()),
            exit_on_esc: true,
            exit_on_ctrl_c: true,
        }
    }
}

#[cfg(any(feature = "sync", feature = "async-tokio"))]
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

#[cfg(any(feature = "sync", feature = "async-tokio"))]
fn exit_on_key(config: &Config, event: &KeyEvent) -> bool {
    use KeyCode::{Char, Esc};
    match (event.code, event.modifiers) {
        (Esc, _) => config.exit_on_esc,
        (Char('c'), m) | (Char('C'), m) => {
            config.exit_on_ctrl_c && m.contains(KeyModifiers::CONTROL)
        }
        _ => false,
    }
}

#[cfg(any(feature = "sync", feature = "async-tokio"))]
fn output_prompt(prompt: &Option<String>) {
    if let Some(s) = prompt {
        print_now!("{} ", s);
    }
}

#[cfg(any(feature = "sync", feature = "async-tokio"))]
fn output_with_prompt(prompt: &Option<String>, command: &str, newline: bool) {
    clear_line!();
    match (prompt, newline) {
        (Some(s), true) => println!("{} {}", s, command),
        (None, true) => println!("{}", command),
        (Some(s), false) => {
            print_now!("{} {}", s, command);
        }
        (None, false) => {
            print_now!(command);
        }
    }
}

#[cfg(any(feature = "sync", feature = "async-tokio"))]
fn output_on_key<F>(cfg: &Config, evt: &KeyEvent, cmd: &mut String, cb: &F) -> bool
where
    F: Fn(String),
{
    match evt.code {
        KeyCode::Enter => {
            println!("\r");
            if !cmd.is_empty() {
                if let Some(exit_command) = &cfg.exit_command {
                    if cmd == exit_command {
                        return true;
                    }
                }
                cb(cmd.to_string());
                cmd.clear();
            }
            output_prompt(&cfg.input_prompt);
        }
        KeyCode::Backspace => {
            cmd.pop();
            clear_line!();
            output_with_prompt(&cfg.input_prompt, cmd, false);
        }
        KeyCode::Char(c) => {
            cmd.push(c);
            print_now!(c);
        }
        _ => {}
    }
    false
}

#[cfg(any(feature = "sync", feature = "async-tokio"))]
fn output_on_info(config: &Config, info: &str, cmd: &str) {
    output_with_prompt(&config.output_prompt, info, true);
    output_with_prompt(&config.input_prompt, cmd, false);
}

pub fn println_clint(info: String) {
    clear_line!();
    println!("{}", info);
    clear_line!();
}

#[cfg(feature = "sync")]
pub fn loop_sync<F>(
    config: Config,
    interval: Duration,
    receiver: SyncReceiver<String>,
    callback: F,
) -> CrossTermResult<()>
where
    F: Fn(String),
{
    enable_raw_mode()?;

    let result = loop_sync_internal(config, interval, receiver, callback);
    if result.is_err() {
        let _ = disable_raw_mode();
        return result;
    }

    disable_raw_mode()
}

#[cfg(feature = "sync")]
fn loop_sync_internal<F>(
    config: Config,
    interval: Duration,
    receiver: SyncReceiver<String>,
    callback: F,
) -> CrossTermResult<()>
where
    F: Fn(String),
{
    // let mut writer = stdout();
    let mut cmd = String::new();
    println!("\r");
    output_prompt(&config.input_prompt);
    loop {
        if poll(interval)? {
            if let Event::Key(keyevent) = read()? {
                if exit_on_key(&config, &keyevent) {
                    println!("\r");
                    break;
                }
                if output_on_key(&config, &keyevent, &mut cmd, &callback) {
                    break;
                }
            }
        } else if let Ok(info) = receiver.try_recv() {
            output_on_info(&config, &info, &cmd);
        }
    }
    Ok(())
}

// #[cfg(feature = "async")]
// async fn output(text: String) {
//     output_on_info(&config, &info, &cmd);
// }

#[cfg(feature = "async-tokio")]
pub fn loop_async_tokio_blocking<F>(
    config: Config,
    receiver: TokioReceiver<String>,
    callback: F,
) -> CrossTermResult<()>
where
    F: Fn(String),
{
    enable_raw_mode()?;

    // Create the runtime
    let rt  = Runtime::new().unwrap();

    // Execute the future, blocking the current thread until completion
    let result = rt.block_on(async move {
        loop_async_tokio_internal(config, receiver, callback).await
    });

    if result.is_err() {
        let _ = disable_raw_mode();
        return result;
    }

    disable_raw_mode()
}

#[cfg(feature = "async-tokio")]
async fn loop_async_tokio_internal<F>(
    config: Config,
    mut receiver: TokioReceiver<String>,
    callback: F,
) -> CrossTermResult<()>
where
    F: Fn(String),
{
    let mut reader = EventStream::new();
    let mut cmd = String::new();

    loop {
        let delay = Delay::new(Duration::from_millis(1_000));
        let event = reader.next();
        let ext_event = receiver.recv();

        tokio::select! {
            _ = delay => {},
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        // println!("Event::{:?}\r", event);
                        if let Event::Key(keyevent) = event {
                            if exit_on_key(&config, &keyevent) {
                                println!("\r");
                                break Ok(());
                            }
                            if output_on_key(&config, &keyevent, &mut cmd, &callback) {
                                break Ok(());
                            }
                        }
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break Ok(()),
                }
            },
            ext = ext_event => {
                if let Some(info) = ext{
                        output_on_info(&config, &info, &cmd);
                }
            }
        };
    }
}