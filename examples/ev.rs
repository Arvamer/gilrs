extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ev::filter::{Filter, Jitter, Repeat};

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();
    let noise_filter = Jitter::new();
    let repeat_filter = Repeat::new();

    loop {
        while let Some(ev) = gilrs.next_event().filter(&noise_filter, &gilrs).filter(
            &repeat_filter,
            &gilrs,
        )
        {
            gilrs.update(&ev);
            println!("{:?}", ev);
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
