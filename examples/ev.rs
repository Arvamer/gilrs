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
        for e in gil.poll_events() {
            println!("{:?}", e);
        }

        if counter % 100 == 0 {
            println!("Power info: {:?}",  gil.gamepad(0).power_info());
        }

        counter += 1;
        thread::sleep(Duration::from_millis(33));
    }
}
