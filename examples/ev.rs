extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ev::State;
use gilrs::ev::filter::{Filter, Noise, Repeat};

use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();
    let mut state = State::new();
    let noise_filter = Noise::new();
    let repeat_filter = Repeat {
        after: Duration::from_millis(1000),
        every: Duration::from_millis(50),
    };

    let mut counter = 0;

    loop {
        while let Some(ev) = gil.next_event().filter(&noise_filter, &state).filter(
            &repeat_filter,
            &state,
        )
        {
            state.update(&ev);
            println!("{:?}", ev);
        }

        if counter % 1000 == 0 {
            for (id, gamepad) in gil.gamepads() {
                println!(
                    "Power info of gamepad {}({}): {:?}",
                    id,
                    gamepad.name(),
                    gamepad.power_info()
                );
            }
        }

        counter += 1;
        thread::sleep(Duration::from_millis(10));
    }
}
