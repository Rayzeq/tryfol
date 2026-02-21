use gtk::{
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, EventSequenceState,
    GestureClick, Tooltip, Widget, gdk,
    glib::Propagation,
    glib::{
        self, SignalHandlerId, WeakRef, clone,
        clone::{Downgrade, Upgrade},
        object::{Cast, IsA, ObjectExt},
    },
    prelude::*,
};
use gtk4 as gtk;

pub trait Hoverable {
    fn connect_hover_notify<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, bool) + 'static + Clone;
}

pub trait Clickable {
    fn connect_clicked<F>(&self, button: u32, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, i32, f64, f64) + 'static;
    fn connect_left_clicked<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, i32, f64, f64) + 'static;
    fn connect_middle_clicked<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, i32, f64, f64) + 'static;
    fn connect_right_clicked<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, i32, f64, f64) + 'static;
}

pub trait Scrollable {
    fn connect_horizontal_scroll<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, f64) + 'static;
    fn connect_vertical_scroll<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, f64) + 'static;
    fn connect_both_scroll<F>(&self, callback: F)
    where
        Self: Downgrade,
        Self::Weak: Upgrade,
        F: for<'a> Fn(&'a <Self::Weak as Upgrade>::Strong, f64, f64) + 'static;
}

pub trait HasTooltip {
    fn set_better_tooltip(&self, tooltip: Option<String>);
    fn set_better_tooltip_markup(&self, tooltip: Option<String>);
}

impl<W> Hoverable for W
where
    W: WidgetExt + Downgrade,
    W::Weak: Upgrade,
{
    fn connect_hover_notify<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, bool) + 'static + Clone,
    {
        let motion_controller = EventControllerMotion::new();
        motion_controller.connect_enter(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            callback,
            move |_, _, _| callback(&this, true)
        ));
        motion_controller.connect_leave(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                callback(&this, false);
            }
        ));
        self.add_controller(motion_controller);
    }
}

impl<W> Clickable for W
where
    W: WidgetExt + Downgrade,
    W::Weak: Upgrade,
{
    fn connect_clicked<F>(&self, button: u32, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, i32, f64, f64) + 'static,
    {
        let gesture_controller = GestureClick::builder().button(button).build();
        gesture_controller.connect_pressed(clone!(
            #[weak(rename_to = this)]
            self,
            move |gesture, press_count, x, y| {
                gesture.set_state(EventSequenceState::Claimed);
                callback(&this, press_count, x, y);
            }
        ));
        self.add_controller(gesture_controller);
    }

    fn connect_left_clicked<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, i32, f64, f64) + 'static,
    {
        self.connect_clicked(gdk::BUTTON_PRIMARY, callback);
    }

    fn connect_middle_clicked<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, i32, f64, f64) + 'static,
    {
        self.connect_clicked(gdk::BUTTON_MIDDLE, callback);
    }

    fn connect_right_clicked<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, i32, f64, f64) + 'static,
    {
        self.connect_clicked(gdk::BUTTON_SECONDARY, callback);
    }
}

impl<W> Scrollable for W
where
    W: WidgetExt + Downgrade,
    W::Weak: Upgrade,
{
    fn connect_horizontal_scroll<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, f64) + 'static,
    {
        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::HORIZONTAL);
        scroll_controller.connect_scroll(clone!(
            #[weak(rename_to = this)]
            self,
            #[upgrade_or]
            Propagation::Proceed,
            move |_, dx, _| {
                callback(&this, dx);
                Propagation::Stop
            }
        ));
        self.add_controller(scroll_controller);
    }

    fn connect_vertical_scroll<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, f64) + 'static,
    {
        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(clone!(
            #[weak(rename_to = this)]
            self,
            #[upgrade_or]
            Propagation::Proceed,
            move |_, _, dy| {
                callback(&this, dy);
                Propagation::Stop
            }
        ));
        self.add_controller(scroll_controller);
    }

    fn connect_both_scroll<F>(&self, callback: F)
    where
        F: for<'a> Fn(&'a <W::Weak as Upgrade>::Strong, f64, f64) + 'static,
    {
        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);
        scroll_controller.connect_scroll(clone!(
            #[weak(rename_to = this)]
            self,
            #[upgrade_or]
            Propagation::Proceed,
            move |_, dx, dy| {
                callback(&this, dx, dy);
                Propagation::Stop
            }
        ));
        self.add_controller(scroll_controller);
    }
}

impl<W> HasTooltip for W
where
    W: ObjectExt + WidgetExt + Downgrade,
{
    fn set_better_tooltip(&self, tooltip: Option<String>) {
        unsafe {
            set_tooltip_full(self, tooltip, false);
        }
    }

    fn set_better_tooltip_markup(&self, tooltip: Option<String>) {
        unsafe {
            set_tooltip_full(self, tooltip, true);
        }
    }
}

// unsafe because I use a lot widget.data and widget.set_data
unsafe fn set_tooltip_full<W>(widget: &W, mut tooltip_text: Option<String>, is_markup: bool)
where
    W: ObjectExt + WidgetExt + Downgrade,
{
    if matches!(tooltip_text.as_deref(), Some("")) {
        tooltip_text = None;
    }

    widget.set_has_tooltip(tooltip_text.is_some());
    if let Some(tooltip_text) = tooltip_text {
        // if there is already a tooltip, update it
        if let Some(tooltip) = unsafe { get_tooltip_for_widget(widget) } {
            if is_markup {
                tooltip.set_markup(Some(&tooltip_text));
            } else {
                tooltip.set_text(Some(&tooltip_text));
            }
        }

        unsafe {
            widget.set_data("__rayzeq_tooltip_text", tooltip_text);
            widget.set_data("__rayzeq_tooltip_is_markup", is_markup);
        }
    }

    // add a tooltip handler if we don't have one yet
    if unsafe { widget.data::<SignalHandlerId>("__rayzeq_tooltip_handler") }.is_none() {
        let handler_id = widget.connect_query_tooltip(|widget, _, _, _, tooltip| {
            let (text, is_markup): (&String, &bool) = unsafe {
                (
                    widget.data("__rayzeq_tooltip_text").unwrap().as_ref(),
                    widget.data("__rayzeq_tooltip_is_markup").unwrap().as_ref(),
                )
            };

            if *is_markup {
                tooltip.set_markup(Some(text));
            } else {
                tooltip.set_text(Some(text));
            }

            unsafe {
                tooltip.set_data("__rayzeq_tooltip_owner", Downgrade::downgrade(&widget));
                widget.set_data("__rayzeq_tooltip_widget", Downgrade::downgrade(&tooltip));
            }

            true
        });
        unsafe {
            widget.set_data("__rayzeq_tooltip_handler", handler_id);
        }
    }
}

/// Get the currently shown tooltip for a widget, if any.
unsafe fn get_tooltip_for_widget<W>(widget: &W) -> Option<Tooltip>
where
    W: ObjectExt + IsA<Widget>,
{
    let tooltip = Upgrade::upgrade(unsafe {
        widget
            .data::<WeakRef<Tooltip>>("__rayzeq_tooltip_widget")?
            .as_ref()
    })?;

    if let Some(owner) = unsafe { get_tooltip_owner(&tooltip) }
        && widget.upcast_ref() != &owner
    {
        return None;
    }

    Some(tooltip)
}

/// Get the current owner of a tooltip, if any.
///
/// We save the owner of tooltips because they can be re-used by gtk for different widgets.
unsafe fn get_tooltip_owner(tooltip: &Tooltip) -> Option<Widget> {
    Upgrade::upgrade(unsafe {
        tooltip
            .data::<WeakRef<Widget>>("__rayzeq_tooltip_owner")?
            .as_ref()
    })
}
