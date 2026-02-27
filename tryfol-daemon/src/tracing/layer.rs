use std::{fmt::Write, time::SystemTime};

use humantime::format_rfc3339_seconds;
use tracing::{
	Event, Level, Subscriber,
	field::{Field, Visit},
	span,
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use super::LogStore;

#[derive(Debug)]
pub struct ModuleLogLayer(pub(crate) LogStore);

struct ModuleNameExt(String);

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for ModuleLogLayer {
	fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
		let mut visitor = ModuleFieldVisitor(None);
		attrs.record(&mut visitor);

		if let Some(name) = visitor.0
			&& let Some(span) = ctx.span(id)
		{
			span.extensions_mut().insert(ModuleNameExt(name));
		}
	}

	fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
		let module_name = (|| {
			let mut span = ctx.lookup_current();
			while let Some(p) = span {
				if let Some(m) = p.extensions().get::<ModuleNameExt>() {
					return Some(m.0.clone());
				}
				span = p.parent();
			}

			None
		})();

		if let Some(module) = module_name {
			let mut visitor = EventVisitor(String::new());
			event.record(&mut visitor);

			let level_color = match *event.metadata().level() {
				Level::TRACE => "35",
				Level::DEBUG => "34",
				Level::INFO => "32",
				Level::WARN => "33",
				Level::ERROR => "31",
			};
			let line = format!(
				"[\x1b[90m{} \x1b[{level_color}m{:5}\x1b[0m] {}",
				format_rfc3339_seconds(SystemTime::now()),
				event.metadata().level(),
				visitor.0
			);
			self.0.push(module, line);
		}
	}
}

struct ModuleFieldVisitor(Option<String>);

impl Visit for ModuleFieldVisitor {
	fn record_str(&mut self, field: &Field, value: &str) {
		if field.name() == "module" {
			self.0 = Some(value.to_owned());
		}
	}

	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		if field.name() == "module" {
			self.0 = Some(format!("{value:?}"));
		}
	}
}

struct EventVisitor(String);

impl Visit for EventVisitor {
	fn record_str(&mut self, field: &Field, value: &str) {
		if !self.0.is_empty() {
			self.0.push(' ');
		}
		if field.name() == "message" {
			self.0.push_str(value);
		} else {
			write!(self.0, "{}={}", field.name(), value).ok();
		}
	}
	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		if !self.0.is_empty() {
			self.0.push(' ');
		}
		if field.name() == "message" {
			write!(self.0, "{value:?}").ok();
		} else {
			write!(self.0, "{}={:?}", field.name(), value).ok();
		}
	}
}
