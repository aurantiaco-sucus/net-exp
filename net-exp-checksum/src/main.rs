use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};
use rayon::prelude::*;

fn read_all(path: &str) -> Vec<u16> {
    let file = File::open(path).unwrap();
    let size = file.metadata().unwrap().len();
    let mut file = BufReader::new(file);
    let mut fc = Vec::with_capacity((size / 2) as usize);
    let mut buf = [0; 2];
    while let Ok(n) = file.read(&mut buf) {
        if n == 0 {
            break
        }
        if n == 1 {
            buf[1] = 0;
        }
        fc.push(u16::from_be_bytes(buf));
    }
    fc
}

fn checksum(data: &[u16]) -> u16 {
    let mut cur = checksum_pass(data);
    while cur.len() > 1 {
        cur = checksum_pass(&cur);
    }
    cur[0]
}

fn checksum_pass(data: &[u16]) -> Vec<u16> {
    data.chunks(2)
        .par_bridge()
        .map(|x| {
            if x.len() == 1 {
                x[0]
            } else {
                let (mut sum, of) = x[0].overflowing_add(x[1]);
                if of {
                    sum += 1;
                }
                sum
            }
        }).collect::<Vec<_>>()
}

fn main() {
    let file = args().nth(1)
        .expect("No input file specified!");
    let data = read_all(&file);
    let sum = checksum(&data);
    println!("{sum:x}");
}
