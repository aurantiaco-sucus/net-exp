use std::collections::HashSet;
use rand::prelude::*;
use net_exp_bridge::{Address, Frame, Segment};
use std::io::Write;

const VALID_ADDR_CNT: usize = 10000;
const INVALID_ADDR_CNT: usize = 2500;
const SEQ_CNT: usize = 100;
const VALID_FRAME_CNT: usize = 1000_0000;
const INVALID_FRAME_CNT: usize = 50_0000;

fn gen_byte_arr<const N: usize>() -> [u8; N] {
    let mut data = [0u8; N];
    data.iter_mut().for_each(|x| *x = fastrand::u8(..));
    data
}

fn gen_addr() -> Address {
    Address { data: gen_byte_arr() }
}

fn gen_addr_pool(count: usize) -> HashSet<Address> {
    let pb = indicatif::ProgressBar::new(count as u64).with_prefix("ADDR");
    let mut unique_set: HashSet<Address> = HashSet::with_capacity(count);
    while unique_set.len() < count {
        unique_set.insert(gen_addr());
        pb.set_position(unique_set.len() as u64);
    }
    unique_set
}

fn gen_invalid_addr_pool(addr_pool: &HashSet<Address>, count: usize) -> HashSet<Address> {
    let pb = indicatif::ProgressBar::new(count as u64).with_prefix("INV_ADDR");
    let mut unique_set: HashSet<Address> = HashSet::with_capacity(count);
    while unique_set.len() < count {
        let addr = gen_addr();
        if !addr_pool.contains(&addr) {
            unique_set.insert(addr);
        }
        pb.set_position(unique_set.len() as u64);
    }
    unique_set
}

fn gen_seg() -> Segment {
    Segment { data: gen_byte_arr() }
}

fn gen_seg_pool(count: usize) -> HashSet<Segment> {
    let pb = indicatif::ProgressBar::new(count as u64).with_prefix("SEG");
    let mut unique_set: HashSet<Segment> = HashSet::with_capacity(count);
    while unique_set.len() < count {
        unique_set.insert(gen_seg());
        pb.set_position(unique_set.len() as u64);
    }
    unique_set
}

fn gen_data() -> [u8; 16] {
    gen_byte_arr()
}

fn gen_frame(src_pool: &[Address], dst_pool: &[Address]) -> Frame {
    let src = src_pool[fastrand::usize(0..src_pool.len())];
    let mut dst = src;
    while dst == src {
        dst = dst_pool[fastrand::usize(0..dst_pool.len())];
    }
    let data = gen_data();
    Frame { src, dst, data }
}

fn gen_frame_seq(src_pool: &[Address], dst_pool: &[Address], count: usize) -> Vec<Frame> {
    let pb = indicatif::ProgressBar::new(count as u64).with_prefix("FRAME");
    let mut seq = Vec::with_capacity(count);
    for _ in 0..count {
        seq.push(gen_frame(src_pool, dst_pool));
        pb.inc(1);
    }
    seq
}

fn gen_addr_seg(addr_pool: Vec<Address>, seg_pool: &[Segment]) -> Vec<(Address, Segment)> {
    let mut seq = Vec::with_capacity(addr_pool.len() * seg_pool.len());
    let least = addr_pool.len() / seg_pool.len();
    let pb = indicatif::ProgressBar::new(seg_pool.len() as u64).with_prefix("ADDR_SEG");
    for (i, seg) in seg_pool.iter().enumerate() {
        let begin = i * least;
        for j in 0..least {
            seq.push((addr_pool[begin + j], *seg));
        }
        pb.inc(1);
    }
    if seq.len() < addr_pool.len() {
        let begin = seq.len();
        for i in begin..addr_pool.len() {
            seq.push((addr_pool[i], seg_pool[fastrand::usize(0..seg_pool.len())]));
        }
    }
    seq
}

fn serialize(addr_seg_seq: &[(Address, Segment)], inv_addr_pool: &[Address], frame_seq: &[Frame]) {
    let addr_seg_file = std::fs::File::create("addr_seg.txt").unwrap();
    let inv_addr_file = std::fs::File::create("inv_addr.txt").unwrap();
    let frame_file = std::fs::File::create("frame.txt").unwrap();
    let mut addr_seg_bw = std::io::BufWriter::new(addr_seg_file);
    let mut inv_addr_bw = std::io::BufWriter::new(inv_addr_file);
    let mut frame_bw = std::io::BufWriter::new(frame_file);
    for (addr, seg) in addr_seg_seq {
        writeln!(addr_seg_bw, "{} {}", addr, seg).unwrap();
    }
    for addr in inv_addr_pool {
        writeln!(inv_addr_bw, "{}", addr).unwrap();
    }
    for frame in frame_seq {
        writeln!(frame_bw, "{} {} {}",
                 frame.src,
                 frame.dst,
                 frame.data.iter()
                     .map(|x| format!("{:02x}", x))
                     .collect::<Vec<_>>().join("")).unwrap();
    }
}

fn main() {
    let addr_pool = gen_addr_pool(VALID_ADDR_CNT);
    let inv_addr_pool = gen_addr_pool(INVALID_ADDR_CNT);
    let seg_pool = gen_seg_pool(SEQ_CNT);

    let addr_pool = addr_pool.into_iter().collect::<Vec<_>>();
    let inv_addr_pool = inv_addr_pool.into_iter().collect::<Vec<_>>();
    let seg_pool = seg_pool.into_iter().collect::<Vec<_>>();

    let frame_seq = {
        let mut frame_seq = gen_frame_seq(&addr_pool, &addr_pool, VALID_FRAME_CNT);
        let inv_frame_seq = gen_frame_seq(&addr_pool, &inv_addr_pool, INVALID_FRAME_CNT);
        frame_seq.extend_from_slice(&inv_frame_seq);
        frame_seq.shuffle(&mut thread_rng());
        frame_seq
    };

    let addr_seg_seq = gen_addr_seg(addr_pool, &seg_pool);

    serialize(&addr_seg_seq, &inv_addr_pool, &frame_seq);
}