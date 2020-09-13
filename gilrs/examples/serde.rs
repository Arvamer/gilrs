use toml;
use gilrs::Button;

fn main() {
    let button = Button::DPadDown;

    let data = toml::to_string(&button).expect("failed to encode");
    println!("{}",  data)
}
