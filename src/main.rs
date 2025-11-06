mod main_window;
mod recent_store;
mod sticker_window;

use gtk::prelude::*;
use gtk::{glib, Application};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use recent_store::RecentStore;

const APP_ID: &str = "com.github.toasterrepair.Stickerbook";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        adw::init().expect("Failed to initialize libadwaita");
    });

    let recent_store = Rc::new(RefCell::new(RecentStore::load()));

    app.connect_activate(move |app| {
        main_window::create_main_window(app, recent_store.clone());
    });

    app.run()
}
