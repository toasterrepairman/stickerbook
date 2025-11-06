use gtk::prelude::*;
use gtk::{gdk, glib, Application};
use libadwaita as adw;
use gdk_pixbuf::PixbufAnimation;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

pub fn create_sticker_window(app: &Application, image_path: &str) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .default_width(400)
        .default_height(400)
        .decorated(false)
        .build();

    // Make window background transparent
    window.add_css_class("transparent-window");

    // Create overlay for headerbar
    let overlay = gtk::Overlay::new();

    // Create the image/animation display using gtk::Picture
    let picture = gtk::Picture::new();
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Contain);

    // Load the image or animated GIF using PixbufAnimation
    if let Ok(animation) = PixbufAnimation::from_file(image_path) {
        if animation.is_static_image() {
            // For static images, just set the pixbuf
            if let Some(pixbuf) = animation.static_image() {
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                picture.set_paintable(Some(&texture));
            }
        } else {
            // For animated images, set up frame animation
            let iter = animation.iter(None);
            let iter_rc = Rc::new(RefCell::new(iter));
            let picture_clone = picture.clone();

            // Set initial frame
            let pixbuf = iter_rc.borrow().pixbuf();
            let texture = gdk::Texture::for_pixbuf(&pixbuf);
            picture.set_paintable(Some(&texture));

            // Animate frames
            glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let iter = iter_rc.borrow_mut();
                iter.advance(SystemTime::now());
                let pixbuf = iter.pixbuf();
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                picture_clone.set_paintable(Some(&texture));
                glib::ControlFlow::Continue
            });
        }
    } else {
        // Fallback to filename if loading fails
        picture.set_filename(Some(image_path));
    }

    overlay.set_child(Some(&picture));

    // Create headerbar
    let headerbar = adw::HeaderBar::new();
    headerbar.add_css_class("osd");
    headerbar.set_show_title(false);

    // Add close button
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .build();

    let window_clone = window.clone();
    close_button.connect_clicked(move |_| {
        window_clone.close();
    });

    headerbar.pack_end(&close_button);

    // Wrap headerbar in a revealer for auto-hide behavior
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(200)
        .reveal_child(false)
        .valign(gtk::Align::Start)
        .build();

    revealer.set_child(Some(&headerbar));
    overlay.add_overlay(&revealer);

    // Set up motion controller for showing/hiding headerbar
    let motion_controller = gtk::EventControllerMotion::new();

    let revealer_show = revealer.clone();
    motion_controller.connect_enter(move |_, _, _| {
        revealer_show.set_reveal_child(true);
    });

    let revealer_hide = revealer.clone();
    motion_controller.connect_leave(move |_| {
        revealer_hide.set_reveal_child(false);
    });

    window.add_controller(motion_controller);

    window.set_child(Some(&overlay));

    // Apply CSS for transparency
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_string(
        r#"
        .transparent-window {
            background: transparent;
        }
        "#,
    );

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to display"),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window.present();
    window
}
