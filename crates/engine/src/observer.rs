use parking_lot::Mutex;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    sync::{atomic::AtomicUsize, Arc, Weak},
};

type ObserverMap<T> = Mutex<HashMap<usize, T>>;

pub struct Observers<T> {
    observers: Arc<ObserverMap<T>>,
    last_insert_id: AtomicUsize,
}

impl<T> Debug for Observers<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observers")
            .field("last_insert_id", &self.last_insert_id)
            .finish_non_exhaustive()
    }
}

impl<T> Default for Observers<T> {
    fn default() -> Self {
        Self {
            observers: Default::default(),
            last_insert_id: Default::default(),
        }
    }
}

impl<T> Observers<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn connect(&self, observer: T) -> ObserverHandle<T> {
        let id = self.last_insert_id.fetch_add(1, atomic::Ordering::Relaxed);

        let handle = ObserverHandle {
            observers: Arc::downgrade(&self.observers),
            id,
        };
        self.observers.lock().insert(id, observer);

        handle
    }

    pub fn emit(&self, mut f: impl FnMut(&mut T)) {
        for (_, it) in self.observers.lock().iter_mut() {
            f(it);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObserverHandle<T> {
    id: usize,
    observers: Weak<ObserverMap<T>>,
}

impl<T> ObserverHandle<T> {
    pub fn disconnect(self) -> Option<T> {
        self.observers.disconnect(self.id)
    }

    pub fn is_disconnected(&self) -> bool {
        self.observers.is_disconnected(self.id)
    }
}

impl<T: Send + 'static> ObserverHandle<T> {
    pub fn send_handle_any(self) -> AnyObserverHandleSend {
        AnyObserverHandleSend {
            map: Box::new(self.observers),
            id: self.id,
        }
    }
}

pub trait ObserverMapTrait {
    fn disconnect_any(&self, id: usize) -> bool;
    fn is_disconnected(&self, id: usize) -> bool;
}

pub trait TypedObserverMapTrait: ObserverMapTrait {
    type Out;
    fn disconnect(&self, id: usize) -> Self::Out;
}

impl<T> ObserverMapTrait for Weak<ObserverMap<T>> {
    fn disconnect_any(&self, id: usize) -> bool {
        self.upgrade()
            .and_then(|it| it.lock().remove(&id))
            .is_some()
    }

    fn is_disconnected(&self, id: usize) -> bool {
        self.upgrade()
            .and_then(|it| it.lock().get(&id).map(|_| false))
            .unwrap_or(true)
    }
}

impl<T> TypedObserverMapTrait for Weak<ObserverMap<T>> {
    type Out = Option<T>;
    fn disconnect(&self, id: usize) -> Self::Out {
        self.upgrade().and_then(|it| it.lock().remove(&id))
    }
}

pub struct AnyObserverHandleSend {
    map: Box<dyn ObserverMapTrait + Send>,
    id: usize,
}

impl AnyObserverHandleSend {
    pub fn is_disconnected(&self) -> bool {
        self.map.is_disconnected(self.id)
    }

    pub fn disconnect(self) -> bool {
        self.map.disconnect_any(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer() {
        let observer = Observers::new();

        let handle = observer.connect(1);

        assert!(!handle.is_disconnected());

        assert_eq!(handle.disconnect(), Some(1));

        let h2 = observer.connect(2);
        let h_2 = observer.connect(2);

        assert_eq!(h2.clone().disconnect(), Some(2));
        assert_eq!(h_2.disconnect(), Some(2));

        assert!(h2.is_disconnected());
        assert_eq!(h2.disconnect(), None);

        observer.connect(1);
        observer.connect(2);
        observer.connect(3);

        let mut i = 0;
        observer.emit(|_| {
            i += 1;
        });
        assert_eq!(i, 3);
    }

    #[test]
    fn observer_any() {
        let observer = Observers::new();

        let handle = observer.connect(1);
        let _ = handle.clone().send_handle_any().disconnect();
        assert!(handle.is_disconnected());
        assert!(handle.clone().send_handle_any().is_disconnected());
        assert!(!handle.clone().send_handle_any().disconnect());
        assert_eq!(handle.disconnect(), None);
    }

    #[test]
    fn observer_debug() {
        let observer = Observers::<()>::new();
        assert_eq!(
            format!("{:?}", observer),
            "Observers { last_insert_id: 0, .. }"
        );
    }
}
