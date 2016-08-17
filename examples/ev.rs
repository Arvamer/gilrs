extern crate gilrs;
extern crate env_logger;
use gilrs::Gilrs;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();
    loop {
        for e in gil.poll_events() {
            println!("{:?}", e);
        }
    }
}
