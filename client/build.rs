use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/images/icon.ico");
        // Sets the application name in Task Manager
        res.set("FileDescription", "Kakolookiyam");
        res.set("ProductName", "Kakolookiyam");
        res.compile().unwrap();
    }
}
