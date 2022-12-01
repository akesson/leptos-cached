use std::{fmt::Debug, future::Future, pin::Pin, rc::Rc};

use leptos::{
    create_effect, create_memo, create_signal, spawn_local, Memo, ReadSignal, Scope, WriteSignal,
};

/// A KeyedSignal associates a key with a value.
/// This is typically used for caching, where a key is needed
/// for searching in the cache.
///
/// Only when the key changes, when it's not equal to previous value,
/// the lookup function is called.
pub struct KeyedSignal<Key, Val>
where
    Key: 'static + PartialEq + Clone + Debug,
    Val: 'static,
{
    inner: KeyedSignalInner<Key, Val>,
    pub value: ReadSignal<Option<Val>>,
}

impl<Key, Val> KeyedSignal<Key, Val>
where
    Key: 'static + PartialEq + Clone + Debug,
    Val: 'static + Clone,
{
    pub fn get(&self) -> Option<Val> {
        self.value.get()
    }
    pub fn get_key(&self) -> Key {
        self.inner.key.get()
    }

    pub fn key(&self) -> Memo<Key> {
        self.inner.key
    }
}

pub fn create_keyed_signal<Key, Val, F, Fu>(
    cx: Scope,
    key: impl Fn() -> Key + 'static,
    lookup: F,
) -> KeyedSignal<Key, Val>
where
    Key: 'static + PartialEq + Clone + Debug,
    Val: 'static,
    F: Fn(Key, WriteSignal<Option<Val>>) -> Fu + 'static,
    Fu: Future<Output = ()> + 'static,
{
    let (value, inner) = create_key_signal_inner(cx, key, lookup);
    let act = inner.action_fn.clone();
    create_effect(cx, move |_| {
        let key = inner.key.get().clone();
        let fut = (act)(key, inner.set_value);
        spawn_local(async move { fut.await })
    });
    KeyedSignal { inner, value }
}

struct KeyedSignalInner<Key, Val>
where
    Key: 'static + PartialEq + Clone + Debug,
    Val: 'static,
{
    key: Memo<Key>,
    set_value: WriteSignal<Option<Val>>,

    action_fn: Rc<dyn Fn(Key, WriteSignal<Option<Val>>) -> Pin<Box<dyn Future<Output = ()>>>>,
}

fn create_key_signal_inner<Key, Val, F, Fu>(
    cx: Scope,
    key: impl Fn() -> Key + 'static,
    lookup: F,
) -> (ReadSignal<Option<Val>>, KeyedSignalInner<Key, Val>)
where
    Key: 'static + PartialEq + Clone + Debug,
    Val: 'static,
    F: Fn(Key, WriteSignal<Option<Val>>) -> Fu + 'static,
    Fu: Future<Output = ()> + 'static,
{
    let key = create_memo(cx, move |_| key());
    let (value, set_value) = create_signal(cx, None);

    let action_fn = Rc::new(move |input: Key, value: WriteSignal<Option<Val>>| {
        let input = input.clone();
        let fut = lookup(input, value);
        Box::pin(async move { fut.await }) as Pin<Box<dyn Future<Output = ()>>>
    });

    (
        value,
        KeyedSignalInner {
            key,
            set_value,
            action_fn,
        },
    )
}
