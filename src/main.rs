use cairo::{self, Context, ImageSurface};
use gdk::{Cursor, CursorType, EventMask, ModifierType};
use gio::prelude::*;
use gtk::{prelude::*, DrawingArea, FileChooserExt, ResponseType, WidgetExt};
use std::f64::consts::PI;

use gdk::WindowExt;
use glib::clone;
use gtk::{Application, ApplicationWindow};
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

const H: i32 = 500;
const W: i32 = 500;

// draw a rectangle to interpolate between two cursor positions
fn interpolate(cr: &Context, x0: f64, y0: f64, x1: f64, y1: f64) {
    let ab = [x1 - x0, y1 - y0];
    let norm = (ab[0] * ab[0] + ab[1] * ab[1]).sqrt();
    let v = [-ab[1] / norm * 10.0, ab[0] / norm * 10.0];
    cr.move_to(x0 + v[0], y0 + v[1]);
    cr.line_to(x1 + v[0], y1 + v[1]);
    cr.line_to(x1 - v[0], y1 - v[1]);
    cr.line_to(x0 - v[0], y0 - v[1]);
    cr.fill();
}

fn main() {
    let application = Application::new(Some("com.github.maz3max.drawing-rs"), Default::default())
        .expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        // create and set up window
        let window = ApplicationWindow::new(app);
        window.set_title("Drawing Example");
        window.set_default_size(W, H);
        window.set_resizable(false);

        // a place to put the cursor position
        let pos = Rc::new(RefCell::new((100.0, 100.0)));
        // a surface to store the drawn image
        let surface =
            ImageSurface::create(cairo::Format::ARgb32, W, H).expect("Can't create surface");
        let surface_cr = Context::new(&surface);
        // fill the image with the background color
        surface_cr.set_source_rgb(0.015625, 0.39453125, 0.5078125);
        surface_cr.paint();

        // populate window with a drawing area which we will draw on
        let drawing_area = Box::new(DrawingArea::new)();
        // define what happens when the drawing area has to be (re-)drawn
        drawing_area.connect_draw(
            clone!(@strong pos, @strong surface => move |_drawing_area,cr|{
                let pos = pos.borrow();
                // draw the stored surface
                cr.set_source_surface(&surface, 0.0, 0.0);
                cr.paint();
                // draw the pointer
                cr.set_source_rgb(1.0,1.0,1.0);
                cr.arc(pos.0, pos.1, 10.0, 0.0, PI*2.0);
                cr.fill();
                Inhibit(false)
            }),
        );
        // subscribe to some mouse events
        drawing_area.add_events(
            EventMask::POINTER_MOTION_MASK // mouse hovering
                | EventMask::BUTTON_PRESS_MASK // needed for the events below
                | EventMask::BUTTON1_MOTION_MASK // mouse drag (left button)
                | EventMask::BUTTON3_MOTION_MASK, // mouse drag (right button)
        );

        // define what happens when button press events are triggered
        drawing_area.connect_button_press_event(
            clone!(@strong pos, @strong surface_cr => move|drawing_area, event_button|{
                let mut pos = pos.borrow_mut();
                *pos = event_button.get_position();
                let button = event_button.get_button();
                if button == 1 { // left mouse button
                    surface_cr.set_source_rgb(0.94921875, 0.56640625, 0.53515625);
                    surface_cr.arc(pos.0, pos.1, 10.0, 0.0, PI*2.0);
                    surface_cr.fill();
                }
                if button == 3 { //right mouse button
                    surface_cr.set_source_rgb(0.015625, 0.39453125, 0.5078125);
                    surface_cr.arc(pos.0, pos.1, 10.0, 0.0, PI*2.0);
                    surface_cr.fill();
                }
                drawing_area.queue_draw(); // force redraw of the drawing area
                Inhibit(false)
        }));
        
        // define what happens when motion events are triggered
        drawing_area.connect_motion_notify_event(
            clone!(@strong pos, @strong surface_cr => move |drawing_area, event_motion|{
                let old_pos = *pos.borrow();
                let mut pos = pos.borrow_mut();
                *pos = event_motion.get_position();
                let state = event_motion.get_state();
                if state.contains(ModifierType::BUTTON1_MASK) { // left mouse button
                    surface_cr.set_source_rgb(0.94921875, 0.56640625, 0.53515625);
                    surface_cr.arc(pos.0, pos.1, 10.0, 0.0, PI*2.0);
                    surface_cr.fill();
                    interpolate(&surface_cr, old_pos.0,old_pos.1,pos.0,pos.1);
                } else if state.contains(ModifierType::BUTTON3_MASK) { //right mouse button
                    surface_cr.set_source_rgb(0.015625, 0.39453125, 0.5078125);
                    surface_cr.arc(pos.0, pos.1, 10.0, 0.0, PI*2.0);
                    surface_cr.fill();
                    interpolate(&surface_cr, old_pos.0,old_pos.1,pos.0,pos.1);
                }
                drawing_area.queue_draw(); // force redraw of the drawing area
                Inhibit(false)
            }),
        );

        window.connect_realize(|app_window|{
            let gdk_window = app_window.get_window();
            // hide cursor if we can
            // also try to get more motion events if possible
            if let Some(gdk_window) = gdk_window {
                let cursor = Cursor::new(CursorType::BlankCursor);
                gdk_window.set_cursor(Some(&cursor));
                gdk_window.set_event_compression(false);
            }
        });

        // save on ctrl-s
        window.add_events(EventMask::KEY_PRESS_MASK);
        window.connect_key_press_event(
            clone!(@strong surface, @strong window => move |_, event_key| {
                if event_key.get_state().contains(ModifierType::CONTROL_MASK) {
                    if let Some(keyname) = event_key.get_keyval().name() {
                        if keyname == "s" {
                            // build file_chooser
                            let file_chooser = gtk::FileChooserDialog::new(
                                Some("Save Drawing"),
                                Some(&window),
                                gtk::FileChooserAction::Save,
                            );
                            file_chooser.add_buttons(&[
                                ("Cancel", gtk::ResponseType::Cancel),
                                ("Save", gtk::ResponseType::Ok),
                            ]);
                            file_chooser.set_current_name("drawing.png");
                            file_chooser.connect_response(
                                clone!(@strong surface => move |dialog,response| {
                                if response == ResponseType::Ok {
                                    if let Some(filename) = dialog.get_filename() {
                                        println!("saving under {:?}", filename);
                                        let mut file = File::create(filename).expect("Couldn't create file. Sorry.");
                                        surface.write_to_png(&mut file).expect("Couldn't save drawing. Sorry.");
                                    }
                                }
                                dialog.close();
                            }));
                            file_chooser.show_all();
                        }
                    }
                }
                Inhibit(false)
            }),
        );

        window.add(&drawing_area);
        window.show_all();
    });
    application.run(&[]);
}
