pub const SERVER_PORT: u16 = 9999;
pub const CLIENT_PORT: u16 = 0;
pub const MSG_BUF_SIZE: usize = 4096;

pub type ResultB<T> = ::std::result::Result<T, Box<::std::error::Error>>;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum FnCall {
    Add(i32),
    Echo(i32),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum FnRes {
    Add(i32),
    Echo(i32),
}
