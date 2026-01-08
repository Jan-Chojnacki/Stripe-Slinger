use std::sync::{Arc, Mutex};

use raid_rs::layout::stripe::traits::stripe::Stripe;
use raid_rs::retention::volume::Volume;

use crate::fs::metadata::{Entry, Header};

pub struct FsState<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub volume: Volume<D, N, T>,
    pub header: Header,
    pub entries: Vec<Entry>,
}

pub struct RaidFs<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub state: Arc<Mutex<FsState<D, N, T>>>,
    pub capacity: u64,
}
