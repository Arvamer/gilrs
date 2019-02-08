extern crate gilrs_core;
#[macro_use]
extern crate stdweb;

use gilrs_core::Gilrs;
use stdweb::web::set_timeout;

fn main() {
    gamepad_loop(Gilrs::new().unwrap());
}

fn gamepad_loop(mut gilrs: Gilrs) {
    while let Some(ev) = gilrs.next_event() {
        console!(log, format!("{:?}", ev));
    }
    set_timeout(move || gamepad_loop(gilrs), 1000);
}
