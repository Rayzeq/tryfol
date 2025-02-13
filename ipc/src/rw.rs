use anyhow::Context;
use std::{borrow::Cow, convert::Infallible, future::Future, io};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait Read {
    type Error;

    fn read(
        stream: &mut (impl AsyncReadExt + Unpin + Send),
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send
    where
        Self: Sized;
}

pub trait Write {
    type Error;

    fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

#[derive(Debug, Error)]
#[error("Invalid discriminant for {type_name}: {value}")]
pub struct InvalidDiscriminantError {
    pub type_name: &'static str,
    pub value: usize,
}

impl Read for () {
    type Error = Infallible;

    async fn read(_stream: &mut (impl AsyncReadExt + Unpin + Send)) -> Result<Self, Self::Error>
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

            async fn read(
                stream: &mut (impl AsyncReadExt + Unpin + Send),
            ) -> Result<Self, Self::Error>
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

    async fn read(stream: &mut (impl AsyncReadExt + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let len = u64::read(stream)
            .await?
            .try_into()
            .context("String is too long for this platform")?;

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

    async fn read(stream: &mut (impl AsyncReadExt + Unpin + Send)) -> Result<Self, Self::Error>
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
{
    type Error = anyhow::Error;

    async fn read(stream: &mut (impl AsyncReadExt + Unpin + Send)) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let len: usize = u64::read(stream)
            .await?
            .try_into()
            .context("Too many elements for this platform")?;

        let mut result = Self::with_capacity(len);
        for _ in 0..len {
            result.push(T::read(stream).await?);
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
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        T::write(self, stream).await
    }
}

impl Write for () {
    type Error = Infallible;

    async fn write(
        &self,
        _stream: &mut (impl AsyncWriteExt + Unpin + Send),
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
                stream: &mut (impl AsyncWriteExt + Unpin + Send),
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

impl Write for &str {
    type Error = io::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (self.len() as u64).write(stream).await?;
        stream.write_all(self.as_bytes()).await
    }
}

impl Write for str {
    type Error = io::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (&self).write(stream).await
    }
}

impl Write for String {
    type Error = <&'static str as Write>::Error;

    async fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
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
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        match self {
            Cow::Borrowed(v) => v.write(stream).await,
            Cow::Owned(v) => v.write(stream).await,
        }
    }
}

#[derive(Debug, Error)]
pub enum WriteVectorError<T> {
    #[error("Cannot write the length of the vector: {0}")]
    Length(#[source] io::Error),
    #[error("Cannot write the content of the vector: {0}")]
    Content(#[source] T),
}

impl<T> Write for [T]
where
    T: Write + Sync,
{
    type Error = WriteVectorError<<T as Write>::Error>;

    async fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        (self.len() as u64)
            .write(stream)
            .await
            .map_err(WriteVectorError::Length)?;

        for e in self {
            e.write(stream).await.map_err(WriteVectorError::Content)?;
        }

        Ok(())
    }
}

impl<T> Write for Vec<T>
where
    T: Write + Sync,
{
    type Error = WriteVectorError<<T as Write>::Error>;

    async fn write(
        &self,
        stream: &mut (impl AsyncWriteExt + Unpin + Send),
    ) -> Result<(), Self::Error> {
        self.as_slice().write(stream).await
    }
}

ipc_macros::__impl_rw_for_result!();
ipc_macros::__impl_rw_for_option!();

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_macros::{Read, Write};
    use tokio::io::{BufReader, BufWriter};

    #[derive(Read, Write)]
    pub struct EmptyStruct {}

    #[derive(Read, Write)]
    pub enum EmptyEnum {}

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
        let data = &[42u8];
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
    async fn test_read_string() {
        let data = &[0u8, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111];
        let mut reader = BufReader::new(&data[..]);
        let result = String::read(&mut reader).await.unwrap();
        assert_eq!(result, "Hello");
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
        let data = &[0u8, 0, 0, 0, 0, 0, 0, 3, 1, 2, 3];
        let mut reader = BufReader::new(&data[..]);
        let result: Vec<u8> = Vec::read(&mut reader).await.unwrap();
        assert_eq!(result, vec![1, 2, 3]);
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
        let data = &[0u8, 0, 0, 0, 0, 0, 0, 5, 72, 101, 108, 108, 111];
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
}
