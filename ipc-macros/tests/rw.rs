#![allow(clippy::unwrap_used)]

use ipc::{Read as _, Write as _};
use ipc_macros::{Read, Write};
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};

#[tokio::test]
async fn unit_struct() {
    #[derive(Read, Write)]
    struct UnitStruct;

    let x = UnitStruct;

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    UnitStruct::read(&mut reader).await.unwrap();
}

#[tokio::test]
async fn empty_tuple_struct() {
    #[derive(Read, Write)]
    struct EmptyTupleStruct();

    let x = EmptyTupleStruct();

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    EmptyTupleStruct::read(&mut reader).await.unwrap();
}

#[tokio::test]
async fn tuple_struct() {
    #[derive(Read, Write, Debug, PartialEq)]
    struct TupleStruct(u8, u32, String);

    let x = TupleStruct(255, 0xDEAD_BEEF, "Tuple".to_string());

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = TupleStruct::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
}

#[tokio::test]
async fn empty_named_struct() {
    #[derive(Read, Write)]
    struct EmptyNamedStruct {}

    let x = EmptyNamedStruct {};

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    EmptyNamedStruct::read(&mut reader).await.unwrap();
}

#[tokio::test]
async fn named_struct() {
    #[derive(Read, Write, Debug, PartialEq)]
    struct NamedStruct {
        id: u64,
        values: Vec<i32>,
        flag: Option<u8>,
    }

    let x = NamedStruct {
        id: 123_456_789,
        values: vec![-1, -2, -3],
        flag: Some(1),
    };

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = NamedStruct::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
}

#[tokio::test]
async fn generic_struct() {
    #[derive(Read, Write, Debug, PartialEq)]
    struct Container<T> {
        item: T,
        count: u32,
    }

    let x = Container {
        item: "Generic".to_string(),
        count: 5,
    };

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = Container::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
}

#[tokio::test]
async fn struct_with_cow() {
    use std::borrow::Cow;

    #[derive(Read, Write, Debug, PartialEq)]
    struct Message<'a> {
        id: u16,
        content: Cow<'a, str>,
    }

    // Test with Owned data
    let x = Message {
        id: 1,
        content: Cow::Owned("Owned String".to_string()),
    };

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = Message::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
    assert!(matches!(result.content, Cow::Owned(_)));

    // Test with Borrowed data
    let x = Message {
        id: 2,
        content: Cow::Borrowed("Borrowed String"),
    };

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = Message::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
    assert!(matches!(result.content, Cow::Owned(_)));
}

#[allow(dead_code)]
#[derive(Read, Write)]
enum EmptyEnum {}

#[tokio::test]
async fn enum_unit_variants() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum Color {
        Red,
        Green,
        Blue,
    }

    let variants = vec![Color::Red, Color::Green, Color::Blue];

    for x in variants {
        let mut writer = BufWriter::new(Vec::new());
        x.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();

        let mut data = &writer.into_inner()[..];
        let mut reader = BufReader::new(&mut data);
        let result = Color::read(&mut reader).await.unwrap();

        assert_eq!(x, result);
    }
}

#[tokio::test]
async fn enum_mixed_variants() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum Command {
        Quit,
        Move(i32, i32),
        Write { text: String, id: u8 },
    }

    let inputs = vec![Command::Quit, Command::Move(10, -10), Command::Write { text: "Hello".to_string(), id: 7 }];

    for x in inputs {
        let mut writer = BufWriter::new(Vec::new());
        x.write(&mut writer).await.unwrap();
        writer.flush().await.unwrap();

        let mut data = &writer.into_inner()[..];
        let mut reader = BufReader::new(&mut data);
        let result = Command::read(&mut reader).await.unwrap();

        assert_eq!(x, result);
    }
}

#[tokio::test]
async fn generic_enum() {
    #[derive(Read, Write, Debug, PartialEq)]
    enum Either<A, B> {
        Left(A),
        Right(B),
    }

    let x: Either<u32, String> = Either::Right("Success".to_string());

    let mut writer = BufWriter::new(Vec::new());
    x.write(&mut writer).await.unwrap();
    writer.flush().await.unwrap();

    let mut data = &writer.into_inner()[..];
    let mut reader = BufReader::new(&mut data);
    let result = Either::<u32, String>::read(&mut reader).await.unwrap();

    assert_eq!(x, result);
}
