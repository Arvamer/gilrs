extern crate gilrs_core;

use gilrs_core::Gilrs;

fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    loop {
        while let Some(ev) = gilrs.next_event() {
            println!("{:?}", ev);
        }
    }
}
