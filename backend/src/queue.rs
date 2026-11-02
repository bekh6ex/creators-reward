use futures::channel::mpsc::Sender;

pub(crate) type InMemSink<T> = Sender<T>;

pub(crate) fn create_memory_queue<T>() -> (
    // impl futures::Sink<T, Error = futures::channel::mpsc::SendError> + Clone + Unpin,
    InMemSink<T>,
    impl futures::Stream<Item = T>,
) {
    futures::channel::mpsc::channel(100)
}
