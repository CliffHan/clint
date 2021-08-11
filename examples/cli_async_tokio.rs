use clint::*;
use log::debug;
use std::thread::Builder;
use tokio;
use tokio::sync::mpsc::unbounded_channel;

const HELP: &str = "Async Interactive client built on tokio example";

fn print_help(config: &Config) {
    let mut help = HELP.to_owned();
    if let Some(cmd) = &config.exit_command {
        help = format!("{}\n- Use command \"{}\" to exit", help, cmd);
    }
    if config.exit_on_ctrl_c {
        help = format!("{}\n- Use CTRL+C to exit", help);
    }
    if config.exit_on_esc {
        help = format!("{}\n- Use Esc to exit", help);
    }
    println!("{}", help);
}

fn main() {
    // Initiate
    let mut config = Config::default();
    let (tx, rx) = unbounded_channel();

    // Reset some prompt in config
    config.input_prompt = Some("User>".into());
    config.output_prompt = Some("Computer>".into());

    print_help(&config);

    ClintLogger::init(tx);

    // Start a thread and keep printing something
    Builder::new()
        .name("handler".into())
        .spawn(move || {
            let mut count = 0;
            loop {
                let one_second = std::time::Duration::from_secs(1);
                std::thread::sleep(one_second);
                count += 1;
                debug!("Started {} seconds", count);
            }
        })
        .unwrap();

    // Start loop
    let result = loop_async_tokio_blocking(config, rx, |cmd| {
        // "println_clint" is used to print better
        // println_clint(format!("Dispatcher received cmd={}", cmd));
        debug!("Dispatcher received cmd={}", cmd);
    });
    if let Err(e) = result {
        println!("Error: {:?}\r", e);
    }
}
