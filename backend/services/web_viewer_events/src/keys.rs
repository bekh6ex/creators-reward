use std::future::Future;

use futures::{Stream, StreamExt};

use common::viewer_registration::{ViewerEventsKeyRegistered, ViewerPublicKey};
use common::{StupidAsyncMemoryMap, ViewerEventsId};

#[derive(Clone)]
pub struct ViewerPublicKeys {
    map: StupidAsyncMemoryMap<ViewerEventsId, ViewerPublicKey>,
}

impl Default for ViewerPublicKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewerPublicKeys {
    pub fn new() -> Self {
        ViewerPublicKeys {
            map: StupidAsyncMemoryMap::new(),
        }
    }

    pub async fn get(&self, id: ViewerEventsId) -> Option<ViewerPublicKey> {
        self.map.get(&id).await
    }

    pub fn start_listener<S>(&self, stream: S) -> impl Future<Output = ()> + use<S>
    where
        S: Stream<Item = ViewerEventsKeyRegistered>,
    {
        let map = self.map.clone();
        stream.for_each(move |event| {
            let map = map.clone();
            async move {
                map.insert(event.events_id, event.public_key)
                    .await
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use futures::stream;
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[gtest]
    #[rstest]
    fn listener_stores_public_key_under_events_id() {
        let keys = ViewerPublicKeys::new();
        let events_id = ViewerEventsId::new();
        let public_key = ViewerPublicKey::try_from(&[1u8; 32][..]).unwrap();

        block_on(
            keys.start_listener(stream::iter([ViewerEventsKeyRegistered {
                events_id,
                public_key,
            }])),
        );

        expect_that!(block_on(keys.get(events_id)), some(eq(public_key)));
        expect_that!(block_on(keys.get(ViewerEventsId::new())), none());
    }
}
