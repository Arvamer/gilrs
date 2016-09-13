extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();

    let mut counter = 0;

    loop {
        for event in gil.poll_events() {
            println!("{:?}", event);
        }

        if counter % 250 == 0 {
            for (id, gamepad) in gil.gamepads() {
                println!("Power info of gamepad {}({}): {:?}",
                         id,
                         gamepad.name(),
                         gamepad.power_info());
            }
        }

        counter += 1;
        thread::sleep(Duration::from_millis(33));
    }
}
