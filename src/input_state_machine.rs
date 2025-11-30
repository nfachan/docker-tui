use derive_more::Display;
use std::{
    collections::{HashMap, hash_map},
    error::Error,
    hash::Hash,
    result::Result,
};

#[derive(Debug, Display, Eq, PartialEq)]
pub enum BindingErrorKind {
    #[display("extension of binding already bound")]
    ExtensionAlreadyBound,
    #[display("prefix of binding already bound")]
    PrefixAlreadyBound,
    #[display("attempt to bind an empty key sequence")]
    Empty,
}

#[derive(derive_more::Debug, Display)]
#[debug("{kind:?}")]
#[display("{kind}")]
pub struct BindingError<K, V> {
    kind: BindingErrorKind,
    builder: Builder<K, V>,
}

impl<K, V> Error for BindingError<K, V> {}

#[derive(Debug)]
enum Entry<V> {
    Done(V),
    NeedMore(usize),
}

pub struct InputStateMachine<K, V> {
    states: Vec<HashMap<K, Entry<V>>>,
    current_state: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum InputResult<V> {
    Done(V),
    NeedMore,
    Invalid,
}

impl<K, V> InputStateMachine<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn input(&mut self, key: K) -> InputResult<V> {
        match self.states[self.current_state].get(&key) {
            None => {
                self.current_state = 0;
                InputResult::Invalid
            }
            Some(Entry::Done(value)) => {
                self.current_state = 0;
                InputResult::Done(value.clone())
            }
            Some(Entry::NeedMore(next_state)) => {
                self.current_state = *next_state;
                InputResult::NeedMore
            }
        }
    }
}

#[derive(Debug)]
pub struct Builder<K, V>(Vec<HashMap<K, Entry<V>>>);

impl<K, V> Default for Builder<K, V> {
    fn default() -> Self {
        Builder(vec![HashMap::default()])
    }
}

impl<K, V> Builder<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn binding(
        mut self,
        keys: impl IntoIterator<Item = K>,
        value: V,
    ) -> Result<Self, BindingError<K, V>> {
        let keys = Vec::from_iter(keys);
        let num_keys = keys.len();
        if num_keys == 0 {
            return Err(BindingError {
                kind: BindingErrorKind::Empty,
                builder: self,
            });
        }
        let mut keys = keys.into_iter();
        let mut state = 0;
        for _ in 0..num_keys - 1 {
            let key = keys.next().unwrap();
            match self.0[state].get(&key) {
                None => {
                    let next_state = self.0.len();
                    self.0[state].insert(key, Entry::NeedMore(next_state));
                    self.0.push(HashMap::default());
                    state = next_state;
                }
                Some(Entry::NeedMore(next_state)) => {
                    state = *next_state;
                }
                Some(Entry::Done(_)) => {
                    return Err(BindingError {
                        kind: BindingErrorKind::PrefixAlreadyBound,
                        builder: self,
                    });
                }
            }
        }
        let key = keys.next().unwrap();
        assert!(keys.next().is_none());
        match self.0[state].entry(key) {
            hash_map::Entry::Occupied(_) => Err(BindingError {
                kind: BindingErrorKind::ExtensionAlreadyBound,
                builder: self,
            }),
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Entry::Done(value));
                Ok(self)
            }
        }
    }

    pub fn build(self) -> InputStateMachine<K, V> {
        InputStateMachine {
            states: self.0,
            current_state: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut machine = Builder::<char, i32>::default().build();
        assert_eq!(machine.input('a'), InputResult::Invalid);
    }

    #[test]
    fn two_bindings() {
        let mut machine = Builder::<char, i32>::default()
            .binding(['a'], 1)
            .unwrap()
            .binding(['b'], 2)
            .unwrap()
            .build();
        assert_eq!(machine.input('a'), InputResult::Done(1));
        assert_eq!(machine.input('b'), InputResult::Done(2));
        assert_eq!(machine.input('c'), InputResult::Invalid);
    }

    #[test]
    fn multikey_bindings() {
        let mut machine = Builder::<char, i32>::default()
            .binding(['a', 'a'], 1)
            .unwrap()
            .binding(['a', 'b', 'c'], 2)
            .unwrap()
            .binding(['a', 'c'], 3)
            .unwrap()
            .binding(['z', 'z'], 4)
            .unwrap()
            .binding(['x', 'x', 'x'], 5)
            .unwrap()
            .binding(['y'], 6)
            .unwrap()
            .build();

        assert_eq!(machine.input('a'), InputResult::NeedMore);
        assert_eq!(machine.input('a'), InputResult::Done(1));

        assert_eq!(machine.input('a'), InputResult::NeedMore);
        assert_eq!(machine.input('z'), InputResult::Invalid);

        assert_eq!(machine.input('a'), InputResult::NeedMore);
        assert_eq!(machine.input('b'), InputResult::NeedMore);
        assert_eq!(machine.input('c'), InputResult::Done(2));

        assert_eq!(machine.input('a'), InputResult::NeedMore);
        assert_eq!(machine.input('c'), InputResult::Done(3));

        assert_eq!(machine.input('z'), InputResult::NeedMore);
        assert_eq!(machine.input('z'), InputResult::Done(4));

        assert_eq!(machine.input('x'), InputResult::NeedMore);
        assert_eq!(machine.input('x'), InputResult::NeedMore);
        assert_eq!(machine.input('x'), InputResult::Done(5));

        assert_eq!(machine.input('y'), InputResult::Done(6));

        assert_eq!(machine.input('w'), InputResult::Invalid);
    }

    #[test]
    fn empty_sequence() {
        assert_eq!(
            Builder::<char, i32>::default()
                .binding([], 1)
                .unwrap_err()
                .kind,
            BindingErrorKind::Empty
        );
    }

    #[test]
    fn prefix_already_bound() {
        assert_eq!(
            Builder::<char, i32>::default()
                .binding(['a'], 1)
                .unwrap()
                .binding(['a', 'b'], 2)
                .unwrap_err()
                .kind,
            BindingErrorKind::PrefixAlreadyBound
        );
    }

    #[test]
    fn extention_already_bound() {
        assert_eq!(
            Builder::<char, i32>::default()
                .binding(['a', 'b'], 1)
                .unwrap()
                .binding(['a'], 2)
                .unwrap_err()
                .kind,
            BindingErrorKind::ExtensionAlreadyBound
        );
    }
}
