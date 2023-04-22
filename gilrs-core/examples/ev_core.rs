use gilrs_core::Gilrs;

fn main() {
    env_logger::init();

    let mut gilrs = Gilrs::new().unwrap();
    loop {
        while let Some(ev) = gilrs.next_event_blocking(None) {
            println!("{:?}", ev);
        }
    }
}
