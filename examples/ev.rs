extern crate env_logger;
extern crate gilrs;

use gilrs::GilrsBuilder;
use gilrs::ev::filter::{Filter, Repeat};

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = GilrsBuilder::new()
        .add_env_mappings()
        .add_included_mappings()
        .set_update_state(false)
        .build();
    let repeat_filter = Repeat::new();

    loop {
        while let Some(ev) = gilrs.next_event().filter_ev(&repeat_filter, &gilrs) {
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
