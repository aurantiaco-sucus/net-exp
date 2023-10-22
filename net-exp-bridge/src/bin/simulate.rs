use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender};
use net_exp_bridge::{Address, Frame, Segment};

enum Event {
    IncomingFrame(Frame),
    Response(Segment),
    Shutdown,
}

enum Command {
    Broadcast(Frame),
    Dispatch(Frame, Segment),
    Discard(Frame),
}

fn bridge(tx: Sender<Command>, rx: Receiver<Event>) {

}

fn orchestrator(frame_seq: Vec<Frame>, tx: Sender<Event>) {

}

fn facility(mapping: BTreeMap<Address, Segment>, tx: Sender<Event>, rx: Receiver<Command>) {

}

fn

fn main() {
    let (tx, rx) = std::sync::mpsc::channel();
}