use std::collections::{BTreeMap, LinkedList};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{Receiver, Sender};
use std::{fs, thread};
use std::f64::consts::PI;
use std::time::{Duration, Instant};
use log::info;
use net_exp_bridge::{Address, Frame, Segment};
use std::io::Write;

enum Event {
    Request(Frame),
    Success(Address, Segment),
    Failure(Address),
    Shutdown,
}

enum Command {
    Broadcast(Address),
    Dispatch(Frame, Segment),
    Discard(Frame),
}

struct Holder {
    map: BTreeMap<Address, Vec<Frame>>
}

impl Holder {
    fn new() -> Self {
        Holder { map: BTreeMap::new() }
    }

    fn hold(&mut self, frame: Frame) {
        let frames = self.map.entry(frame.dst)
            .or_insert_with(Vec::new);
        frames.push(frame);
    }

    fn release(&mut self, addr: Address) -> Vec<Frame> {
        self.map.remove(&addr).unwrap_or_default()
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

pub enum BridgeStatRecord {
    Broadcast(Frame),
    Dispatch(Frame),
    Discard(Frame),
}

impl BridgeStatRecord {
    pub fn frame(&self) -> &Frame {
        match self {
            BridgeStatRecord::Broadcast(frame) => frame,
            BridgeStatRecord::Dispatch(frame) => frame,
            BridgeStatRecord::Discard(frame) => frame,
        }
    }
}

pub struct BridgeStat {
    pub records: Vec<BridgeStatRecord>,
    pub times: Vec<Instant>,
    pub init: Instant,
}

impl BridgeStat {
    fn new() -> Self {
        BridgeStat { records: Vec::new(), times: Vec::new(), init: Instant::now() }
    }

    fn broadcast(&mut self, frame: Frame) {
        self.records.push(BridgeStatRecord::Broadcast(frame));
        self.times.push(Instant::now());
    }

    fn dispatch(&mut self, frame: Frame) {
        self.records.push(BridgeStatRecord::Dispatch(frame));
        self.times.push(Instant::now());
    }

    fn discard(&mut self, frame: Frame) {
        self.records.push(BridgeStatRecord::Discard(frame));
        self.times.push(Instant::now());
    }

    fn len(&self) -> usize {
        self.records.len()
    }

    fn export_activity_scatter(&self) {
        let sc_src = self.records.iter()
            .zip(self.times.iter())
            .map(|(x, y)| (x, y.duration_since(self.init).as_micros()))
            .map(|(x, y)| (x, y.to_string()));

        let mut sc_broadcast = Vec::new();
        let mut sc_dispatch = Vec::new();
        let mut sc_discard = Vec::new();

        for (x, y) in sc_src {
            match x {
                BridgeStatRecord::Broadcast(_) => sc_broadcast.push(y),
                BridgeStatRecord::Dispatch(_) => sc_dispatch.push(y),
                BridgeStatRecord::Discard(_) => sc_discard.push(y),
            }
        }

        let sc_broadcast = sc_broadcast.join(" ");
        let sc_dispatch = sc_dispatch.join(" ");
        let sc_discard = sc_discard.join(" ");

        fs::write("sc_broadcast_activity.txt", sc_broadcast).unwrap();
        fs::write("sc_dispatch_activity.txt", sc_dispatch).unwrap();
        fs::write("sc_discard_activity.txt", sc_discard).unwrap();
    }

    fn export_latency_scatter(&self) {
        let total = {
            let now = Instant::now();
            now.duration_since(self.init).as_millis()
        };
        let holds = self.records.iter()
            .zip(self.times.iter())
            .filter(|(x, _)| matches!(x, BridgeStatRecord::Broadcast(_)))
            .map(|(x, y)| (x.frame().clone(), y))
            .map(|(x, y)| (x, y.duration_since(self.init).as_micros()))
            .collect::<Vec<_>>();
        let releases = self.records.iter()
            .zip(self.times.iter())
            .filter(|(x, _)| !matches!(x, BridgeStatRecord::Broadcast(_)))
            .map(|(x, y)| (x.frame().clone(), y))
            .map(|(x, y)| (x, y.duration_since(self.init).as_micros()))
            .collect::<Vec<_>>();
        let mut latencies = Vec::with_capacity(holds.len());
        let mut last_i = 0;
        let pb = indicatif::ProgressBar::new(holds.len() as u64).with_prefix("LATENCY");
        for (hold, hold_t) in holds {
            while releases[last_i].1 >= hold_t && last_i > 0 {
                last_i -= 1;
            }
            let (release_i, release_t) = releases[last_i..].iter().enumerate()
                .find(|(_, (release, _))| release == &hold)
                .map(|(i, (_, release_t))| (i, release_t))
                .unwrap();
            latencies.push((hold_t, release_t - hold_t));
            last_i = release_i;
            pb.inc(1);
        }
        let latencies = latencies.iter()
            .map(|(x, y)| format!("{} {}", x, y))
            .collect::<Vec<String>>()
            .join("\n");
        fs::write("sc_latency.txt", latencies).unwrap();
    }
}

pub struct BridgePendingStat {
    pub records: Vec<usize>,
    pub times: Vec<Instant>,
    pub init: Instant,
}

impl BridgePendingStat {
    fn new() -> Self {
        BridgePendingStat { records: Vec::new(), times: Vec::new(), init: Instant::now() }
    }

    fn rec(&mut self, count: usize) {
        self.records.push(count);
        self.times.push(Instant::now());
    }

    fn len(&self) -> usize {
        self.records.len()
    }

    fn export_congestion_scatter(&self) {
        let sc_congestion = self.records.iter()
            .zip(self.times.iter())
            .map(|(x, y)| (x, y.duration_since(self.init).as_micros()))
            .map(|(x, y)| format!("{} {}", y, x))
            .collect::<Vec<String>>()
            .join("\n");
        fs::write("sc_congestion.txt", sc_congestion).unwrap();
    }
}

fn bridge(tc: Sender<Command>, re: Receiver<Event>) {
    info!(target: "bridge", "Bridge started.");
    let mut mapping = BTreeMap::new();
    let mut pending = Holder::new();
    let mut stat = BridgeStat::new();
    let mut pending_stat = BridgePendingStat::new();
    let mut req_cnt = 0;
    let mut b_cnt = 0;
    let mut dp_cnt = 0;
    let mut dc_cnt = 0;
    let mut last_t = Instant::now();
    while let Ok(event) = re.recv() {
        match event {
            Event::Request(frame) => {
                if let Some(segment) = mapping.get(&frame.dst) {
                    stat.dispatch(frame.clone());
                    tc.send(Command::Dispatch(frame, *segment)).unwrap();
                    req_cnt += 1;
                    dp_cnt += 1;
                } else {
                    stat.broadcast(frame.clone());
                    tc.send(Command::Broadcast(frame.dst)).unwrap();
                    pending_stat.rec(pending.len());
                    pending.hold(frame);
                    b_cnt += 1;
                }
            }
            Event::Success(address, segment) => {
                mapping.insert(address, segment);
                for frame in pending.release(address) {
                    stat.dispatch(frame.clone());
                    tc.send(Command::Dispatch(frame, segment)).unwrap();
                    dp_cnt += 1;
                }
                pending_stat.rec(pending.len());
            }
            Event::Failure(address) => {
                for frame in pending.release(address) {
                    stat.discard(frame.clone());
                    tc.send(Command::Discard(frame)).unwrap();
                    dc_cnt += 1;
                }
                pending_stat.rec(pending.len());
            }
            Event::Shutdown => {
                info!(target: "bridge", "Received shutdown signal.");
                stat.export_activity_scatter();
                stat.export_latency_scatter();
                pending_stat.export_congestion_scatter();
                break;
            }
        }
        if last_t.elapsed() > Duration::from_millis(50) {
            info!(target: "bridge", "Received {} requests. Done {} broadcasts, {} dispatches and {} discards.",
                    req_cnt, b_cnt, dp_cnt, dc_cnt);
            req_cnt = 0;
            b_cnt = 0;
            dp_cnt = 0;
            dc_cnt = 0;
            last_t = Instant::now();
        }
    }
    info!(target: "bridge", "Bridge exiting.");
}

fn half_circle_dist_cdf(x: f64) -> f64 {
    let x = x * PI - PI / 2.0;
    (x.sin() + 1.0) / 2.0
}

fn distribute(frame_seq: Vec<Frame>, dur_sec: usize, dist: fn(f64) -> f64) -> Vec<Vec<Frame>> {
    let mut buckets = vec![Vec::new(); dur_sec * 1000];
    let mut last_pos = 0;
    let dur = dur_sec * 1000;
    for (i, vec) in buckets.iter_mut().enumerate() {
        let pos = (dist(i as f64 / dur as f64) * frame_seq.len() as f64) as usize;
        vec.extend_from_slice(&frame_seq[last_pos..pos]);
        last_pos = pos;
    }
    if last_pos < frame_seq.len() {
        buckets.last_mut().unwrap().extend_from_slice(&frame_seq[last_pos..]);
    }
    buckets
}

fn orchestrator(frame_seq: Vec<Frame>, te: Sender<Event>) {
    info!(target: "orchestrator", "Orchestrator started.");
    let frame_seq = distribute(frame_seq, 10, half_circle_dist_cdf);
    let begin = Instant::now();
    let mut last = 0;
    let mut last_t = Instant::now();
    let mut count = 0;
    loop {
        let now = Instant::now();
        let dur = now.duration_since(begin);
        let cur = dur.as_secs() * 1000 + dur.subsec_millis() as u64;
        if cur >= frame_seq.len() as u64 {
            for buckets in frame_seq[last..].iter() {
                for frame in buckets {
                    te.send(Event::Request(frame.clone())).unwrap();
                }
            }
            break;
        }
        if cur > last as u64 {
            for buckets in frame_seq[last..cur as usize].iter() {
                for frame in buckets {
                    te.send(Event::Request(frame.clone())).unwrap();
                    count += 1;
                }
            }
            last = cur as usize;
        }
        if now.duration_since(last_t) > Duration::from_millis(250) {
            info!(target: "orchestrator", "Sent {} frames.", count);
            count = 0;
            last_t = now;
        }
        thread::sleep(Duration::from_millis(1));
    }
    info!(target: "orchestrator", "Orchestrator exiting.");
}

struct FacilityMeter {
    s_cnt: usize,
    f_cnt: usize,
    dp_cnt: usize,
    dc_cnt: usize,
}

impl FacilityMeter {
    fn new() -> Self {
        FacilityMeter { s_cnt: 0, f_cnt: 0, dp_cnt: 0, dc_cnt: 0 }
    }

    fn inc_success(&mut self) {
        self.s_cnt += 1;
    }

    fn inc_failure(&mut self) {
        self.f_cnt += 1;
    }

    fn inc_dispatch(&mut self) {
        self.dp_cnt += 1;
    }

    fn inc_discard(&mut self) {
        self.dc_cnt += 1;
    }

    fn report(&mut self) {
        info!(target: "facility", "Handled {} successes, {} failures, {} dispatches and {} discards.",
            self.s_cnt, self.f_cnt, self.dp_cnt, self.dc_cnt);
        self.s_cnt = 0;
        self.f_cnt = 0;
        self.dp_cnt = 0;
        self.dc_cnt = 0;
    }
}

fn facility(count: usize, mapping: BTreeMap<Address, Segment>, te: Sender<Event>, rc: Receiver<Command>) {
    info!(target: "facility", "Facility started.");
    let mut cur_n = 0;
    let mut meter = FacilityMeter::new();
    let mut last_t = Instant::now();
    while let Ok(command) = rc.recv() {
        match command {
            Command::Broadcast(addr) => {
                if let Some(segment) = mapping.get(&addr) {
                    te.send(Event::Success(addr, *segment)).unwrap();
                    meter.inc_success();
                } else {
                    te.send(Event::Failure(addr)).unwrap();
                    meter.inc_failure();
                }
            }
            Command::Dispatch(_, _) => {
                meter.inc_dispatch();
                cur_n += 1;
            }
            Command::Discard(_) => {
                meter.inc_discard();
                cur_n += 1;
            }
        }
        if last_t.elapsed() > Duration::from_millis(250) {
            meter.report();
            last_t = Instant::now();
        }
        if cur_n == count {
            te.send(Event::Shutdown).unwrap();
            break;
        }
    }
    info!(target: "facility", "Facility exiting.");
}

fn load_mapping() -> BTreeMap<Address, Segment> {
    let addr_seg = BufReader::new(File::open("addr_seg.rmp").unwrap());
    let addr_seg: Vec<(Address, Segment)> = rmp_serde::from_read(addr_seg).unwrap();
    BTreeMap::from_iter(addr_seg)
}

fn load_frames() -> Vec<Frame> {
    let frame = BufReader::new(File::open("frame.rmp").unwrap());
    rmp_serde::from_read(frame).unwrap()
}

fn main() {
    env_logger::init();
    let (tc, rc) = std::sync::mpsc::channel();
    let (te, re) = std::sync::mpsc::channel();
    let frames = load_frames();

    let facility = {
        let mapping = load_mapping();
        let te = te.clone();
        let len = frames.len();
        thread::spawn(move || facility(len, mapping, te, rc))
    };

    let bridge = {
        let tc = tc.clone();
        thread::spawn(move || bridge(tc, re))
    };

    let orchestrator = {
        let te = te.clone();
        thread::spawn(move || orchestrator(frames, te))
    };

    orchestrator.join().unwrap();
    facility.join().unwrap();
    bridge.join().unwrap();
}