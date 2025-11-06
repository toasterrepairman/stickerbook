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

    // Create the image/animation display using gtk::Picture
    let picture = gtk::Picture::new();
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Contain);

    // Set minimum size for the picture widget
    picture.set_size_request(25, 25);

    // Variables to store aspect ratio
    let mut aspect_ratio = 1.0_f32;
    let mut has_image = false;

    // Load the image or animated GIF using PixbufAnimation
    if let Ok(animation) = PixbufAnimation::from_file(image_path) {
        let pixbuf = if animation.is_static_image() {
            animation.static_image()
        } else {
            Some(animation.iter(None).pixbuf())
        };

        // Get aspect ratio and set window size
        if let Some(pixbuf) = &pixbuf {
            let width = pixbuf.width();
            let height = pixbuf.height();
            aspect_ratio = width as f32 / height as f32;
            has_image = true;

            // Set initial window size based on image dimensions (minimum 25x25)
            let initial_width = width.max(25);
            let initial_height = height.max(25);
            window.set_default_size(initial_width, initial_height);
        }

        if animation.is_static_image() {
            // For static images, just set the pixbuf
            if let Some(pixbuf) = pixbuf {
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

    // Wrap picture in AspectFrame to maintain aspect ratio during resize
    let aspect_frame = gtk::AspectFrame::builder()
        .ratio(aspect_ratio)
        .obey_child(false)
        .build();
    aspect_frame.set_child(Some(&picture));

    // Create overlay for headerbar
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&aspect_frame));

    // Create headerbar
    let headerbar = adw::HeaderBar::new();
    headerbar.add_css_class("osd");
    headerbar.set_show_title(false);
    headerbar.set_show_start_title_buttons(false);
    headerbar.set_show_end_title_buttons(false);

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

    // Add Ctrl+W accelerator to close window
    let close_action = gio::SimpleAction::new("close", None);
    let window_close = window.clone();
    close_action.connect_activate(move |_, _| {
        window_close.close();
    });
    window.add_action(&close_action);
    app.set_accels_for_action("win.close", &["<Control>w"]);

    window.set_child(Some(&overlay));

    // Apply CSS for transparency and rounded corners
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_string(
        r#"
        .transparent-window,
        .transparent-window:backdrop {
            background: transparent;
            box-shadow: none;
            border: none;
        }

        /* Remove any default window decorations/shadows */
        .transparent-window decoration,
        .transparent-window:backdrop decoration {
            background: transparent;
            box-shadow: none;
            border: none;
        }

        /* Make headerbar rounded and semi-transparent for both active and backdrop states */
        .transparent-window headerbar.osd,
        .transparent-window:backdrop headerbar.osd {
            border-radius: 12px !important;
            background: alpha(@headerbar_bg_color, 0.9);
            backdrop-filter: blur(10px);
            margin: 6px;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
            overflow: hidden;
        }

        /* Ensure headerbar children respect rounded corners */
        .transparent-window headerbar.osd > *,
        .transparent-window:backdrop headerbar.osd > * {
            border-radius: 12px;
        }

        /* Clip content to rounded corners */
        .transparent-window headerbar.osd windowhandle,
        .transparent-window:backdrop headerbar.osd windowhandle {
            border-radius: 12px;
        }

        /* Ensure revealer doesn't add extra styling */
        .transparent-window revealer,
        .transparent-window:backdrop revealer {
            background: transparent;
        }
        "#,
    );

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to display"),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );

    window.present();
    window
}
