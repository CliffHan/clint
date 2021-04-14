use clint::{create_channel, interact, Config};
use std::time::Duration;

// const HELP: &str = r#"Blocking poll() & non-blocking read()
//  - Keyboard events enabled
//  - Use Esc to quit
// "#;
const HELP: &str = "Blocking interact with sender/receiver";

fn main() {
    let (emitter, info_sender, cmd_receiver) = create_channel();
    let interval = Duration::from_millis(100);
    let config: Config = Config::default();

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

    let builder = std::thread::Builder::new().name("handler".into());
    builder.spawn(move || {
      loop {
        let cmd_result = cmd_receiver.recv();
        let cmd_str = format!("Received command: {:?}", cmd_result);
        if let Err(_) = info_sender.send(cmd_str) {
          break;
        }
      }
    }).unwrap();

    let result = interact(emitter, interval, config);
    if let Err(e) = result {
        println!("Error: {:?}\r", e);
    }
}
