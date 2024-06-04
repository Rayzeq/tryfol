use gtk::{
    glib::{
        clone::{Downgrade, Upgrade},
        object::{Cast, IsA, ObjectExt},
        SignalHandlerId, WeakRef,
    },
    prelude::WidgetExt,
    Tooltip, Widget,
};
use gtk4 as gtk;

pub trait HasTooltip {
    fn set_better_tooltip(&self, tooltip: Option<String>);
    fn set_better_tooltip_markup(&self, tooltip: Option<String>);
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
unsafe fn set_tooltip_full<W>(widget: &W, tooltip_text: Option<String>, is_markup: bool)
where
    W: ObjectExt + WidgetExt + Downgrade,
{
    widget.set_has_tooltip(tooltip_text.is_some());
    if let Some(tooltip_text) = tooltip_text {
        // if there is already a tooltip, update it
        if let Some(tooltip) = get_tooltip_for_widget(widget) {
            if is_markup {
                tooltip.set_markup(Some(&tooltip_text));
            } else {
                tooltip.set_text(Some(&tooltip_text));
            }
        }

        widget.set_data("__rayzeq_tooltip_text", tooltip_text);
        widget.set_data("__rayzeq_tooltip_is_markup", is_markup);
    }

    // add a tooltip handler if we don't have one yet
    if widget
        .data::<SignalHandlerId>("__rayzeq_tooltip_handler")
        .is_none()
    {
        let handler_id = widget.connect_query_tooltip(|widget, _, _, _, tooltip| {
            let text: &String = widget.data("__rayzeq_tooltip_text").unwrap().as_ref();
            let is_markup: &bool = widget.data("__rayzeq_tooltip_is_markup").unwrap().as_ref();

            if *is_markup {
                tooltip.set_markup(Some(text));
            } else {
                tooltip.set_text(Some(text));
            }

            tooltip.set_data("__rayzeq_tooltip_owner", Downgrade::downgrade(&widget));
            widget.set_data("__rayzeq_tooltip_widget", Downgrade::downgrade(&tooltip));

            true
        });
        widget.set_data("__rayzeq_tooltip_handler", handler_id);
    }
}

/// Get the currently shown tooltip for a widget, if any.
unsafe fn get_tooltip_for_widget<W>(widget: &W) -> Option<Tooltip>
where
    W: ObjectExt + IsA<Widget>,
{
    let tooltip = Upgrade::upgrade(
        widget
            .data::<WeakRef<Tooltip>>("__rayzeq_tooltip_widget")?
            .as_ref(),
    )?;

    if let Some(owner) = get_tooltip_owner(&tooltip) {
        if widget.upcast_ref() != &owner {
            return None;
        }
    }

    Some(tooltip)
}

/// Get the current owner of a tooltip, if any.
///
/// We save the owner of tooltips because they can be re-used by gtk for different widgets.
unsafe fn get_tooltip_owner(tooltip: &Tooltip) -> Option<Widget> {
    Upgrade::upgrade(
        tooltip
            .data::<WeakRef<Widget>>("__rayzeq_tooltip_owner")?
            .as_ref(),
    )
}
