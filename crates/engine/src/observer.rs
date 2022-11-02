use parking_lot::Mutex;
use std::{
    fmt::Debug,
    sync::{Arc, Weak},
};

type ObserverMap<T> = Mutex<slab::Slab<T>>;

pub struct Observers<T> {
    observers: Arc<ObserverMap<T>>,
}

impl<T> Debug for Observers<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Observers");

        d.finish()
    }
}

impl<T> Default for Observers<T> {
    fn default() -> Self {
        Self {
            observers: Default::default(),
        }
    }
}

impl<T> Observers<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn connect(&self, observer: T) -> ObserverHandle<T> {
        let id = self.observers.lock().insert(observer);

        ObserverHandle {
            observers: Arc::downgrade(&self.observers),
            id: Arc::new(Mutex::new(Some(id))),
        }
    }

    pub fn emit(&self, mut f: impl FnMut(&mut T)) {
        for (_, it) in self.observers.lock().iter_mut() {
            f(it);
        }
    }
}

#[derive(Debug)]
pub struct ObserverHandle<T> {
    id: Arc<Mutex<Option<usize>>>,
    observers: Weak<ObserverMap<T>>,
}

impl<T> Clone for ObserverHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            observers: self.observers.clone(),
        }
    }
}

impl<T> ObserverHandle<T> {
    pub fn disconnect(self) -> Option<T> {
        self.id
            .lock()
            .take()
            .and_then(|it| self.observers.disconnect(it))
    }

    pub fn is_disconnected(&self) -> bool {
        self.id.lock().is_none()
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
            .and_then(|it| it.lock().try_remove(id))
            .is_some()
    }

    fn is_disconnected(&self, id: usize) -> bool {
        self.upgrade()
            .and_then(|it| it.lock().get(id).map(|_| false))
            .unwrap_or(true)
    }
}

impl<T> TypedObserverMapTrait for Weak<ObserverMap<T>> {
    type Out = Option<T>;
    fn disconnect(&self, id: usize) -> Self::Out {
        self.upgrade().and_then(|it| it.lock().try_remove(id))
    }
}

pub struct AnyObserverHandleSend {
    map: Box<dyn ObserverMapTrait + Send>,
    id: Arc<Mutex<Option<usize>>>,
}

impl AnyObserverHandleSend {
    pub fn is_disconnected(&self) -> bool {
        self.id.lock().is_none()
    }

    pub fn disconnect(self) -> bool {
        self.id
            .lock()
            .take()
            .map(|it| self.map.disconnect_any(it))
            .unwrap_or(false)
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
        assert_eq!(format!("{:?}", observer), "Observers");
    }
}
