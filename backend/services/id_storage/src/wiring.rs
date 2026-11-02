use std::future::Future;

use futures::Stream;

use crate::internal_api::{IdExchanger, ViewerRegistered};
use crate::service::IdStorage;

/// The id exchange as seen by the composition root. Holds the service's
/// state privately and exposes only what wiring needs: typed exchangers to
/// hand to services, and the registration listener to run in the event loop.
/// Never hand a `Wiring` to a service or put it in `AppState`.
pub struct Wiring {
    storage: IdStorage,
}

pub fn create_wiring() -> Wiring {
    Wiring {
        storage: IdStorage::new(),
    }
}

impl Wiring {
    /// A handle that translates `Id1` into `Id2` and nothing else. Give each
    /// service one per translation it needs.
    pub fn exchanger<Id1, Id2>(&self) -> IdExchanger<Id1, Id2> {
        self.storage.to_exchanger()
    }

    /// Consumes `ViewerRegistered` events; the only way ids get in. Push the
    /// returned future into the global listener loop.
    pub fn registration_listener<S>(&self, stream: S) -> impl Future<Output = ()> + use<S>
    where
        S: Stream<Item = ViewerRegistered>,
    {
        self.storage.start_registration_listener(stream)
    }
}
