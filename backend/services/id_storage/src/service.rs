use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use futures::lock::Mutex;
use futures::{Stream, StreamExt};

use common::ids::TaggedId;

use crate::internal_api::{IdExchanger, ViewerIdSet, ViewerRegistered};

struct IdMap {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl IdMap {
    fn from_set(set: ViewerIdSet) -> IdMap {
        let mut map = IdMap {
            map: HashMap::new(),
        };
        map.add(set.auth());
        map.add(set.billing());
        map.add(set.events());
        map
    }

    fn add<T: Any + Send + Sync + Copy>(&mut self, id: T) {
        self.map.insert(id.type_id(), Box::new(id));
    }

    fn get_by_type_id<T: Any + Send + Sync + Copy>(&self) -> Option<T> {
        self.map
            .get(&TypeId::of::<T>())?
            .downcast_ref::<T>()
            .copied()
    }
}

/// The id exchange's state. Crate-private: outside this crate it is reachable
/// only through `Wiring` and the `IdExchanger`s it hands out.
#[derive(Clone)]
pub(crate) struct IdStorage {
    db: Arc<Mutex<Vec<IdMap>>>,
}

impl IdStorage {
    pub(crate) fn new() -> Self {
        IdStorage {
            db: Arc::new(Mutex::new(vec![])),
        }
    }

    pub(crate) fn start_registration_listener<S>(
        &self,
        stream: S,
    ) -> impl Future<Output = ()> + use<S>
    where
        S: Stream<Item = ViewerRegistered>,
    {
        let storage = self.clone();
        stream.for_each(move |event| {
            let storage = storage.clone();
            async move { storage.add(event.ids).await }
        })
    }

    pub(crate) fn to_exchanger<Id1, Id2>(&self) -> IdExchanger<Id1, Id2> {
        IdExchanger::new(self.clone())
    }

    async fn add(&self, set: ViewerIdSet) {
        self.db.lock().await.push(IdMap::from_set(set));
    }

    pub(crate) async fn get_secondary<
        Id1: TaggedId + 'static,
        Id2: TaggedId + 'static,
    >(
        &self,
        from: Id1,
    ) -> Option<Id2> {
        let secondary = {
            let guard = self.db.lock().await;
            let map = guard
                .iter()
                .find(|i| i.get_by_type_id::<Id1>() == Some(from))?;
            let secondary = map.get_by_type_id::<Id2>()?;
            drop(guard);
            secondary
        };
        Some(secondary)
    }
}

#[cfg(test)]
mod tests {
    use common::{ViewerBillingId, ViewerEventsId, ViewerIdAuthentication};
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::internal_api::IdExchange;

    #[gtest]
    #[rstest]
    fn registration_listener_stores_the_ids_it_receives() {
        let storage = IdStorage::new();
        let ids = ViewerIdSet::new(ViewerIdAuthentication::new());

        futures::executor::block_on(storage.start_registration_listener(
            futures::stream::iter([ViewerRegistered { ids }]),
        ));

        let exchanger: IdExchanger<ViewerEventsId, ViewerBillingId> =
            storage.to_exchanger();
        let billing = futures::executor::block_on(exchanger.get_secondary(ids.events()));
        expect_that!(billing, some(eq(ids.billing())));
    }
}
