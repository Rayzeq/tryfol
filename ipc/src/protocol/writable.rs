use anyhow::Context;

use crate::Write;

pub trait Writable<T>: Write {}

impl<T: Write> Writable<T> for T {}

impl<T: Write + Sync> Writable<T> for &T {}

impl Writable<String> for &str {}

impl<T> Writable<Vec<T>> for &[T]
where
    T: Write + Sync,
    anyhow::Error: From<T::Error>,
    Result<(), T::Error>: Context<(), T::Error>,
{
}

impl<T> Writable<Box<T>> for &[T]
where
    T: Write + Sync,
    anyhow::Error: From<T::Error>,
    Result<(), T::Error>: Context<(), T::Error>,
{
}
