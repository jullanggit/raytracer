use std::{
    fmt::Debug,
    iter::Step,
    marker::PhantomData,
    ops::{Index, IndexMut},
    range::Range,
};

use crate::vec3::AsConvert;

#[derive(Debug)]
/// An indexer for something containing `Data`
#[repr(transparent)]
pub struct Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize>,
{
    inner: IndexerType,
    _phantom: PhantomData<Data>,
}
impl<IndexerType, Data> Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize>,
{
    pub const fn new(value: IndexerType) -> Self {
        Self {
            inner: value,
            _phantom: PhantomData,
        }
    }
    pub fn inner(self) -> IndexerType {
        self.inner
    }
    pub fn index<Collection>(self, collection: &Collection) -> &Data
    where
        Collection: Index<usize, Output = Data> + ?Sized,
    {
        collection.index(self.inner.as_convert())
    }
    pub fn index_mut<Collection>(self, collection: &mut Collection) -> &mut Data
    where
        Collection: IndexMut<usize, Output = Data> + ?Sized,
    {
        collection.index_mut(self.inner.as_convert())
    }
    pub fn index_range<Collection>(range: Range<Self>, collection: &Collection) -> &[Data]
    where
        Collection: Index<Range<usize>, Output = [Data]> + ?Sized,
    {
        let range = Range::from(range.start.inner.as_convert()..range.end.inner.as_convert());
        collection.index(range)
    }
    pub fn index_range_mut<Collection>(
        range: Range<Self>,
        collection: &mut Collection,
    ) -> &mut [Data]
    where
        Collection: IndexMut<Range<usize>, Output = [Data]> + ?Sized,
    {
        let range = Range::from(range.start.inner.as_convert()..range.end.inner.as_convert());
        collection.index_mut(range)
    }
}
impl<IndexerType, Data> Clone for Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize> + Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}
impl<IndexerType, Data> Copy for Indexer<IndexerType, Data> where
    IndexerType: AsConvert<usize> + Copy
{
}
// Required for Range::iter()
impl<IndexerType, Data> Step for Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize> + Step,
{
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        Step::steps_between(&start.inner, &end.inner)
    }
    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Step::forward_checked(start.inner, count).map(Self::new)
    }
    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Step::backward_checked(start.inner, count).map(Self::new)
    }
}
impl<IndexerType, Data> PartialEq for Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize> + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}
impl<IndexerType, Data> PartialOrd for Indexer<IndexerType, Data>
where
    IndexerType: AsConvert<usize> + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}
// impl<IndexerType, Data> Deref for Indexer<IndexerType, Data>
// where
//     IndexerType: Into<usize>,
// {
//     type Target = IndexerType;
//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }
