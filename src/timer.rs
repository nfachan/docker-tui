use color_eyre::{Report, Result};
use itertools::{Either, Itertools as _};
use std::{collections::HashMap, iter::Extend, time::Duration};
use tokio::{
    runtime::Builder,
    sync::mpsc,
    task,
    time::{self, Instant},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id(pub u64);

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

async fn main_inner<E>(
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

pub fn main<E: Send + 'static>(
    receiver: mpsc::UnboundedReceiver<MessageIn>,
    sender: mpsc::Sender<E>,
    sender_processor: impl Fn(MessageOut) -> E + Send + Sync + 'static,
) -> Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move { task::spawn(main_inner(receiver, sender, sender_processor)).await })
        .map_err(Report::from)
}
