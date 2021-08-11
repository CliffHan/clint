use clint::*;
use log::debug;
use std::thread::Builder;
use std::time::Duration;

const HELP: &str = "Sync Interactive client example";

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
    let interval = Duration::from_millis(100);
    let mut config = Config::default();
    // let mut dispatcher = Dispatcher::default();
    let (tx, rx) = flume::unbounded();
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
                // tx.send(format!("Started {} seconds", count)).unwrap();
                debug!("Started {} seconds", count)
            }
        })
        .unwrap();

    // Start loop
    let result = loop_sync(config, interval, rx, |cmd| {
        // "println_clint" is used to print better
        // println_clint(format!("Dispatcher received cmd={}", cmd));
        debug!("Dispatcher received cmd={}", cmd)
    });
    if let Err(e) = result {
        println!("Error: {:?}\r", e);
    }
}
