#[derive(Debug)]
pub enum CommStructError {
    SignatureError,
    SerializeError,
    DeSerializeError,
    KeyIndexError,
}
