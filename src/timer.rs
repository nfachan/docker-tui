use derive_more::From;
use itertools::{Either, Itertools as _};
use std::{collections::HashMap, iter::Extend, ops::AddAssign, time::Duration};
use tokio::{
    sync::mpsc,
    time::{self, Instant},
};

#[derive(Clone, Copy, Debug, Default, Eq, From, Hash, PartialEq)]
pub struct Id(pub u64);

impl<Rhs> AddAssign<Rhs> for Id
where
    Id: From<Rhs>,
{
    fn add_assign(&mut self, rhs: Rhs) {
        let rhs = Id::from(rhs);
        self.0 += rhs.0;
    }
}

pub enum MessageIn {
    Start(Id, Duration),
    Cancel(Id),
}

#[derive(Debug)]
pub enum MessageOut {
    TimerExpired(Id),
}

struct Min<T>(Option<T>);

impl<T> Default for Min<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Ord> Extend<T> for Min<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0 = iter.into_iter().min();
    }
}

pub async fn main<E>(
    mut receiver: mpsc::UnboundedReceiver<MessageIn>,
    sender: mpsc::Sender<E>,
    sender_processor: impl Fn(MessageOut) -> E,
) {
    let mut active_timers = HashMap::<Id, Instant>::default();
    loop {
        let now = Instant::now();
        let (pending, expired): (Min<Instant>, Vec<_>) =
            active_timers.iter().partition_map(|(id, when)| {
                now.checked_duration_since(*when)
                    .map_or_else(|| Either::Left(*when), |_| Either::Right(*id))
            });
        for id in expired {
            active_timers.remove(&id);
            if sender
                .send(sender_processor(MessageOut::TimerExpired(id)))
                .await
                .is_err()
            {
                return;
            }
        }
        let message = match pending.0 {
            Some(when) => match time::timeout_at(when, receiver.recv()).await {
                Err(_) => {
                    continue;
                }
                Ok(result) => result,
            },
            None => receiver.recv().await,
        };
        match message {
            None => {
                return;
            }
            Some(MessageIn::Start(id, duration)) => {
                active_timers.insert(id, Instant::now() + duration);
            }
            Some(MessageIn::Cancel(id)) => {
                active_timers.remove(&id);
            }
        }
    }
}
