use okazis::{Precondition, AppendError, PersistedEvent, EventStore, Since};
use event_stream::MemoryEventStream;
use std::sync::RwLock;
use std::hash::{Hash, BuildHasher};
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use super::Never;

pub struct MemoryEventStore<AggregateId, Event, Hasher = RandomState>
    where
        AggregateId: Hash + Eq,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<AggregateId, MemoryEventStream<Event>, Hasher>>,
}

impl<AggregateId, Event, Hasher> MemoryEventStore<AggregateId, Event, Hasher>
    where
        AggregateId: Hash + Eq + Clone,
        Hasher: BuildHasher,
{
    fn try_get_stream(&self, agg_id: &AggregateId) -> Option<MemoryEventStream<Event>> {
        self.data.read().unwrap()
            .get(agg_id)
            .map(|es| es.clone())
    }

    fn create_stream(&self, agg_id: &AggregateId) -> MemoryEventStream<Event> {
        let mut lock = self.data.write().unwrap();
        match lock.get(&agg_id) {
            Some(es) => return es.clone(),
            None => {}
        }

        let new_stream = MemoryEventStream::default();
        lock.insert(agg_id.clone(), new_stream.clone());

        new_stream
    }
}

impl<AggregateId, Event, Hasher> Default for MemoryEventStore<AggregateId, Event, Hasher>
    where
        AggregateId: Hash + Eq,
        Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        MemoryEventStore {
            data: RwLock::new(HashMap::<_, _, Hasher>::default()),
        }
    }
}

impl<AggregateId, Event, Hasher> EventStore for MemoryEventStore<AggregateId, Event, Hasher>
    where
        AggregateId: Hash + Eq + Clone,
        Event: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggregateId;
    type Event = Event;
    type Offset = usize;
    type AppendResult = Result<Option<Self::Offset>, AppendError<Self::Offset, Never>>;
    type ReadResult = Result<Option<Vec<PersistedEvent<Self::Offset, Self::Event>>>, Never>;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition<Self::Offset>) -> Self::AppendResult {
        if let Some(stream) = self.try_get_stream(&agg_id) {
            stream.append_events(events, condition)
        } else {
            if condition == Precondition::Always || condition == Precondition::NewStream || condition == Precondition::EmptyStream {
                let stream = self.create_stream(&agg_id);
                stream.append_events(events, Precondition::Always)
            } else {
                Err(AppendError::PreconditionFailed(condition))
            }
        }
    }

    fn read(&self, agg_id: &Self::AggregateId, since: Since<Self::Offset>) -> Self::ReadResult {
        match self.try_get_stream(&agg_id) {
            Some(es) => Ok(Some(es.read(since))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
#[path = "./event_store_tests.rs"]
mod tests;
