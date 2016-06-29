extern crate gilrs;

use gilrs::Gilrs;

fn main() {
    let mut gilrs = Gilrs::new();

    loop {
        for _ in gilrs.pool_events() {}
        // Clear
        print!("{}[2J", 0o33 as char);
        println!("{:#?}", gilrs.gamepad(0).unwrap().state());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
