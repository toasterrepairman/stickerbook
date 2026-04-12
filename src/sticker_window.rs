use gdk_pixbuf::PixbufAnimation;
use gtk::prelude::*;
use gtk::{gdk, glib, Application};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

pub fn create_sticker_window(app: &Application, image_path: &str) -> gtk::ApplicationWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .default_width(400)
        .default_height(400)
        .decorated(false)
        .resizable(false)
        .build();

    // Make window background transparent
    window.add_css_class("transparent-window");

    // Create the image/animation display using gtk::Picture
    let picture = gtk::Picture::new();
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Cover);

    // Set minimum size for the picture widget
    picture.set_size_request(25, 25);

    // Variables to store aspect ratio
    let mut aspect_ratio = 1.0_f32;
    let mut has_image = false;
    let mut image_width = 400;
    let mut image_height = 400;

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
            image_width = width.max(25);
            image_height = height.max(25);

            // Set initial window size based on image dimensions
            window.set_default_size(image_width, image_height);
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

    // Create popover with controls
    let popover = gtk::Popover::new();
    let popover_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    popover_box.set_margin_start(6);
    popover_box.set_margin_end(6);
    popover_box.set_margin_top(6);
    popover_box.set_margin_bottom(6);

    // Store rotation and scale state
    let rotation_angle = Rc::new(RefCell::new(0));
    let scale = Rc::new(RefCell::new(1.0_f32));
    let rotation_angle_clone = rotation_angle.clone();
    let scale_clone = scale.clone();
    let picture_clone = picture.clone();
    let window_clone = window.clone();

    // Rotate button in popover
    let rotate_button = gtk::Button::builder()
        .icon_name("object-rotate-right-symbolic")
        .tooltip_text("Rotate 90°")
        .build();

    rotate_button.connect_clicked(move |_| {
        let mut angle = rotation_angle_clone.borrow_mut();
        *angle = (*angle + 90) % 360;

        let css_provider = gtk::CssProvider::new();
        let rotation_css = format!(
            r#"
            picture {{
                transform: rotate({}deg);
                transition: transform 200ms ease-in-out;
            }}
            "#,
            *angle
        );
        css_provider.load_from_string(&rotation_css);

        picture_clone
            .style_context()
            .add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);

        if has_image {
            let s = scale_clone.borrow();
            let ratio = if *angle == 90 || *angle == 270 {
                image_height as f32 / image_width as f32
            } else {
                image_width as f32 / image_height as f32
            };
            let w = (image_width as f32 * *s).max(25.0) as i32;
            let h = (w as f32 / ratio).max(25.0) as i32;
            window_clone.set_default_size(w, h);
        }
    });

    // Close button in popover
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close")
        .build();

    let window_close = window.clone();
    close_button.connect_clicked(move |_| {
        window_close.close();
    });

    popover_box.append(&rotate_button);
    popover_box.append(&close_button);
    popover.set_child(Some(&popover_box));
    popover.set_parent(&aspect_frame);

    // Right-click gesture to show popover
    let right_click = gtk::GestureClick::new();
    right_click.set_button(3);

    let popover_show = popover.clone();
    right_click.connect_pressed(move |_, _, x, y| {
        popover_show.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover_show.popup();
    });

    aspect_frame.add_controller(right_click);

    // Double-click gesture for rotate
    let double_click = gtk::GestureClick::new();
    double_click.set_button(1);

    let rotate_btn = rotate_button.clone();
    double_click.connect_pressed(move |_, n_press, _, _| {
        if n_press == 2 {
            rotate_btn.emit_clicked();
        }
    });

    aspect_frame.add_controller(double_click);

    // Scroll-to-scale gesture
    let scale_clone = scale.clone();
    let aspect_ratio_for_scale = aspect_ratio;
    let rotation_angle_for_scale = rotation_angle.clone();

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);

    let scroll_window = window.clone();
    scroll.connect_scroll(move |_, _, dy| {
        let mut s = scale_clone.borrow_mut();
        let delta = (-dy as f32 * 0.1).clamp(-0.2, 0.2);
        *s = (*s + delta).clamp(0.2, 5.0);

        let angle = *rotation_angle_for_scale.borrow();
        let ratio = if angle == 90 || angle == 270 {
            1.0 / aspect_ratio_for_scale
        } else {
            aspect_ratio_for_scale
        };

        let w = (image_width as f32 * *s).max(25.0) as i32;
        let h = (w as f32 / ratio).max(25.0) as i32;
        scroll_window.set_default_size(w, h);

        glib::Propagation::Proceed
    });

    aspect_frame.add_controller(scroll);

    // Drag gesture to move the window
    let drag = gtk::GestureDrag::new();

    let drag_window = window.clone();
    drag.connect_drag_begin(move |gesture, start_x, start_y| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        if let Some(surface) = drag_window.surface() {
            if let Ok(toplevel) = surface.downcast::<gdk::Toplevel>() {
                let device = gesture.device().unwrap();
                toplevel.begin_move(
                    &device,
                    gesture.current_button() as i32,
                    start_x,
                    start_y,
                    gesture.current_event_time(),
                );
            }
        }
    });

    aspect_frame.add_controller(drag);

    // Add Ctrl+W accelerator to close window
    let close_action = gio::SimpleAction::new("close", None);
    let window_close = window.clone();
    close_action.connect_activate(move |_, _| {
        window_close.close();
    });
    window.add_action(&close_action);
    app.set_accels_for_action("win.close", &["<Control>w"]);

    window.set_child(Some(&aspect_frame));

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
