extern crate gamepad;
use gamepad::Gamepads;

fn main() {
    let mut gamepads = Gamepads::new();
    loop {
        for e in gamepads.pool_events() {
            println!("{:?}", e);
        }
    }
}
