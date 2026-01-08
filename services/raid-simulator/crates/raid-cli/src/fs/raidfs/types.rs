use std::sync::{Arc, Mutex};

use raid_rs::layout::stripe::traits::stripe::Stripe;
use raid_rs::retention::volume::Volume;

use crate::fs::metadata::{Entry, Header};

pub(crate) struct FsState<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub(crate) volume: Volume<D, N, T>,
    pub(crate) header: Header,
    pub(crate) entries: Vec<Entry>,
}

pub(crate) struct RaidFs<const D: usize, const N: usize, T: Stripe<D, N>> {
    pub(crate) state: Arc<Mutex<FsState<D, N, T>>>,
    pub(crate) capacity: u64,
}
