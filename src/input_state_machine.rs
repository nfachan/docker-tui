use std::{collections::HashMap, hash::Hash};

enum InputStateMachineEntry<V> {
    Done(V),
    NeedMore(usize),
}

#[derive(Debug, Eq, PartialEq)]
pub enum InputStateMachineResult<V> {
    Done(V),
    NeedMore,
    Invalid,
}

pub struct InputStateMachine<K, V> {
    states: Vec<HashMap<K, InputStateMachineEntry<V>>>,
    current_state: usize,
}

impl<K, V> InputStateMachine<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn input(&mut self, key: K) -> InputStateMachineResult<V> {
        match self.states[self.current_state].get(&key) {
            None => {
                self.current_state = 0;
                InputStateMachineResult::Invalid
            }
            Some(InputStateMachineEntry::Done(value)) => {
                self.current_state = 0;
                InputStateMachineResult::Done(value.clone())
            }
            Some(InputStateMachineEntry::NeedMore(next_state)) => {
                self.current_state = *next_state;
                InputStateMachineResult::NeedMore
            }
        }
    }
}

pub struct InputStateMachineBuilder<K, V>(Vec<HashMap<K, InputStateMachineEntry<V>>>);

impl<K, V> Default for InputStateMachineBuilder<K, V> {
    fn default() -> Self {
        InputStateMachineBuilder(vec![HashMap::default()])
    }
}

impl<K, V> InputStateMachineBuilder<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn binding(mut self, keys: impl IntoIterator<Item = K>, value: V) -> Self {
        let keys = Vec::from_iter(keys);
        let num_keys = keys.len();
        assert!(num_keys > 0);
        let mut keys = keys.into_iter();
        let mut state = 0;
        for _ in 0..num_keys - 1 {
            let key = keys.next().unwrap();
            match self.0[state].get(&key) {
                None => {
                    let next_state = self.0.len();
                    self.0[state].insert(key, InputStateMachineEntry::NeedMore(next_state));
                    self.0.push(HashMap::default());
                    state = next_state;
                }
                Some(InputStateMachineEntry::NeedMore(next_state)) => {
                    state = *next_state;
                }
                Some(InputStateMachineEntry::Done(_)) => {
                    panic!("key bindings are not prefix-free");
                }
            }
        }
        let key = keys.next().unwrap();
        assert!(keys.next().is_none());
        let old = self.0[state].insert(key, InputStateMachineEntry::Done(value));
        assert!(old.is_none());
        self
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
        let mut machine = InputStateMachineBuilder::<char, i32>::default().build();
        assert_eq!(machine.input('a'), InputStateMachineResult::Invalid);
    }

    #[test]
    fn two_bindings() {
        let mut machine = InputStateMachineBuilder::<char, i32>::default()
            .binding(['a'], 1)
            .binding(['b'], 2)
            .build();
        assert_eq!(machine.input('a'), InputStateMachineResult::Done(1));
        assert_eq!(machine.input('b'), InputStateMachineResult::Done(2));
        assert_eq!(machine.input('c'), InputStateMachineResult::Invalid);
    }

    #[test]
    fn multikey_bindings() {
        let mut machine = InputStateMachineBuilder::<char, i32>::default()
            .binding(['a', 'a'], 1)
            .binding(['a', 'b', 'c'], 2)
            .binding(['a', 'c'], 3)
            .binding(['z', 'z'], 4)
            .binding(['x', 'x', 'x'], 5)
            .binding(['y'], 6)
            .build();

        assert_eq!(machine.input('a'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('a'), InputStateMachineResult::Done(1));

        assert_eq!(machine.input('a'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('z'), InputStateMachineResult::Invalid);

        assert_eq!(machine.input('a'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('b'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('c'), InputStateMachineResult::Done(2));

        assert_eq!(machine.input('a'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('c'), InputStateMachineResult::Done(3));

        assert_eq!(machine.input('z'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('z'), InputStateMachineResult::Done(4));

        assert_eq!(machine.input('x'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('x'), InputStateMachineResult::NeedMore);
        assert_eq!(machine.input('x'), InputStateMachineResult::Done(5));

        assert_eq!(machine.input('y'), InputStateMachineResult::Done(6));

        assert_eq!(machine.input('w'), InputStateMachineResult::Invalid);
    }
}
