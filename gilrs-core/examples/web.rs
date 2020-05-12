#[cfg(target_arch = "wasm32")]
use gilrs_core::Gilrs;

fn main() {
    #[cfg(target_arch = "wasm32")]
    gamepad_loop(Gilrs::new().unwrap());
}

#[cfg(target_arch = "wasm32")]
fn gamepad_loop(mut gilrs: Gilrs) {
    use stdweb::web::set_timeout;

    while let Some(ev) = gilrs.next_event() {
        stdweb::console!(log, format!("{:?}", ev));
    }

    set_timeout(move || gamepad_loop(gilrs), 50);
}
