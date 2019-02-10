extern crate gilrs_core;
#[cfg(target_platform = "wasm")]
#[macro_use]
extern crate stdweb;

#[cfg(target_platform = "wasm")]
use gilrs_core::Gilrs;

fn main() {
    #[cfg(target_platform = "wasm")]
    gamepad_loop(Gilrs::new().unwrap());
}

#[cfg(target_platform = "wasm")]
fn gamepad_loop(mut gilrs: Gilrs) {
    use stdweb::web::set_timeout;

    while let Some(ev) = gilrs.next_event() {
        console!(log, format!("{:?}", ev));
    }

    set_timeout(move || gamepad_loop(gilrs), 1000);
}
