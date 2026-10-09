#![forbid(unsafe_code)]

fn main() {
    runalytics_desktop_lib::run().expect("Runalytics failed to start");
}
