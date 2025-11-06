//! Read / Write traits to send values over IPC

use anyhow::Context;
use std::{borrow::Cow, future::Future, io};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub trait Read {
    type Error;

    fn read(
        stream: &mut (impl AsyncRead + Unpin + Send),
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send
    where
        Self: Sized;
}

pub trait Write {
    type Error;

    fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

#[derive(Debug, Error)]
#[error("Invalid discriminant for {type_name}: {value}")]
pub struct InvalidDiscriminantError {
    pub type_name: &'static str,
    pub value: usize,
}

impl Read for () {
    type Error = !;

    async fn read(_stream: &mut (impl AsyncRead + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(())
    }
}

macro_rules! simple_read_impl {
    ($type:ty, $method:ident) => {
        impl Read for $type {
            type Error = io::Error;

            async fn read(stream: &mut (impl AsyncRead + Unpin + Send)) -> Result<Self, Self::Error>
            where
                Self: Sized,
            {
                stream.$method().await
            }
        }
    };
}

simple_read_impl!(u8, read_u8);
simple_read_impl!(u16, read_u16);
simple_read_impl!(u32, read_u32);
simple_read_impl!(u64, read_u64);

simple_read_impl!(i8, read_i8);
simple_read_impl!(i16, read_i16);
simple_read_impl!(i32, read_i32);
simple_read_impl!(i64, read_i64);

impl Read for String {
    type Error = anyhow::Error;

    async fn read(stream: &mut (impl AsyncRead + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let len = u64::read(stream)
            .await?
            .try_into()
            .context("length exceeds platform capacity")?;

        let mut buf = vec![0; len];
        stream.read_exact(&mut buf).await?;

        Ok(Self::from_utf8(buf)?)
    }
}

impl<T, U> Read for Cow<'_, T>
where
    T: ToOwned<Owned = U> + ?Sized,
    U: Read,
{
    type Error = <U as Read>::Error;

    async fn read(stream: &mut (impl AsyncRead + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        U::read(stream).await.map(Cow::Owned)
    }
}

impl<T> Read for Vec<T>
where
    T: Read + Send,
    anyhow::Error: From<T::Error>,
    Result<T, T::Error>: Context<T, T::Error>,
{
    type Error = anyhow::Error;

    async fn read(stream: &mut (impl AsyncRead + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let len: usize = usize::try_from(u64::read(stream).await?)
            .context("length exceeds platform capacity")?;

        let mut result = Self::with_capacity(len);
        for i in 0..len {
            result.push(
                T::read(stream)
                    .await
                    .with_context(|| format!("while reading element {i}"))?,
            );
        }

        Ok(result)
    }
}

impl<'a, T> Write for &'a T
where
    T: Write + 'a + Sync,
{
    type Error = T::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        T::write(self, stream).await
    }
}

impl Write for () {
    type Error = !;

    async fn write(
        &self,
        _stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

macro_rules! simple_write_impl {
    ($type:ty, $method:ident) => {
        impl Write for $type {
            type Error = io::Error;

            async fn write(
                &self,
                stream: &mut (impl AsyncWrite + Unpin + Send),
            ) -> Result<(), Self::Error> {
                stream.$method(*self).await
            }
        }
    };
}

simple_write_impl!(u8, write_u8);
simple_write_impl!(u16, write_u16);
simple_write_impl!(u32, write_u32);
simple_write_impl!(u64, write_u64);

simple_write_impl!(i8, write_i8);
simple_write_impl!(i16, write_i16);
simple_write_impl!(i32, write_i32);
simple_write_impl!(i64, write_i64);

impl Write for str {
    type Error = io::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (&self).write(stream).await
    }
}

impl Write for &str {
    type Error = io::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (self.len() as u64).write(stream).await?;
        stream.write_all(self.as_bytes()).await
    }
}

impl Write for String {
    type Error = io::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        self.as_str().write(stream).await
    }
}

impl<T> Write for Cow<'_, T>
where
    T: ToOwned + Write + ?Sized + Sync,
    T::Owned: Write<Error = T::Error> + Sync,
{
    type Error = <T as Write>::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        match self {
            Cow::Borrowed(v) => v.write(stream).await,
            Cow::Owned(v) => v.write(stream).await,
        }
    }
}

impl<T> Write for [T]
where
    T: Write + Sync,
    anyhow::Error: From<T::Error>,
    Result<(), T::Error>: Context<(), T::Error>,
{
    type Error = anyhow::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (self.len() as u64).write(stream).await?;

        for (i, e) in self.iter().enumerate() {
            e.write(stream)
                .await
                .with_context(|| format!("while writing element {i}"))?;
        }

        Ok(())
    }
}

impl<T> Write for &[T]
where
    T: Write + Sync,
    anyhow::Error: From<T::Error>,
    Result<(), T::Error>: Context<(), T::Error>,
{
    type Error = anyhow::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (*self).write(stream).await
    }
}

impl<T> Write for Vec<T>
where
    T: Write + Sync,
    anyhow::Error: From<T::Error>,
    Result<(), T::Error>: Context<(), T::Error>,
{
    type Error = anyhow::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWrite + Unpin + Send),
    ) -> Result<(), Self::Error> {
        self.as_slice().write(stream).await
    }
}

ipc_macros::__impl_rw_for_external! {
    enum Result<T, E> {
        Ok(T),
        Err(E)
    }
}

ipc_macros::__impl_rw_for_external! {
    enum Option<T> {
            Some(T),
            None
        }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{BufReader, BufWriter};

    #[tokio::test]
    async fn test_read_unit() {
        let data = &[];
        let mut reader = BufReader::new(&data[..]);
        <()>::read(&mut reader).await.unwrap();
    }

    #[tokio::test]
    async fn test_write_unit() {
        let mut writer = BufWriter::new(Vec::new());
        ().write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![]);
    }

    #[tokio::test]
    async fn test_read_u8() {
        let data = &[42];
        let mut reader = BufReader::new(&data[..]);
        let result = u8::read(&mut reader).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_write_u8() {
        let mut writer = BufWriter::new(Vec::new());
        42u8.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![42]);
    }

    #[tokio::test]
    async fn test_read_i8() {
        let data = &[255]; // -1 in two's complement
        let mut reader = BufReader::new(&data[..]);
        let result = i8::read(&mut reader).await.unwrap();
        assert_eq!(result, -1);
    }

    #[tokio::test]
    async fn test_write_i8() {
        let mut writer = BufWriter::new(Vec::new());
        (-1i8).write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![255]);
    }

    #[tokio::test]
    async fn test_read_string() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111];
        let mut reader = BufReader::new(&data[..]);
        let result = String::read(&mut reader).await.unwrap();
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_write_str() {
        let mut writer = BufWriter::new(Vec::new());
        "Hello".write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111]);
    }

    #[tokio::test]
    async fn test_write_string() {
        let mut writer = BufWriter::new(Vec::new());
        "Hello".to_string().write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111]);
    }

    #[tokio::test]
    async fn test_read_vec_u8() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3];
        let mut reader = BufReader::new(&data[..]);
        let result: Vec<u8> = Vec::read(&mut reader).await.unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_write_slice() {
        let mut writer = BufWriter::new(Vec::new());
        [1u8, 2, 3].write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_write_ref_slice() {
        let mut writer = BufWriter::new(Vec::new());
        (&[1u8, 2, 3] as &[u8]).write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_write_vec_u8() {
        let mut writer = BufWriter::new(Vec::new());
        vec![1u8, 2, 3].write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_read_cow_str() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111];
        let mut reader = BufReader::new(&data[..]);
        let result: Cow<str> = Cow::read(&mut reader).await.unwrap();
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_write_cow_str_borrowed() {
        let mut writer = BufWriter::new(Vec::new());
        let cow: Cow<str> = Cow::Borrowed("Hello");
        cow.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111]);
    }

    #[tokio::test]
    async fn test_write_cow_str_owned() {
        let mut writer = BufWriter::new(Vec::new());
        let cow: Cow<str> = Cow::Owned("Hello".to_string());
        cow.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111]);
    }

    #[tokio::test]
    async fn test_read_result_ok() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 0, 42]; // discriminant for Ok, then value
        let mut reader = BufReader::new(&data[..]);
        let result = <Result<u8, u8>>::read(&mut reader).await.unwrap();
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_read_result_err() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 1, 24]; // discriminant for Err, then value
        let mut reader = BufReader::new(&data[..]);
        let result = <Result<u8, u8>>::read(&mut reader).await.unwrap();
        assert_eq!(result, Err(24));
    }

    #[tokio::test]
    async fn test_write_result_ok() {
        let mut writer = BufWriter::new(Vec::new());
        let val: Result<u8, u8> = Ok(99);
        val.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        // discriminant for Ok (0) followed by the value 99
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 0, 99]);
    }

    #[tokio::test]
    async fn test_write_result_err() {
        let mut writer = BufWriter::new(Vec::new());
        let val: Result<u8, u8> = Err(101);
        val.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        // discriminant for Err (1) followed by the value 101
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 1, 101]);
    }

    #[tokio::test]
    async fn test_read_option_some() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 0, 42]; // discriminant for Some, then value
        let mut reader = BufReader::new(&data[..]);
        let result = <Option<u8>>::read(&mut reader).await.unwrap();
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_read_option_none() {
        let data = &[0, 0, 0, 0, 0, 0, 0, 1]; // discriminant for None
        let mut reader = BufReader::new(&data[..]);
        let result = <Option<u8>>::read(&mut reader).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_write_option_some() {
        let mut writer = BufWriter::new(Vec::new());
        let val: Option<u8> = Some(99);
        val.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();
        // discriminant for Some (0) followed by the value 99
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 0, 99]);
    }

    #[tokio::test]
    async fn test_write_option_none() {
        let mut writer = BufWriter::new(Vec::new());
        let val: Option<u8> = None;
        val.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();
        let result = writer.into_inner();

        // discriminant for None (1)
        assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 1]);
    }
}
