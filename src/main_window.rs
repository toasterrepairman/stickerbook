use gtk::prelude::*;
use gtk::{gdk, glib, Application};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;
use std::time::SystemTime;
use gdk_pixbuf::PixbufAnimation;

use crate::recent_store::RecentStore;
use crate::sticker_window;

pub fn create_main_window(app: &Application, recent_store: Rc<RefCell<RecentStore>>) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Stickerbook")
        .default_width(800)
        .default_height(600)
        .build();

    // Track child windows
    let child_windows: Rc<RefCell<Vec<gtk::ApplicationWindow>>> = Rc::new(RefCell::new(Vec::new()));

    // Close all child windows when main window closes
    let child_windows_close = child_windows.clone();
    window.connect_close_request(move |_| {
        for child in child_windows_close.borrow().iter() {
            child.close();
        }
        glib::Propagation::Proceed
    });

    // Add Ctrl+Q accelerator to quit
    let quit_action = gio::SimpleAction::new("quit", None);
    let window_quit = window.clone();
    quit_action.connect_activate(move |_, _| {
        window_quit.close();
    });
    window.add_action(&quit_action);
    app.set_accels_for_action("win.quit", &["<Control>q"]);

    // Create headerbar
    let headerbar = adw::HeaderBar::new();

    // Add file chooser button
    let add_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Add sticker")
        .build();

    headerbar.pack_start(&add_button);

    // Create toolbar view
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&headerbar);

    // Create scrolled window for recent items
    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    // Create grid for recent items (supports multiple rows)
    let recent_grid = gtk::Grid::builder()
        .row_spacing(12)
        .column_spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .vexpand(true)
        .valign(gtk::Align::Fill)
        .build();

    scrolled.set_child(Some(&recent_grid));
    toolbar_view.set_content(Some(&scrolled));

    // Track number of rows to display based on window height
    let max_rows = Rc::new(RefCell::new(2));

    // Load and display recent items
    refresh_recent_items(&recent_grid, app, recent_store.clone(), child_windows.clone(), *max_rows.borrow());

    // Set up window resize handler to adapt row count
    let max_rows_resize = max_rows.clone();
    let recent_grid_resize = recent_grid.clone();
    let recent_store_resize = recent_store.clone();
    let app_resize = app.clone();
    let child_windows_resize = child_windows.clone();

    window.connect_default_height_notify(move |win| {
        let height = win.default_height();
        let new_max_rows = if height < 400 { 1 } else { 2 };
        let current_max_rows = *max_rows_resize.borrow();

        if new_max_rows != current_max_rows {
            *max_rows_resize.borrow_mut() = new_max_rows;
            refresh_recent_items(
                &recent_grid_resize,
                &app_resize,
                recent_store_resize.clone(),
                child_windows_resize.clone(),
                new_max_rows
            );
        }
    });

    // Set up file chooser
    let recent_store_clone = recent_store.clone();
    let app_clone = app.clone();
    let recent_grid_clone = recent_grid.clone();
    let child_windows_clone = child_windows.clone();
    let max_rows_clone = max_rows.clone();
    add_button.connect_clicked(move |button| {
        let dialog = gtk::FileDialog::builder()
            .title("Select Image or GIF")
            .modal(true)
            .build();

        let filter = gtk::FileFilter::new();
        filter.add_mime_type("image/*");
        filter.set_name(Some("Images"));

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));

        let window = button
            .root()
            .and_then(|root| root.downcast::<gtk::Window>().ok());

        let recent_store = recent_store_clone.clone();
        let app = app_clone.clone();
        let recent_grid = recent_grid_clone.clone();
        let child_windows = child_windows_clone.clone();
        let max_rows = max_rows_clone.clone();

        dialog.open(window.as_ref(), gtk::gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let path_str = path.to_string_lossy().to_string();
                    recent_store.borrow_mut().add(path_str);
                    let _ = recent_store.borrow().save();
                    refresh_recent_items(&recent_grid, &app, recent_store.clone(), child_windows.clone(), *max_rows.borrow());
                }
            }
        });
    });

    window.set_content(Some(&toolbar_view));
    window.present();
}

fn refresh_recent_items(
    container: &gtk::Grid,
    app: &Application,
    recent_store: Rc<RefCell<RecentStore>>,
    child_windows: Rc<RefCell<Vec<gtk::ApplicationWindow>>>,
    max_rows: i32,
) {
    // Clear existing items
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let items = recent_store.borrow().items().to_vec();

    let mut col = 0;
    let mut row = 0;

    for item in items {
        if !Path::new(&item.path).exists() {
            continue;
        }

        let item_overlay = gtk::Overlay::new();

        // Create picture for the sticker thumbnail (supports animations)
        let picture = gtk::Picture::builder()
            .width_request(150)
            .height_request(150)
            .can_shrink(true)
            .vexpand(true)
            .hexpand(false)
            .content_fit(gtk::ContentFit::Cover)
            .build();

        // Load thumbnail - supports both static and animated images
        if let Ok(animation) = PixbufAnimation::from_file(&item.path) {
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

                // Animate frames (thumbnails animate slower to reduce CPU usage)
                glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
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
            picture.set_filename(Some(&item.path));
        }

        // Make picture clickable
        let gesture = gtk::GestureClick::new();
        let app_clone = app.clone();
        let path_clone = item.path.clone();
        let recent_store_click = recent_store.clone();
        let child_windows_click = child_windows.clone();
        gesture.connect_released(move |_, _, _, _| {
            recent_store_click.borrow_mut().add(path_clone.clone());
            let _ = recent_store_click.borrow().save();
            let child_window = sticker_window::create_sticker_window(&app_clone, &path_clone);
            child_windows_click.borrow_mut().push(child_window);
        });
        picture.add_controller(gesture);

        item_overlay.set_child(Some(&picture));

        // Create remove button overlay
        let remove_button = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_top(6)
            .margin_end(6)
            .build();

        remove_button.add_css_class("osd");
        remove_button.add_css_class("circular");

        let recent_store_remove = recent_store.clone();
        let container_clone = container.clone();
        let app_clone = app.clone();
        let child_windows_remove = child_windows.clone();
        let path_for_remove = item.path.clone();
        remove_button.connect_clicked(move |_| {
            recent_store_remove.borrow_mut().remove(&path_for_remove);
            let _ = recent_store_remove.borrow().save();
            refresh_recent_items(&container_clone, &app_clone, recent_store_remove.clone(), child_windows_remove.clone(), max_rows);
        });

        item_overlay.add_overlay(&remove_button);

        // Add frame for better appearance
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&item_overlay));
        frame.set_vexpand(true);
        frame.set_valign(gtk::Align::Fill);

        // Add to grid with row/column layout
        container.attach(&frame, col, row, 1, 1);

        // Update position for next item
        row += 1;
        if row >= max_rows {
            row = 0;
            col += 1;
        }
    }
}
