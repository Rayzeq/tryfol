use std::cmp::max;

use gtk::{
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
    Allocation, Orientation, Widget,
};
use gtk4 as gtk;

// Object holding the state
#[derive(Default)]
pub struct StopFuckingWithMyClockLayoutImpl;

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for StopFuckingWithMyClockLayoutImpl {
    const NAME: &'static str = "StopFuckingWithMyClockLayout";
    type Type = StopFuckingWithMyClockLayout;
    type ParentType = gtk::LayoutManager;
}

impl ObjectImpl for StopFuckingWithMyClockLayoutImpl {}

impl LayoutManagerImpl for StopFuckingWithMyClockLayoutImpl {
    fn allocate(&self, widget: &gtk::Widget, mut width: i32, height: i32, baseline: i32) {
        let left = widget
            .first_child()
            .expect("custom layout must have 3 wigets");
        let center = left
            .next_sibling()
            .expect("custom layout must have 3 wigets");
        let right = center
            .next_sibling()
            .expect("custom layout must have 3 wigets");

        let (_, center_width, _, _) = center.measure(Orientation::Horizontal, -1);

        let allocation = Allocation::new(width / 2 - center_width / 2, 0, center_width, height);
        center.size_allocate(&allocation, baseline);
        width -= center_width;

        let allocation = Allocation::new(0, 0, width / 2, height);
        left.size_allocate(&allocation, baseline);
        let allocation = Allocation::new(width / 2 + center_width, 0, width / 2, height);
        right.size_allocate(&allocation, baseline);
    }

    fn create_layout_child(
        &self,
        widget: &gtk::Widget,
        for_child: &gtk::Widget,
    ) -> gtk::LayoutChild {
        self.parent_create_layout_child(widget, for_child)
    }

    fn layout_child_type() -> Option<glib::Type> {
        None
    }

    fn request_mode(&self, widget: &gtk::Widget) -> gtk::SizeRequestMode {
        self.parent_request_mode(widget)
    }

    fn measure(
        &self,
        widget: &Widget,
        orientation: Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let mut minimum = 0;
        let mut natural = 0;

        let left = widget
            .first_child()
            .expect("custom layout must have 3 wigets");
        let center = left
            .next_sibling()
            .expect("custom layout must have 3 wigets");
        let right = center
            .next_sibling()
            .expect("custom layout must have 3 wigets");

        for child in [left, center, right] {
            let (child_min, child_nat, _, _) = child.measure(orientation, for_size);
            if orientation == Orientation::Horizontal {
                minimum += child_min;
                natural += child_nat;
            } else {
                minimum = max(minimum, child_min);
                natural = max(natural, child_nat);
            }
        }

        (minimum, natural, -1, -1)
    }

    fn root(&self) {
        self.parent_root();
    }

    fn unroot(&self) {
        self.parent_unroot();
    }
}

glib::wrapper! {
    pub struct StopFuckingWithMyClockLayout(ObjectSubclass<StopFuckingWithMyClockLayoutImpl>)
        @extends gtk::LayoutManager;
}

impl StopFuckingWithMyClockLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
