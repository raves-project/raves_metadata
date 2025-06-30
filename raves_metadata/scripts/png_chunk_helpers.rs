//! Create tedious chunk data in PNG test cases.
//!
//! You may paste this into `play.rust-lang.org` and call needed functions in
//! `main`.

#![allow(unused)]

fn main() {}

/// Prints a number as bytes.
///
/// Provide with `1234_u32.to_be_bytes()` for a slice input,
/// not a number.
fn print_number_as_bytes(input: &[u8]) {
    println!("{input:X?}");
}

/// Computes the length of a given PNG chunk data slice.
fn compute_data_length(data: &[u8]) -> u32 {
    let len = data.len() as u32;

    print!("data length: `{len}`. as bytes => ",);
    for b in len.to_be_bytes() {
        print!("0x{:X}, ", b);
    }
    println!();

    len
}

/// Computes the CRC32 of a given PNG chunk data slice.
///
/// Implemented by Copilot, but matches the common implementation.
fn compute_crc32(data: &[u8]) -> [u8; 4] {
    const POLY: u32 = 0xEDB8_8320;

    let mut crc: u32 = 0xFFFF_FFFF;

    for &byte in data {
        let mut cur = byte as u32;
        for _ in 0..8 {
            let bit = (crc ^ cur) & 1;
            crc >>= 1;
            if bit != 0 {
                crc ^= POLY;
            }
            cur >>= 1;
        }
    }

    let crc_bytes = (!crc).to_be_bytes();
    print!("crc32 => ");
    for b in crc_bytes.iter() {
        print!("0x{:X}, ", b);
    }
    println!();

    crc_bytes
}

/// Prints out an `iTXt` block's keyword in Rust byte char format.
///
/// Ex: "Raves" => `[b'R', b'a', b'v', b'e', b's', b'\0']`
fn print_itxt_keyword_for_paste(keyword: &str) {
    print!("[");
    for c in keyword.chars() {
        print!("b'{c}', ");
    }
    print!(r#"b'\0']"#);
}
