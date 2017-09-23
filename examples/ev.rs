extern crate env_logger;
extern crate gilrs;

use gilrs::Gilrs;
use gilrs::ev::filter::{axis_dpad_to_button, deadzone, Filter, Jitter, Repeat};

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();
    let noise_filter = Jitter::new();
    let repeat_filter = Repeat::new();

    loop {
        while let Some(ev) = gilrs
            .next_event()
            .filter(&axis_dpad_to_button, &gilrs)
            .filter(&noise_filter, &gilrs)
            .filter(&deadzone, &gilrs)
            .filter(&repeat_filter, &gilrs)
        {
            if !ev.is_dropped() {
                gilrs.update(&ev);
                println!("{:?}", ev);
            }
        }

        if gilrs.counter() % 250 == 0 {
            for (id, gamepad) in gilrs.gamepads() {
                println!(
                    "Power info of gamepad {}({}): {:?}",
                    id,
                    gamepad.name(),
                    gamepad.power_info()
                );
            }
        }

        gilrs.inc();
        thread::sleep(Duration::from_millis(33));
    }
}
