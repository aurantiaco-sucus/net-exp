use std::env::args;
use std::fs::File;
use std::io::Read;
use tqdm::tqdm;

const BUF_INITIAL_SIZE: usize = 16384;

fn main() {
    let file = args().nth(1)
        .expect("No input file specified!");
    let mut file = File::open(file)
        .expect("Can't open the file!");
    let mut data = Vec::with_capacity(BUF_INITIAL_SIZE);
    file.read_to_end(&mut data)
        .expect("Error reading the file!");
    if data.len() == 0 {
        panic!("File is empty!");
    }
    if data.len() % 2 == 1 {
        data.push(0);
    }
    let mut sum: u16 = 0;
    let mut overflow = false;
    for word in tqdm(data.chunks_exact(2)) {
        let word = u16::from_be_bytes(word.try_into().unwrap());
        (sum, overflow) = sum.overflowing_add(word);
        if overflow {
            sum += 1;
        }
    }
    println!("{sum:x}");
}
