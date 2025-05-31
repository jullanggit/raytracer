use std::{
    marker::PhantomData,
    ops::{Deref, Index},
};

pub struct Collection<Container, Data>
where
    Container: Deref<Target: Index<usize, Output = Data>>,
{
    inner: Container,
    _phantom: PhantomData<Data>,
}
impl<Container, Data> Collection<Container, Data>
where
    Container: Deref<Target: Index<usize, Output = Data>>,
{
    pub fn as_ref<ContainerRef>(&self) -> Collection<&ContainerRef, Data>
    where
        Container: AsRef<ContainerRef>,
        ContainerRef: Index<usize, Output = Data> + ?Sized,
    {
        Collection {
            inner: self.inner.as_ref(),
            _phantom: PhantomData,
        }
    }
    pub fn as_mut<ContainerMut>(&mut self) -> Collection<&mut ContainerMut, Data>
    where
        Container: AsMut<ContainerMut>,
        ContainerMut: Index<usize, Output = Data> + ?Sized,
    {
        Collection {
            inner: self.inner.as_mut(),
            _phantom: PhantomData,
        }
    }
}
// impl<Container, Data> Deref for Collection<Container, Data>
// where
//     Container: Deref<Target: Index<usize, Output = Data>>,
// {
//     type Target = Container;
//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

pub struct Indexer<IndexerType, Data>
where
    IndexerType: Into<usize>,
{
    inner: IndexerType,
    _phantom: PhantomData<Data>,
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

impl<IndexerType, Container, Data> Index<Indexer<IndexerType, Data>> for Collection<Container, Data>
where
    IndexerType: Into<usize>,
    Container: Deref<Target: Index<usize, Output = Data>>,
{
    type Output = Data;
    fn index(&self, index: Indexer<IndexerType, Data>) -> &Self::Output {
        self.inner.index(index.inner.into())
    }
}

#[cfg(test)]
mod test {
    use super::{Collection, Indexer};
    use crate::{bvh::BvhNode, shapes::Shape};
    use std::marker::PhantomData;

    fn test_generic<T: Shape>() {
        let bvhnodes: Collection<Vec<BvhNode<T>>, BvhNode<T>> = Collection {
            inner: Default::default(),
            _phantom: PhantomData,
        };
        let indexer: Indexer<u16, BvhNode<T>> = Indexer {
            inner: 0_u16,
            _phantom: PhantomData,
        };
        let out = &bvhnodes[indexer];

        let converted: Collection<&[BvhNode<T>], BvhNode<T>> = bvhnodes.as_ref();
    }
}
