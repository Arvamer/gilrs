extern crate gilrs;
use gilrs::Gilrs;

fn main() {
    let mut gil = Gilrs::new();
    loop {
        gil.handle_hotplug();
        for e in gil.pool_events() {
            println!("{:?}", e);
        }
    }
}
