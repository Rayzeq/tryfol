use std::pin::Pin;

use tokio_util::sync::CancellationToken;

pub trait Module {
	fn name() -> &'static str
	where
		Self: Sized;

	fn run(
		&self,
		token: CancellationToken,
	) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
}
