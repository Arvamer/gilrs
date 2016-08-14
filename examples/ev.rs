extern crate gilrs;
extern crate env_logger;
use gilrs::Gilrs;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();
    loop {
        for e in gil.pool_events() {
            println!("{:?}", e);
        }
    }
}
