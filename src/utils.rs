pub fn usize_to_u8_array(x: usize) -> [u8; 4] {
    let b1: u8 = ((x >> 24) & 0xff) as u8;
    let b2: u8 = ((x >> 16) & 0xff) as u8;
    let b3: u8 = ((x >> 8) & 0xff) as u8;
    let b4: u8 = (x & 0xff) as u8;

    [b1, b2, b3, b4]
}

pub fn u8_array_to_usize(buf: &[u8], i: usize) -> usize {
    let b1: usize = (buf[i] as usize) << 24;
    let b2: usize = (buf[i + 1] as usize) << 16;
    let b3: usize = (buf[i + 2] as usize) << 8;
    let b4: usize = buf[i + 3] as usize;

    b1 + b2 + b3 + b4
}