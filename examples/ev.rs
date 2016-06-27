extern crate gilrs;
use gilrs::Gilrs;

fn main() {
    let mut gil = Gilrs::new();
    loop {
        for e in gil.pool_events() {
            println!("{:?}", e);
        }
    }
}
